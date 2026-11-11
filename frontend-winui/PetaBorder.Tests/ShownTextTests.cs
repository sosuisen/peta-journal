using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>
/// The text of an item as it is shown, with its decoration (ADR 025). The AST of the first
/// test comes from a real journal server, so that test is skipped without
/// <c>PETA_E2E_SERVER</c>.
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

    /// <summary>An item with this body, as the journal server gives it back: with its AST.</summary>
    private static Item ItemWithBody(string body, JournalServer server)
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
            "the server gave the AST",
            () => shell.View.Outline.Rows[0].Item.Ast.Length > 0
        );
        return shell.View.Outline.Rows[0].Item;
    }

    // ASTのどの種類の中身も、表示するテキストに入っている。
    [Fact]
    public void What_is_in_every_kind_of_node_is_in_the_shown_text()
    {
        using var server = new JournalServer(18112);
        var item = ItemWithBody(
            string.Join("\n\n", EveryKind.Select(kind => kind.Markdown)),
            server
        );

        var shown = ShownText.Of(item).Text;

        foreach (var (kind, _, seen) in EveryKind)
        {
            Assert.True(
                item.Ast.Contains($"\"type\":\"{kind}\""),
                $"the body has no node of the kind {kind}"
            );
            Assert.True(
                shown.Contains(seen),
                $"the kind {kind} is not shown: {seen} is missing in\n{shown}"
            );
        }
        Assert.Contains("☑ CLOSED", shown);
    }

    // 本文の中のタスクリストは箱の文字で、区切り線は線で、画像は代替の文字で出る。
    [Fact]
    public void A_task_list_a_line_and_an_image_are_put_into_text()
    {
        var html =
            "<ul>\n<li><input type=\"checkbox\" disabled>OPEN</li>\n<li><input type=\"checkbox\" checked disabled>CLOSED</li>\n</ul>\n"
            + "<hr />\n<p><img src=\"images/a.png\" alt=\"ALT\" title=\"T\" /><img src=\"images/b.png\" alt=\"\" /></p>\n";

        var shown = ShownText.OfHtml(html).Text;

        Assert.Equal("☐ OPEN\n☑ CLOSED\n────────\n[ALT][images/b.png]", shown);
    }

    // リンクは、表示するテキストに残る。ウィキリンクはページを、ハッシュタグはタグを指し、外のリンクはそのままである。
    [Fact]
    public void Links_stay_in_the_shown_text()
    {
        var html =
            "<p><a href=\"notes/Idea\" data-wikilink=\"true\">Idea</a> <a href=\"todo\" class=\"hashtag\">#todo</a> "
            + "<a href=\"https://example.com/a\">out</a> <a href=\"#fn-1\">1</a></p>\n";

        var runs = Assert.Single(ShownText.OfHtml(html).Paragraphs).Runs;

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
        var html = "<p><a href=\"%E3%83%A1%E3%83%A2/%E6%A1%88\" data-wikilink=\"true\">案</a></p>";

        var run = Assert.Single(Assert.Single(ShownText.OfHtml(html).Paragraphs).Runs);

        Assert.Equal(new LinkTarget.Page("メモ/案"), run.Link);
    }

    // 文字の装飾は、表示するテキストの各部分に付く。
    [Fact]
    public void The_decoration_of_the_letters_is_kept()
    {
        var html =
            "<p>plain <strong>bold</strong> <em>italic</em> <del>struck</del> <code>code</code> "
            + "<mark>marked</mark> <strong><em>both</em></strong></p>";

        var styles = Assert
            .Single(ShownText.OfHtml(html).Paragraphs)
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
        var shown = ShownText.OfHtml("<h1>one</h1>\n<h3>three</h3>\n<p>text</p>\n");

        Assert.Equal([1.6, 1.2, 1], shown.Paragraphs.Select(paragraph => paragraph.Scale));
        Assert.Equal(RunStyle.Bold, shown.Paragraphs[0].Runs[0].Style);
        Assert.Equal(RunStyle.None, shown.Paragraphs[2].Runs[0].Style);
    }

    // リストの項目は、印か番号が付き、入れ子の分だけ右に寄る。引用も右に寄る。
    [Fact]
    public void The_entries_of_a_list_have_marks_and_stand_to_the_right_by_their_depth()
    {
        var html =
            "<ul>\n<li>a\n<ol start=\"3\">\n<li>b</li>\n<li>c</li>\n</ol>\n</li>\n</ul>\n<blockquote>\n<p>q</p>\n</blockquote>\n";

        var shown = ShownText.OfHtml(html);

        Assert.Equal("• a\n3. b\n4. c\nq", shown.Text);
        Assert.Equal([0, 1, 1, 1], shown.Paragraphs.Select(paragraph => paragraph.Indent));
    }

    // 表の行は1行になり、セルは線で区切られる。
    [Fact]
    public void A_row_of_a_table_is_one_line()
    {
        var html =
            "<table>\n<thead>\n<tr>\n<th>H1</th>\n<th>H2</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td>a</td>\n<td>b</td>\n</tr>\n</tbody>\n</table>\n";

        Assert.Equal("H1  │  H2\na  │  b", ShownText.OfHtml(html).Text);
    }

    // コードブロックの空白と改行は、そのまま出る。段落の中の改行は、空白1つになる。
    [Fact]
    public void The_white_space_of_a_code_block_is_kept_and_that_of_a_paragraph_is_one_space()
    {
        var shown = ShownText.OfHtml(
            "<pre><code>a\n  b\n</code></pre>\n<p>soft\nbreak<br />\nhard</p>\n"
        );

        Assert.Equal("a\n  b\nsoft break\nhard", shown.Text);
    }

    // 知らないタグは、中身がそのまま出る。スクリプトは出ない。
    [Fact]
    public void A_tag_that_is_not_known_shows_what_is_in_it()
    {
        var shown = ShownText.OfHtml(
            "<p><custom-tag>inside</custom-tag><script>alert(1)</script></p>"
        );

        Assert.Equal("inside", shown.Text);
    }

    // 表示するテキストの終わりに、余分な改行はない。
    [Fact]
    public void The_shown_text_does_not_end_with_a_newline()
    {
        Assert.Equal("one\ntwo", ShownText.OfHtml("<p>one</p>\n<p>two</p>\n").Text);
    }

    // ASTが読めない項目は、本文がそのまま出る。本文のない項目は、何も出ない。
    [Fact]
    public void An_item_whose_AST_cannot_be_read_shows_its_body()
    {
        static Item Item(string body, string ast) =>
            new()
            {
                Id = "a",
                PageId = "p",
                DirPath = "",
                Name = "notes",
                ItemType = "doc",
                Idx = "a",
                Level = 1,
                Body = body,
                Ast = ast,
                Types = [],
                Properties = [],
            };

        Assert.Equal("**as it is** <b>", ShownText.Of(Item("**as it is** <b>", "{\"type\":")).Text);
        Assert.Equal("**no AST yet**", ShownText.Of(Item("**no AST yet**", "")).Text);
        Assert.Empty(ShownText.Of(Item("", "")).Paragraphs);
    }
}
