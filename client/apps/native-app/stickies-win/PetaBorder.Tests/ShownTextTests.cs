using System.Collections.ObjectModel;
using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>
/// The text of an item as it is shown, with its decoration, drawn from the AST that the core
/// gives with the row (ADR 041). The body of the first test goes through a real journal
/// server, so that test is skipped without <c>PETA_E2E_SERVER</c>.
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
        main.WaitUntil(
            "the edit is over",
            () =>
                !shell.View.Outline.Rows[0].IsEditing
                && shell.View.Outline.Rows[0].Item.Body == body
        );
        return shell.View.Outline.Rows[0];
    }

    /// <summary>The names of the types of the nodes in the tree, such as <c>CodeBlock</c>.</summary>
    private static HashSet<string> KindsOf(Node node) =>
        [node.GetType().Name, .. PetaBorder.Nodes.Children(node).SelectMany(KindsOf)];

    /// <summary>The name of the type of <c>Node</c> for a kind of the AST, such as
    /// <c>CodeBlock</c> for <c>code_block</c>.</summary>
    private static string TypeName(string kind) =>
        kind == "subscript"
            ? "SubscriptText"
            : string.Concat(
                kind.Split('_').Select(word => char.ToUpperInvariant(word[0]) + word[1..])
            );

    // ASTのどの種類の中身も、表示するテキストに入っている。
    [Fact]
    public void What_is_in_every_kind_of_node_is_in_the_shown_text()
    {
        using var server = new JournalServer(18112);
        var row = RowWithBody(string.Join("\n\n", EveryKind.Select(kind => kind.Markdown)), server);

        var shown = ShownText.Of(row.BodyModel.Ast).Text;

        var kinds = KindsOf(row.BodyModel.Ast);
        foreach (var (kind, _, seen) in EveryKind)
        {
            Assert.True(kinds.Contains(TypeName(kind)), $"the body has no node of the kind {kind}");
            Assert.True(
                shown.Contains(seen),
                $"the kind {kind} is not shown: {seen} is missing in\n{shown}"
            );
        }
        Assert.Contains("☑ CLOSED", shown);
    }

    private static Node.Text Text(string value) => new(value);

    private static ObservableCollection<Node> Nodes(params Node[] nodes) => new(nodes);

    private static Node.Paragraph Paragraph(params Node[] inline) => new(Nodes(inline));

    private static ShownText Shown(params Node[] blocks) =>
        ShownText.Of(new Node.Document(Nodes(blocks)));

    private static Node.List List(ListType type, uint start, params Node[] entries) =>
        new(type, start, true, ListDelimiter.Period, Nodes(entries));

    private static Node.Item Entry(params Node[] blocks) => new(ListType.Bullet, 1, Nodes(blocks));

    // タスクリストは箱の文字で、区切り線は線で、画像は代替の文字で出る。
    [Fact]
    public void A_task_list_a_line_and_an_image_are_put_into_text()
    {
        var shown = Shown(
            List(
                ListType.Bullet,
                1,
                new Node.TaskItem(null, Nodes(Paragraph(Text("OPEN")))),
                new Node.TaskItem("x", Nodes(Paragraph(Text("CLOSED"))))
            ),
            new Node.ThematicBreak(),
            Paragraph(
                new Node.Embed("images/a.png", "T", [], Nodes(Text("ALT"))),
                new Node.Embed("images/b.png", "", [], Nodes())
            )
        );

        Assert.Equal("☐ OPEN\n☑ CLOSED\n────────\n[ALT][images/b.png]", shown.Text);
    }

    // リンクは、表示するテキストに残る。ウィキリンクはページを、ハッシュタグはタグを指し、外のリンクはそのままである。
    [Fact]
    public void Links_stay_in_the_shown_text()
    {
        var line = Paragraph(
            new Node.Wikilink("notes/Idea", Nodes(Text("Idea"))),
            Text(" "),
            new Node.Hashtag("todo"),
            Text(" "),
            new Node.Link("https://example.com/a", "", Nodes(Text("out"))),
            Text(" "),
            new Node.Link("#fn-1", "", Nodes(Text("1")))
        );

        var runs = Assert.Single(Shown(line).Paragraphs).Runs;

        var links = runs.Where(run => run.Text.Trim().Length > 0)
            .ToDictionary(run => run.Text.Trim(), run => run.Link);
        Assert.Equal(new LinkTarget.Page("notes/Idea"), links["Idea"]);
        Assert.Equal(new LinkTarget.Tag("todo"), links["#todo"]);
        Assert.Equal(new LinkTarget.Out(new Uri("https://example.com/a")), links["out"]);
        // A link that stays in the item, as the one of a footnote, leads nowhere
        Assert.Null(links["1"]);
    }

    // 日本語の名前のウィキリンクは、その名前のページを指す。
    [Fact]
    public void A_wikilink_with_a_name_that_is_not_ASCII_names_its_page()
    {
        var run = Assert.Single(
            Assert
                .Single(
                    Shown(Paragraph(new Node.Wikilink("メモ/案", Nodes(Text("案"))))).Paragraphs
                )
                .Runs
        );

        Assert.Equal(new LinkTarget.Page("メモ/案"), run.Link);
    }

    // 文字の装飾は、表示するテキストの各部分に付く。
    [Fact]
    public void The_decoration_of_the_letters_is_kept()
    {
        var line = Paragraph(
            Text("plain "),
            new Node.Strong(Nodes(Text("bold"))),
            Text(" "),
            new Node.Emph(Nodes(Text("italic"))),
            Text(" "),
            new Node.Strikethrough(Nodes(Text("struck"))),
            Text(" "),
            new Node.Code("code"),
            Text(" "),
            new Node.Highlight(Nodes(Text("marked"))),
            Text(" "),
            new Node.Strong(Nodes(new Node.Emph(Nodes(Text("both")))))
        );

        var styles = Assert
            .Single(Shown(line).Paragraphs)
            .Runs.Where(run => run.Text.Trim().Length > 0)
            .ToDictionary(run => run.Text.Trim(), run => run.Style);

        Assert.Equal(RunStyle.None, styles["plain"]);
        Assert.Equal(RunStyle.Bold, styles["bold"]);
        Assert.Equal(RunStyle.Italic, styles["italic"]);
        Assert.Equal(RunStyle.Strike, styles["struck"]);
        Assert.Equal(RunStyle.Code, styles["code"]);
        Assert.Equal(RunStyle.Highlight, styles["marked"]);
        Assert.Equal(RunStyle.Bold | RunStyle.Italic, styles["both"]);
    }

    // 見出しは、その段の大きさで、太字で出る。大きさは、Web版と同じである。
    [Fact]
    public void A_heading_is_large_and_bold()
    {
        var shown = Shown(
            new Node.Heading(1, Nodes(Text("one"))),
            new Node.Heading(3, Nodes(Text("three"))),
            Paragraph(Text("text"))
        );

        Assert.Equal([1.6, 1.2, 1], shown.Paragraphs.Select(paragraph => paragraph.Scale));
        Assert.Equal(RunStyle.Bold, shown.Paragraphs[0].Runs[0].Style);
        Assert.Equal(RunStyle.None, shown.Paragraphs[2].Runs[0].Style);
    }

    // リストの項目は、印か番号が付き、入れ子の分だけ右に寄る。引用も右に寄る。
    [Fact]
    public void The_entries_of_a_list_have_marks_and_stand_to_the_right_by_their_depth()
    {
        var shown = Shown(
            List(
                ListType.Bullet,
                1,
                Entry(
                    Paragraph(Text("a")),
                    List(
                        ListType.Ordered,
                        3,
                        Entry(Paragraph(Text("b"))),
                        Entry(Paragraph(Text("c")))
                    )
                )
            ),
            new Node.BlockQuote(Nodes(Paragraph(Text("q"))))
        );

        Assert.Equal("• a\n3. b\n4. c\nq", shown.Text);
        Assert.Equal([0, 1, 1, 1], shown.Paragraphs.Select(paragraph => paragraph.Indent));
    }

    // 表の行は1行になり、セルは線で区切られる。
    [Fact]
    public void A_row_of_a_table_is_one_line()
    {
        static Node.TableRow Row(bool header, params string[] cells) =>
            new(
                header,
                Nodes([.. cells.Select(cell => (Node)new Node.TableCell(Nodes(Text(cell))))])
            );

        var table = new Node.Table(
            [Alignment.None, Alignment.None],
            2,
            2,
            Nodes(Row(true, "H1", "H2"), Row(false, "a", "b"))
        );

        Assert.Equal("H1  │  H2\na  │  b", Shown(table).Text);
    }

    // コードブロックの空白と改行は、そのまま出る。段落の中の改行は、空白1つになる。
    [Fact]
    public void The_white_space_of_a_code_block_is_kept_and_that_of_a_paragraph_is_one_space()
    {
        var shown = Shown(
            new Node.CodeBlock(true, "", "a\n  b\n"),
            Paragraph(
                Text("soft"),
                new Node.Softbreak(),
                Text("break"),
                new Node.Linebreak(),
                Text("hard")
            )
        );

        Assert.Equal("a\n  b\nsoft break\nhard", shown.Text);
    }

    // 未知の種類は、中身がそのまま出る。
    [Fact]
    public void A_kind_that_is_not_known_shows_what_is_in_it()
    {
        Assert.Equal(
            "inside",
            Shown(Paragraph(new Node.Unknown("insert", Nodes(Text("inside"))))).Text
        );
    }

    // 表示するテキストの終わりに、余分な改行はない。
    [Fact]
    public void The_shown_text_does_not_end_with_a_newline()
    {
        Assert.Equal("one\ntwo", Shown(Paragraph(Text("one")), Paragraph(Text("two"))).Text);
    }

    // 本文のない項目は、何も出ない。
    [Fact]
    public void An_empty_item_shows_nothing()
    {
        Assert.Empty(Shown().Paragraphs);
    }
}
