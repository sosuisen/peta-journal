using System.Text;
using AngleSharp.Dom;
using AngleSharp.Html.Parser;
using PetaTypes;

namespace PetaBorder;

/// <summary>How a piece of a shown text looks. The screen decides the fonts and the colors.</summary>
[Flags]
public enum RunStyle
{
    None = 0,
    Bold = 1 << 0,
    Italic = 1 << 1,
    Strike = 1 << 2,
    Underline = 1 << 3,

    /// <summary>Code: letters of one width.</summary>
    Code = 1 << 4,

    /// <summary>Marked, as with a pen.</summary>
    Highlight = 1 << 5,

    /// <summary>Text that is not to be read by accident.</summary>
    Spoiler = 1 << 6,

    /// <summary>The words of somebody else: paler than the text.</summary>
    Quote = 1 << 7,
    Superscript = 1 << 8,
    Subscript = 1 << 9,

    /// <summary>A remark below the text: smaller and paler.</summary>
    Small = 1 << 10,
}

/// <summary>A piece of a shown text that looks the same all over.</summary>
/// <param name="Link">What a click on the piece opens, when it is part of a link.</param>
public sealed record ShownRun(string Text, RunStyle Style, LinkTarget? Link);

/// <summary>A line or a block of a shown text.</summary>
/// <param name="Scale">How much larger than the usual text it is: more than 1 for a heading.</param>
/// <param name="Indent">How many steps to the right it stands, in a list or a quote.</param>
public sealed record ShownParagraph(IReadOnlyList<ShownRun> Runs, double Scale, int Indent)
{
    public string Text => string.Concat(Runs.Select(run => run.Text));
}

/// <summary>What a link of a shown text leads to.</summary>
public abstract record LinkTarget
{
    /// <summary>The page of a wikilink, by its path.</summary>
    public sealed record Page(string Path) : LinkTarget;

    /// <summary>The tag of a hashtag, by its name. Its page lies in the folder of the tags.</summary>
    public sealed record Tag(string Name) : LinkTarget;

    /// <summary>An address outside the journal.</summary>
    public sealed record Out(Uri Address) : LinkTarget;
}

/// <summary>
/// The text of an item as it is shown: with its decoration, not as Markdown (ADR 025).
///
/// The shell does not read the AST. Rust makes HTML of it, with <c>ast_to_html</c> of
/// flatmarkdown-ast2html, which takes every kind of node. This reads the HTML (ADR 028):
/// Windows has nothing that turns HTML into text with decoration. A tag that is not known
/// here shows what is in it, so nothing that was written is left out.
/// </summary>
public sealed record ShownText(IReadOnlyList<ShownParagraph> Paragraphs)
{
    /// <summary>The letters of the text, with a line break between its paragraphs.</summary>
    public string Text => string.Join('\n', Paragraphs.Select(paragraph => paragraph.Text));

    /// <summary>
    /// The text of the item. An item whose AST cannot be read shows its body as it is.
    /// </summary>
    public static ShownText Of(Item item)
    {
        ArgumentNullException.ThrowIfNull(item);
        // The AST of what was just typed comes from the server a moment later
        if (item.Ast.Length == 0)
            return Plain(item.Body);
        try
        {
            return OfHtml(PetaCore.PetaNative.ItemHtml(item.Ast));
        }
        catch (PetaCore.BoltException)
        {
            return Plain(item.Body);
        }
    }

    internal static ShownText OfHtml(string html)
    {
        var document = new HtmlParser().ParseDocument($"<body>{html}</body>");
        var reader = new HtmlReader();
        reader.Read(document.Body!);
        return new ShownText(reader.Finish());
    }

    private static ShownText Plain(string body) =>
        new(
            body.Length == 0
                ? []
                : [new ShownParagraph([new ShownRun(body, RunStyle.None, null)], 1, 0)]
        );
}

/// <summary>Walks the HTML of an item and collects its paragraphs.</summary>
internal sealed class HtmlReader
{
    /// <summary>
    /// The tags that start a line of their own. Every other tag stays in the line it is in.
    /// </summary>
    private static readonly HashSet<string> Blocks =
    [
        "p",
        "div",
        "section",
        "article",
        "ul",
        "ol",
        "li",
        "blockquote",
        "pre",
        "table",
        "thead",
        "tbody",
        "tfoot",
        "tr",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "details",
        "summary",
        "dl",
        "dt",
        "dd",
        "figure",
        "figcaption",
        "hr",
        "header",
        "footer",
        "aside",
        "nav",
    ];

    /// <summary>The tags that have no text of their own, only other tags.</summary>
    private static readonly HashSet<string> Layout =
    [
        "table",
        "thead",
        "tbody",
        "tfoot",
        "tr",
        "ul",
        "ol",
    ];

    private static readonly Dictionary<string, RunStyle> Styles = new()
    {
        ["strong"] = RunStyle.Bold,
        ["b"] = RunStyle.Bold,
        ["th"] = RunStyle.Bold,
        ["summary"] = RunStyle.Bold,
        ["em"] = RunStyle.Italic,
        ["i"] = RunStyle.Italic,
        ["del"] = RunStyle.Strike,
        ["s"] = RunStyle.Strike,
        ["u"] = RunStyle.Underline,
        ["ins"] = RunStyle.Underline,
        ["code"] = RunStyle.Code,
        ["pre"] = RunStyle.Code,
        ["kbd"] = RunStyle.Code,
        ["mark"] = RunStyle.Highlight,
        ["blockquote"] = RunStyle.Quote,
        ["sup"] = RunStyle.Superscript,
        ["sub"] = RunStyle.Subscript,
        ["small"] = RunStyle.Small,
    };

    /// <summary>The classes that <c>ast_to_html</c> gives to what HTML has no tag for.</summary>
    private static readonly Dictionary<string, RunStyle> ClassStyles = new()
    {
        ["alert-title"] = RunStyle.Bold,
        ["spoiler"] = RunStyle.Spoiler,
        ["subtext"] = RunStyle.Small,
    };

    private readonly List<ShownParagraph> paragraphs = [];
    private List<ShownRun> runs = [];
    private RunStyle style;
    private LinkTarget? link;
    private double scale = 1;
    private int indent;

    /// <summary>The white space of the text counts, as in a block of code.</summary>
    private bool keepsSpace;

    /// <summary>The lists that are open, each with the number of its next entry: 0 for a list
    /// with bullets.</summary>
    private readonly Stack<int> lists = new();

    public IReadOnlyList<ShownParagraph> Finish()
    {
        EndParagraph();
        return paragraphs;
    }

    public void Read(INode node)
    {
        foreach (var child in node.ChildNodes)
        {
            if (child is IText text)
            {
                // Between the rows and the cells of a table, and between the entries of a
                // list, white space is only how the HTML is laid out
                if (
                    !(
                        Layout.Contains(node.NodeName.ToLowerInvariant())
                        && string.IsNullOrWhiteSpace(text.Data)
                    )
                )
                    Write(text.Data);
            }
            else if (child is IElement element)
                Read(element);
        }
    }

    private void Read(IElement element)
    {
        var tag = element.LocalName;
        switch (tag)
        {
            case "script" or "style" or "head":
                return;
            case "br":
                Add("\n");
                return;
            case "hr":
                EndParagraph();
                Add("────────");
                EndParagraph();
                return;
            case "img":
                // An image is shown as the words for it, and without any as its file. The
                // image itself is not shown yet
                var words = element.GetAttribute("alt");
                Add($"[{(string.IsNullOrEmpty(words) ? element.GetAttribute("src") : words)}]");
                return;
            case "input":
                if (element.GetAttribute("type") == "checkbox")
                    Add(element.HasAttribute("checked") ? "☑ " : "☐ ");
                return;
        }

        var before = (style, link, scale, indent, keepsSpace);
        var isBlock = Blocks.Contains(tag);
        if (isBlock)
            EndParagraph();
        if (Styles.TryGetValue(tag, out var added))
            style |= added;
        foreach (var name in element.ClassList)
        {
            if (ClassStyles.TryGetValue(name, out var byClass))
                style |= byClass;
        }
        switch (tag)
        {
            case "h1" or "h2" or "h3" or "h4" or "h5" or "h6":
                scale = RowLook.HeadingScales[
                    Math.Min(tag[1] - '1', RowLook.HeadingScales.Count - 1)
                ];
                style |= RunStyle.Bold;
                break;
            case "a":
                link = Target(element) ?? link;
                break;
            case "pre":
                keepsSpace = true;
                break;
            case "blockquote":
                indent++;
                break;
            case "ul" or "ol":
                if (lists.Count > 0)
                    indent++;
                lists.Push(tag == "ol" ? StartOf(element) : 0);
                break;
            case "li":
                StartEntry(element);
                break;
            case "td" or "th":
                // The cells of a row stand in one line
                if (element.PreviousElementSibling is not null)
                    Add("  │  ", RunStyle.None);
                break;
        }

        Read((INode)element);

        if (tag is "ul" or "ol")
            lists.Pop();
        if (isBlock)
            EndParagraph();
        (style, link, scale, indent, keepsSpace) = before;
    }

    /// <summary>
    /// The mark before an entry of a list: its number, or a bullet. The box of a task list
    /// stands in the place of the bullet.
    /// </summary>
    private void StartEntry(IElement entry)
    {
        if (lists.Count == 0)
            return;
        var number = lists.Pop();
        lists.Push(number == 0 ? 0 : number + 1);
        if (number > 0)
            Add($"{number}. ", RunStyle.None);
        else if (entry.QuerySelector("input[type=checkbox]") is null)
            Add("• ", RunStyle.None);
    }

    private static int StartOf(IElement list) =>
        int.TryParse(list.GetAttribute("start"), out var start) && start > 0 ? start : 1;

    /// <summary>
    /// What the link leads to. A wikilink and a hashtag name a page or a tag, and have no
    /// address. Nothing for a link that stays in the item, as the one of a footnote.
    /// </summary>
    private static LinkTarget? Target(IElement anchor)
    {
        var href = anchor.GetAttribute("href");
        if (string.IsNullOrEmpty(href) || href.StartsWith('#'))
            return null;
        if (anchor.ClassList.Contains("hashtag"))
            return new LinkTarget.Tag(Uri.UnescapeDataString(href));
        if (Uri.TryCreate(href, UriKind.Absolute, out var address) && !address.IsFile)
            return new LinkTarget.Out(address);
        return new LinkTarget.Page(Uri.UnescapeDataString(href));
    }

    private void Write(string text)
    {
        if (keepsSpace)
        {
            Add(text);
            return;
        }
        // HTML counts any run of white space as one space, and none at the start of a line
        var kept = new StringBuilder(text.Length);
        var atStart =
            runs.Count == 0 || runs[^1].Text.EndsWith(' ') || runs[^1].Text.EndsWith('\n');
        foreach (var letter in text)
        {
            if (!char.IsWhiteSpace(letter))
                kept.Append(letter);
            else if (!atStart)
                kept.Append(' ');
            atStart = char.IsWhiteSpace(letter);
        }
        Add(kept.ToString());
    }

    private void Add(string text) => Add(text, style);

    private void Add(string text, RunStyle with)
    {
        if (text.Length == 0)
            return;
        // Pieces that look the same are one piece
        if (runs.Count > 0 && runs[^1].Style == with && runs[^1].Link == link)
            runs[^1] = runs[^1] with { Text = runs[^1].Text + text };
        else
            runs.Add(new ShownRun(text, with, link));
    }

    private void EndParagraph()
    {
        // The white space and the line break at the end of a block are not shown
        while (runs.Count > 0)
        {
            var trimmed = runs[^1].Text.TrimEnd(' ', '\n');
            if (trimmed.Length > 0)
            {
                runs[^1] = runs[^1] with { Text = trimmed };
                break;
            }
            runs.RemoveAt(runs.Count - 1);
        }
        if (runs.Count == 0)
            return;
        paragraphs.Add(new ShownParagraph(runs, scale, indent));
        runs = [];
    }
}
