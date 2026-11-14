using System.Diagnostics;
using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>
/// The bytes between C# and Rust. No journal server is needed: nothing answers at the address.
///
/// Why these tests matter: the C# types are generated from the shape of the Rust types, not
/// from what serde writes. A Rust type with a hand-written <c>Serialize</c> can write other
/// bytes than its shape says. Then everything compiles, and the shell cannot read the bytes.
/// </summary>
public sealed class BorderTests
{
    private const string Nowhere = "http://127.0.0.1:9";

    /// <summary>What the core asked of the shell. The core may ask from any thread.</summary>
    private sealed class Asked
    {
        private readonly List<Request> requests = [];

        public void Add(IReadOnlyList<Request> added)
        {
            lock (requests)
                requests.AddRange(added);
        }

        /// <summary>
        /// The first request of a kind, once it is there. A request does not always come
        /// before the call that caused it returns: the core starts by asking the server from
        /// threads of its own, and when one of them is at work in the core, it is that thread
        /// that hands the request over.
        /// </summary>
        public Request? FirstOnceThere(Func<Request, bool> isOfTheKind)
        {
            var waited = Stopwatch.StartNew();
            while (true)
            {
                lock (requests)
                {
                    if (requests.FirstOrDefault(isOfTheKind) is { } found)
                        return found;
                }
                if (waited.Elapsed > TimeSpan.FromSeconds(5))
                    return null;
                Thread.Sleep(5);
            }
        }

        /// <summary>The navigations, once there is one.</summary>
        public List<string> NavigationsOnceAny()
        {
            FirstOnceThere(request => request.Effect is ShellEffect.Navigate);
            lock (requests)
            {
                return
                [
                    .. requests
                        .Select(request => request.Effect)
                        .OfType<ShellEffect.Navigate>()
                        .Select(navigate => navigate.Value.Url),
                ];
            }
        }
    }

    // Rustが書いたビューモデルを、C#が読める。C#が送ったイベントの中身が、そこに入っている。
    [Fact]
    public void The_view_model_of_Rust_is_read_in_CSharp()
    {
        using var border = new Border(Nowhere, _ => { });

        border.Send(new Event.SearchQueryChanged("apple 日本語"));

        Assert.Equal("apple 日本語", border.View().Search.Query);
    }

    // Rustが書いたシェルへの依頼を、C#が読める。
    [Fact]
    public void A_request_of_Rust_is_read_in_CSharp()
    {
        var asked = new Asked();
        using var border = new Border(Nowhere, asked.Add);

        border.Send(new Event.NavigateHome());

        Assert.Equal(["/today"], asked.NavigationsOnceAny());
    }

    // C#が書いた答えを、Rustが読める。新しいページの名前を答えると、そのページへ移る依頼が来る。
    [Fact]
    public void An_answer_of_CSharp_is_read_in_Rust()
    {
        var asked = new Asked();
        using var border = new Border(Nowhere, asked.Add);
        border.Send(new Event.CreateNewPage(""));
        var prompt = asked.FirstOnceThere(request =>
            request.Effect is ShellEffect.Ui { Value: UiOp.Prompt }
        );
        Assert.NotNull(prompt);

        border.Answer(prompt.Id, new UiOutput.Text("Notes"));

        Assert.Equal(["/pages/Notes"], asked.NavigationsOnceAny());
    }

    // 項目の行き先を持つイベントを、Rustが読める。行き先とプロパティの値は、Rustでは手で書いたシリアライズを持つ型である。
    [Fact]
    public void An_event_with_the_place_of_an_item_is_read_in_Rust()
    {
        using var border = new Border(Nowhere, _ => { });
        var item = new Item
        {
            Id = "a",
            PageId = "p",
            DirPath = "",
            Name = "notes",
            ItemType = "doc",
            Idx = "a",
            Level = 1,
            Body = "本文",
            Types = [],
            Properties = new()
            {
                ["tags"] = new PropertyValue.List(["x"]),
                ["title"] = new PropertyValue.Text("t"),
            },
        };

        foreach (
            var place in (ItemPosition[])
                [new ItemPosition.First(), new ItemPosition.Last(), new ItemPosition.After("x")]
        )
            border.Send(new Event.Moved([item], place, null, MoveLabel.Move));
    }

    // フィールドのないenumの`Option`を、C#が読める。タスクでない項目と、タスクである項目の両方である。
    [Fact]
    public void An_optional_plain_enum_of_Rust_is_read_in_CSharp()
    {
        var row = new Row
        {
            Item = new Item
            {
                Id = "a",
                PageId = "p",
                DirPath = "",
                Name = "notes",
                ItemType = "doc",
                Idx = "a",
                Body = "",
                Types = [],
                Properties = [],
            },
            BodyModel = new RichText { Blocks = [] },
            Task = TaskState.Running,
        };

        Assert.Equal(TaskState.Running, Row.BincodeDeserialize(row.BincodeSerialize()).Task);
        row.Task = null;
        Assert.Null(Row.BincodeDeserialize(row.BincodeSerialize()).Task);
    }

    // URLでページを開ける。何も指さないURLは、エラーになる。
    [Fact]
    public void A_page_is_opened_by_its_URL()
    {
        using var border = new Border(Nowhere, _ => { });

        border.Open("/pages/notes");
        Assert.Throws<PetaCore.BoltException>(() => border.Open(""));
    }

    // 待っていない依頼への答えは、エラーになる。
    [Fact]
    public void An_answer_nobody_waits_for_is_an_error()
    {
        using var border = new Border(Nowhere, _ => { });

        Assert.Throws<PetaCore.BoltException>(() => border.Answer(9_999, new UiOutput.Done()));
    }
}
