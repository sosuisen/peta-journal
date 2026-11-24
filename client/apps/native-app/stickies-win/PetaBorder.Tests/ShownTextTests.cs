using System.Collections.ObjectModel;
using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>
/// The text of an item as it is shown, with its decoration, drawn from the parts that the core
/// makes of the body and gives with the row (ADR 042). The body of the first test goes through a
/// real journal server, so that test is skipped without <c>PETA_E2E_SERVER</c>.
/// </summary>
public sealed class ShownTextTests
{
    /// <summary>
    /// A body with every kind of node that the AST has, and for each kind what of it has to be
    /// seen. <c>document</c>, <c>item</c>, <c>table_row</c> and <c>table_cell</c> are seen by
    /// what is in them.
    /// </summary>
    private static readonly (string Kind, string Markdown, string Seen)[] EveryKind =
    [
        ("heading", "## HEADING", "HEADING"),
        ("paragraph", "PARAGRAPH", "PARAGRAPH"),
        ("text", "TEXT", "TEXT"),
        ("softbreak", "SOFT\nBREAK", "BREAK"),
        ("linebreak", "LINE\\\nBROKEN", "BROKEN"),
        ("emph", "*EMPH*", "EMPH"),
        ("strong", "**STRONG**", "STRONG"),
        ("strikethrough", "~~STRUCK~~", "STRUCK"),
        ("underline", "__UNDERLINED__", "UNDERLINED"),
        ("highlight", "==HIGHLIGHTED==", "HIGHLIGHTED"),
        ("superscript", "x^SUPER^", "SUPER"),
        ("subscript", "x~SUB~", "SUB"),
        ("spoilered_text", "||SPOILER||", "SPOILER"),
        ("code", "`CODE`", "CODE"),
        ("link", "[LINK](https://example.com/)", "LINK"),
        ("embed", "![ALT](images/a.png)", "ALT"),
        ("wikilink", "[[WIKI]]", "WIKI"),
        ("hashtag", "#hashtag", "#hashtag"),
        ("shortcode", ":smile:", "😄"),
        ("math", "$`MATH`$", "MATH"),
        ("html_inline", "<kbd>KEY</kbd>", "KEY"),
        ("footnote_reference", "NOTED[^note]", "[note]"),
        ("footnote_definition", "[^note]: FOOTNOTE", "FOOTNOTE"),
        ("code_block", "```rust\nCODEBLOCK\n```", "CODEBLOCK"),
        ("block_quote", "> QUOTED", "QUOTED"),
        ("multiline_block_quote", ">>>\nMULTIQUOTED\n>>>", "MULTIQUOTED"),
        ("alert", "> [!TIP]\n> ALERTED", "Tip"),
        ("list", "1. LISTED", "LISTED"),
        ("task_item", "- [ ] OPEN\n- [x] CLOSED", "☐ OPEN"),
        ("table", "| HEAD |\n|---|\n| CELL |", "CELL"),
        ("thematic_break", "---", "────"),
        ("subtext", "-# SUBTEXT", "SUBTEXT"),
        ("html_block", "<details>\nHTMLBLOCK\n</details>", "HTMLBLOCK"),
    ];

    /// <summary>The row of an item with this body, as the journal server gives it back.</summary>
    private static Row RowWithBody(string body, JournalServer server)
    {
        var main = new MainThread();
        using var shell = new Shell(
            server.Address,
            "/pages/shown",
            main.Post,
            reply => reply(null),
            (_, _) => { },
            _ => { }
        );
        main.WaitUntil("the page is shown", () => shell.View.Outline.Rows.Count > 0);
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        shell.Typed(body, first);
        shell.FinishEditing(first);
        main.WaitUntil("the edit is over", () =>
            !shell.View.Outline.Rows[0].IsEditing && shell.View.Outline.Rows[0].Item.Body == body
        );
        return shell.View.Outline.Rows[0];
    }

    // ASTのどの種類の中身も、表示するテキストに入っている。
    [Fact]
    public void What_is_in_every_kind_of_node_is_in_the_shown_text()
    {
        using var server = new JournalServer(18112);
        var row = RowWithBody(string.Join("\n\n", EveryKind.Select(kind => kind.Markdown)), server);

        var shown = ShownText.Of(row.BodyModel).Text;

        foreach (var (kind, _, seen) in EveryKind)
            Assert.True(
                shown.Contains(seen, StringComparison.Ordinal),
                $"the kind {kind} is not shown: {seen} is missing in\n{shown}"
            );
        Assert.Contains("☑ CLOSED", shown, StringComparison.Ordinal);
    }

    private static Looks Plain() => new();

    private static RichRun Run(string text, Looks? looks = null, RichLink? link = null) =>
        new()
        {
            Text = text,
            Looks = looks ?? Plain(),
            Link = link,
        };

    private static RichParagraph Paragraph(params RichRun[] runs) => new() { Runs = new(runs) };

    private static RichBlock.Text Text(params RichParagraph[] paragraphs) => new(new(paragraphs));

    private static ShownText Shown(params RichBlock[] blocks) =>
        ShownText.Of(new RichText { Blocks = new(blocks) });

    private static RichEntry Entry(string mark, params RichBlock[] blocks) =>
        new() { Mark = mark, Blocks = new(blocks) };

    private static RichBlock.Entries Entries(params RichEntry[] entries) => new(new(entries), 1.5f);

    // 段落に触れた位置は、コアが数える表示の文字の数になる。リストの印は数えず、絵文字は1文字と数える（ADR 043）。
    [Fact]
    public void A_touch_in_a_paragraph_counts_the_letters_as_the_core_does()
    {
        var paragraph = Paragraph(Run("a😀"), Run("bc"));
        paragraph.Start = 5;

        var shown = Assert.IsType<ShownParagraph>(
            Assert.Single(Shown(Entries(Entry("•", Text(paragraph)))).Parts)
        );

        // The shown paragraph is "• a😀bc": the mark and its space are not letters of the core
        Assert.Equal(5, shown.LetterAt(0));
        Assert.Equal(5, shown.LetterAt(2));
        Assert.Equal(6, shown.LetterAt(3));
        Assert.Equal(7, shown.LetterAt(5));
        Assert.Equal(8, shown.LetterAt(6));
        Assert.Equal(9, shown.LetterAt(7));
    }

    // コードブロックと表の升に触れた位置も、コアが数える表示の文字の数になる。
    [Fact]
    public void A_touch_in_a_code_block_or_a_cell_counts_from_its_start()
    {
        var cell = Paragraph(Run("xy"));
        cell.Start = 20;
        var shown = Shown(
            new RichBlock.Code("let a\nlet b", 9),
            new RichBlock.Table(new([new RichTableRow { Cells = new([cell]) }]))
        );

        var code = Assert.IsType<ShownParagraph>(shown.Parts[0]);
        var table = Assert.IsType<ShownTable>(shown.Parts[1]);
        Assert.Equal(16, code.LetterAt(7));
        Assert.Equal(21, table.Rows[0].Cells[0].LetterAt(1));
    }

    // 区切り線に触れた位置は、その段落の先頭になる。線の文字はコアの文字ではない。
    [Fact]
    public void A_touch_on_a_rule_is_at_the_start_of_its_paragraph()
    {
        var shown = Assert.IsType<ShownParagraph>(
            Assert.Single(
                Shown(
                    Text(
                        new RichParagraph
                        {
                            Rule = true,
                            Runs = [],
                            Start = 3,
                        }
                    )
                ).Parts
            )
        );

        Assert.Equal(3, shown.LetterAt(4));
    }

    // 段落の印は行の頭に付き、区切り線は線の文字で出る。
    [Fact]
    public void The_mark_of_a_paragraph_leads_its_line_and_a_rule_is_a_line_of_letters()
    {
        var marked = Paragraph(Run("OPEN"));
        marked.Mark = "☐";

        var shown = Shown(Text(marked, new RichParagraph { Rule = true, Runs = [] }));

        Assert.Equal("☐ OPEN\n────────", shown.Text);
    }

    // リンクは、コアが言う行き先のまま、表示するテキストに残る。
    [Fact]
    public void A_link_keeps_where_the_core_says_it_leads()
    {
        var page = new RichLink.Page("/pages/notes/Idea");
        var web = new RichLink.Web("https://example.com/a");
        var file = new RichLink.File("assets/report.pdf");

        var runs = Assert
            .IsType<ShownParagraph>(
                Assert.Single(
                    Shown(
                        Text(
                            Paragraph(
                                Run("Idea", link: page),
                                Run(" "),
                                Run("out", link: web),
                                Run("report", link: file)
                            )
                        )
                    ).Parts
                )
            )
            .Runs;

        Assert.Equal(new RichLink?[] { page, null, web, file }, runs.Select(run => run.Link));
    }

    // 文字の見た目は、表示するテキストの各部分の装いになる。
    [Fact]
    public void The_looks_of_the_letters_become_their_style()
    {
        (Action<Looks> Set, RunStyle Style)[] each =
        [
            (looks => looks.Bold = true, RunStyle.Bold),
            (looks => looks.Italic = true, RunStyle.Italic),
            (looks => looks.Struck = true, RunStyle.Strike),
            (looks => looks.Underlined = true, RunStyle.Underline),
            (looks => looks.Marked = true, RunStyle.Highlight),
            (looks => looks.Code = true, RunStyle.Code),
            (looks => looks.Raised = true, RunStyle.Superscript),
            (looks => looks.Lowered = true, RunStyle.Subscript),
            (looks => looks.Hidden = true, RunStyle.Spoiler),
        ];
        var runs = each.Select(
                (one, at) =>
                {
                    var looks = Plain();
                    one.Set(looks);
                    return Run($"{at}", looks);
                }
            )
            .Append(Run("plain"))
            .ToArray();

        var shown = Assert.IsType<ShownParagraph>(
            Assert.Single(Shown(Text(Paragraph(runs))).Parts)
        );

        Assert.Equal(
            each.Select(one => one.Style).Append(RunStyle.None),
            shown.Runs.Select(run => run.Style)
        );
    }

    // 上付きと下付きの文字は、段の文字の0.75倍の大きさになる。ほかの版と同じである。
    [Fact]
    public void A_raised_or_a_lowered_letter_is_smaller()
    {
        var raised = Plain();
        raised.Raised = true;
        var lowered = Plain();
        lowered.Lowered = true;

        var shown = Assert.IsType<ShownParagraph>(
            Assert.Single(
                Shown(Text(Paragraph(Run("up", raised), Run("down", lowered), Run("plain")))).Parts
            )
        );

        Assert.Equal([0.75, 0.75, 1], shown.Runs.Select(run => run.Scale));
    }

    // 見出しは、その段の大きさで、太字で出る。大きさは、Web版と同じである。
    [Fact]
    public void A_heading_is_large_and_bold()
    {
        var one = Paragraph(Run("one"));
        one.Heading = 1;
        var three = Paragraph(Run("three"));
        three.Heading = 3;

        var shown = Shown(Text(one, three, Paragraph(Run("text"))))
            .Parts.Cast<ShownParagraph>()
            .ToList();

        Assert.Equal([1.6, 1.2, 1], shown.Select(paragraph => paragraph.Scale));
        Assert.Equal(RunStyle.Bold, shown[0].Runs[0].Style);
        Assert.Equal(RunStyle.None, shown[2].Runs[0].Style);
    }

    // リストの項目は入れ子の分だけ右に寄る。引用の中は右に寄り、薄い文字になる。
    [Fact]
    public void The_entries_of_a_list_and_a_quote_stand_to_the_right()
    {
        var top = Paragraph(Run("a"));
        top.Depth = 1;
        top.Mark = "•";
        var inner = Paragraph(Run("b"));
        inner.Depth = 2;
        inner.Mark = "3.";
        var quoted = Paragraph(Run("q"));
        quoted.Quoted = 1;

        var shown = Shown(Text(top, inner, quoted)).Parts.Cast<ShownParagraph>().ToList();

        Assert.Equal(["• a", "3. b", "q"], shown.Select(paragraph => paragraph.Text));
        Assert.Equal([0, 1, 1], shown.Select(paragraph => paragraph.Indent));
        Assert.Equal(RunStyle.Quote, shown[2].Runs[0].Style);
    }

    // コアが分けたリストでは、項目の印が最初の部品の頭に付く。中身のない項目も印が出る。
    [Fact]
    public void An_entry_of_a_list_that_the_core_took_apart_shows_its_mark()
    {
        var shown = Shown(
            Entries(
                Entry("1.", new RichBlock.Code("let x = 1", 0)),
                Entry("2."),
                Entry("3.", new RichBlock.Image(new RichImage { Path = "assets/a.png", Alt = "" }))
            )
        );

        Assert.Equal("1. let x = 1\n2. \n3. \n[assets/a.png]", shown.Text);
        Assert.Equal(RunStyle.Code, Assert.IsType<ShownParagraph>(shown.Parts[0]).Runs[1].Style);
    }

    // コアが分けた入れ子のリストでは、内側の項目が右に寄る。中身のない項目の印も同じである。
    [Fact]
    public void The_entries_of_a_list_inside_a_list_that_the_core_took_apart_stand_to_the_right()
    {
        var shown = Shown(
            Entries(
                Entry(
                    "1.",
                    new RichBlock.Code("x", 0),
                    Entries(Entry("a.", new RichBlock.Code("let x = 1", 0)), Entry("b."))
                )
            )
        );

        Assert.Equal("1. x\na. let x = 1\nb. ", shown.Text);
        Assert.Equal([0, 1, 1], shown.Parts.Cast<ShownParagraph>().Select(part => part.Indent));
    }

    // コアが分けた引用の中の部品は、右に寄る。
    [Fact]
    public void A_part_in_a_quote_that_the_core_took_apart_stands_to_the_right()
    {
        var shown = Shown(new RichBlock.Quote([new RichBlock.Code("code", 0)], 1.5f));

        var code = Assert.IsType<ShownParagraph>(Assert.Single(shown.Parts));
        Assert.Equal(1, code.Indent);
        Assert.Equal(RunStyle.Code | RunStyle.Quote, code.Runs[0].Style);
    }

    // 表は表の部品になり、見出しの行は太字になる。
    [Fact]
    public void A_table_is_a_part_of_its_own_with_its_header_in_bold()
    {
        static RichTableRow Row(bool header, params string[] cells) =>
            new() { Header = header, Cells = new(cells.Select(cell => Paragraph(Run(cell)))) };

        var table = Assert.IsType<ShownTable>(
            Assert.Single(
                Shown(new RichBlock.Table([Row(true, "H1", "H2"), Row(false, "a", "b")])).Parts
            )
        );

        Assert.Equal(
            new[] { new[] { "H1", "H2" }, new[] { "a", "b" } },
            table.Rows.Select(row => row.Cells.Select(cell => cell.Text).ToArray()).ToArray()
        );
        Assert.Equal(RunStyle.Bold, table.Rows[0].Cells[0].Runs[0].Style);
        Assert.Equal(RunStyle.None, table.Rows[1].Cells[0].Runs[0].Style);
    }

    // 画像は、ページが言う大きさを持つ部品になる。ファイルが来るまでは、代替の文字が出る。
    [Fact]
    public void An_image_is_a_part_with_the_size_that_the_page_asks_for()
    {
        var picture = Assert.IsType<ShownPicture>(
            Assert.Single(
                Shown(
                    new RichBlock.Image(
                        new RichImage
                        {
                            Path = "assets/a.png",
                            Alt = "ALT",
                            Width = 120,
                        }
                    )
                ).Parts
            )
        );

        Assert.Equal("assets/a.png", picture.Path);
        Assert.Equal(120u, picture.Width);
        Assert.Null(picture.Height);
        Assert.Equal("[ALT]", picture.Words);
    }

    // 本文のない項目は、何も出ない。
    [Fact]
    public void An_empty_item_shows_nothing()
    {
        Assert.Empty(Shown().Parts);
    }
}
