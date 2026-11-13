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
/// The text of an item as it is shown: with its decoration, not as Markdown. It is drawn from
/// the AST that the core gives with the row (ADR 041). <c>Node</c> is a closed record, so a
/// <c>switch</c> expression that leaves out a kind of node stops the build (CS8509): a kind
/// that the AST gets later is drawn here before the shell builds again.
/// </summary>
public sealed record ShownText(IReadOnlyList<ShownParagraph> Paragraphs)
{
    /// <summary>The letters of the text, with a line break between its paragraphs.</summary>
    public string Text => string.Join('\n', Paragraphs.Select(paragraph => paragraph.Text));

    /// <summary>The text of the AST of an item.</summary>
    public static ShownText Of(Node ast)
    {
        ArgumentNullException.ThrowIfNull(ast);
        var writer = new TextWriter();
        writer.Blocks(Nodes.Children(ast));
        return new ShownText(writer.Finish());
    }
}

/// <summary>What the nodes of the AST hold.</summary>
internal static class Nodes
{
    /// <summary>The children of a node, and none for a leaf.</summary>
    public static IReadOnlyList<Node> Children(Node node) =>
        node switch
        {
            Node.Document n => n.Children,
            Node.Paragraph n => n.Children,
            Node.Heading n => n.Children,
            Node.BlockQuote n => n.Children,
            Node.MultilineBlockQuote n => n.Children,
            Node.List n => n.Children,
            Node.Item n => n.Children,
            Node.TaskItem n => n.Children,
            Node.Table n => n.Children,
            Node.TableRow n => n.Children,
            Node.TableCell n => n.Children,
            Node.FootnoteDefinition n => n.Children,
            Node.Alert n => n.Children,
            Node.Subtext n => n.Children,
            Node.Emph n => n.Children,
            Node.Strong n => n.Children,
            Node.Strikethrough n => n.Children,
            Node.Underline n => n.Children,
            Node.Highlight n => n.Children,
            Node.Superscript n => n.Children,
            Node.SubscriptText n => n.Children,
            Node.SpoileredText n => n.Children,
            Node.Link n => n.Children,
            Node.Embed n => n.Children,
            Node.Wikilink n => n.Children,
            Node.Unknown n => n.Children,
            Node.CodeBlock
            or Node.ThematicBreak
            or Node.HtmlBlock
            or Node.Text
            or Node.Softbreak
            or Node.Linebreak
            or Node.Code
            or Node.FootnoteReference
            or Node.Shortcode
            or Node.Math
            or Node.HtmlInline
            or Node.Hashtag => [],
        };

    /// <summary>The text under the node, without its decoration.</summary>
    public static string PlainText(Node node) =>
        node is Node.Text text ? text.Value : string.Concat(Children(node).Select(PlainText));

    /// <summary>
    /// The text that a reader sees for a footnote. An inline footnote has a name that the
    /// parser made (<c>__inline_1</c>), so its number is shown instead.
    /// </summary>
    public static string FootnoteLabel(string name) =>
        name.StartsWith("__inline_", StringComparison.Ordinal) ? name["__inline_".Length..] : name;
}

/// <summary>Writes the blocks of an item as paragraphs, one line or more each.</summary>
internal sealed class TextWriter
{
    private readonly List<ShownParagraph> paragraphs = [];
    private List<ShownRun> runs = [];
    private RunStyle style;
    private LinkTarget? link;
    private double scale = 1;
    private int indent;

    /// <summary>
    /// What the first line of the next block starts with: the mark of an entry of a list, or
    /// the name of a footnote.
    /// </summary>
    private string? lead;

    public IReadOnlyList<ShownParagraph> Finish()
    {
        EndParagraph();
        return paragraphs;
    }

    public void Blocks(IEnumerable<Node> nodes)
    {
        foreach (var node in nodes)
            Block(node);
    }

    private void Block(Node node)
    {
        Action write = node switch
        {
            Node.Document or Node.Item or Node.TaskItem or Node.Unknown or Node.Table => () =>
                Blocks(Nodes.Children(node)),
            Node.Paragraph or Node.TableCell => () => Line(Nodes.Children(node)),
            Node.Heading heading => () =>
                With(
                    () => Line(heading.Children),
                    added: RunStyle.Bold,
                    scaled: RowLook.HeadingScales[
                        Math.Clamp(heading.Level - 1, 0, RowLook.HeadingScales.Count - 1)
                    ]
                ),
            Node.CodeBlock code => () =>
                With(() => Line([new Node.Text(code.Literal.TrimEnd('\n'))]), added: RunStyle.Code),
            Node.BlockQuote or Node.MultilineBlockQuote => () =>
                Quote(() => Blocks(Nodes.Children(node))),
            Node.Alert alert => () =>
                Quote(() =>
                {
                    With(
                        () => Line([new Node.Text(alert.Title ?? alert.AlertType.ToString())]),
                        added: RunStyle.Bold
                    );
                    Blocks(alert.Children);
                }),
            Node.List list => () => List(list),
            Node.TableRow row => () => Row(row),
            Node.ThematicBreak => () => Line([new Node.Text("────────")]),
            Node.HtmlBlock html => () => Line([new Node.Text(html.Literal.TrimEnd('\n'))]),
            Node.FootnoteDefinition note => () =>
            {
                lead = $"[{Nodes.FootnoteLabel(note.Name)}]: ";
                Blocks(note.Children);
                lead = null;
            },
            Node.Subtext subtext => () => With(() => Line(subtext.Children), added: RunStyle.Small),
            // A line outside a block, as in a list without room between its entries
            Node.Text
            or Node.Softbreak
            or Node.Linebreak
            or Node.Emph
            or Node.Strong
            or Node.Strikethrough
            or Node.Underline
            or Node.Highlight
            or Node.Superscript
            or Node.SubscriptText
            or Node.SpoileredText
            or Node.Code
            or Node.Link
            or Node.Embed
            or Node.FootnoteReference
            or Node.Shortcode
            or Node.Math
            or Node.HtmlInline
            or Node.Wikilink
            or Node.Hashtag => () => Line([node]),
        };
        write();
    }

    /// <summary>
    /// The entries of a list, each with its mark: a box for a task, its number, or a bullet. A
    /// list inside an entry stands one step more to the right.
    /// </summary>
    private void List(Node.List list)
    {
        var number = (int)list.Start;
        depth++;
        foreach (var entry in list.Children)
        {
            lead = entry switch
            {
                Node.TaskItem task => task.Symbol is null ? "☐ " : "☑ ",
                _ when list.ListType == ListType.Ordered => $"{number}. ",
                _ => "• ",
            };
            number++;
            Block(entry);
            lead = null;
        }
        depth--;
    }

    /// <summary>How many lists the block is inside.</summary>
    private int depth;

    /// <summary>The cells of a row stand in one line.</summary>
    private void Row(Node.TableRow row)
    {
        var cells = new List<Node>();
        foreach (var cell in row.Children)
        {
            if (cells.Count > 0)
                cells.Add(new Node.Text(Separator));
            cells.AddRange(Nodes.Children(cell));
        }
        With(() => Line(cells), added: row.Header ? RunStyle.Bold : RunStyle.None);
    }

    private const string Separator = "  │  ";

    private void Quote(Action inside)
    {
        indent++;
        With(inside, added: RunStyle.Quote);
        indent--;
    }

    /// <summary>Does what is inside with more style, and the style before it afterwards.</summary>
    private void With(Action inside, RunStyle added, double? scaled = null)
    {
        var before = (style, scale);
        style |= added;
        scale = scaled ?? scale;
        inside();
        (style, scale) = before;
    }

    /// <summary>One paragraph, with the lead of the block before it.</summary>
    private void Line(IEnumerable<Node> inline)
    {
        EndParagraph();
        var at = indent + Math.Max(depth - 1, 0);
        if (lead is not null)
        {
            Add(lead, RunStyle.None);
            lead = null;
        }
        foreach (var node in inline)
            Write(node);
        EndParagraph(at);
    }

    /// <summary>The letters of an inline node, with the style of the nodes around it.</summary>
    private void Write(Node node)
    {
        Action write = node switch
        {
            Node.Text text => () => Add(text.Value),
            Node.Softbreak => () => Add(" "),
            Node.Linebreak => () => Add("\n"),
            Node.Emph n => () => With(() => WriteAll(n.Children), RunStyle.Italic),
            Node.Strong n => () => With(() => WriteAll(n.Children), RunStyle.Bold),
            Node.Strikethrough n => () => With(() => WriteAll(n.Children), RunStyle.Strike),
            Node.Underline n => () => With(() => WriteAll(n.Children), RunStyle.Underline),
            Node.Highlight n => () => With(() => WriteAll(n.Children), RunStyle.Highlight),
            Node.Superscript n => () => With(() => WriteAll(n.Children), RunStyle.Superscript),
            Node.SubscriptText n => () => With(() => WriteAll(n.Children), RunStyle.Subscript),
            Node.SpoileredText n => () => With(() => WriteAll(n.Children), RunStyle.Spoiler),
            Node.Code code => () => With(() => Add(code.Literal), RunStyle.Code),
            Node.Link n => () => Linked(Target(n.Url), n.Children),
            Node.Wikilink n => () => Linked(new LinkTarget.Page(n.Url), n.Children),
            Node.Hashtag tag => () =>
                Linked(new LinkTarget.Tag(tag.Value), [new Node.Text("#" + tag.Value)]),
            // An image is shown as the words for it, and without any as its file. The image
            // itself is not shown yet
            Node.Embed embed => () =>
            {
                var words = Nodes.PlainText(embed);
                Add($"[{(words.Length == 0 ? embed.Url : words)}]");
            },
            Node.FootnoteReference reference => () =>
                With(() => Add($"[{Nodes.FootnoteLabel(reference.Name)}]"), RunStyle.Superscript),
            Node.Shortcode shortcode => () => Add(shortcode.Emoji),
            Node.Math math => () => Add(math.Literal),
            Node.HtmlInline html => () => Add(html.Value),
            Node.Unknown n => () => WriteAll(n.Children),
            // A block inside a line: only its letters
            Node.Document
            or Node.Paragraph
            or Node.Heading
            or Node.CodeBlock
            or Node.BlockQuote
            or Node.MultilineBlockQuote
            or Node.List
            or Node.Item
            or Node.TaskItem
            or Node.Table
            or Node.TableRow
            or Node.TableCell
            or Node.ThematicBreak
            or Node.HtmlBlock
            or Node.FootnoteDefinition
            or Node.Alert
            or Node.Subtext => () => Add(Nodes.PlainText(node)),
        };
        write();
    }

    private void WriteAll(IEnumerable<Node> nodes)
    {
        foreach (var node in nodes)
            Write(node);
    }

    private void Linked(LinkTarget? target, IEnumerable<Node> inside)
    {
        var before = link;
        link = target ?? link;
        WriteAll(inside);
        link = before;
    }

    /// <summary>
    /// What a link leads to: an address outside the journal, or a page by its path. Nothing
    /// for a link that stays in the item, as the one of a footnote.
    /// </summary>
    private static LinkTarget? Target(string url)
    {
        if (url.Length == 0 || url.StartsWith('#'))
            return null;
        if (Uri.TryCreate(url, UriKind.Absolute, out var address) && !address.IsFile)
            return new LinkTarget.Out(address);
        return new LinkTarget.Page(Uri.UnescapeDataString(url));
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

    private void EndParagraph(int at = 0)
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
        paragraphs.Add(new ShownParagraph(runs, scale, at));
        runs = [];
    }
}
