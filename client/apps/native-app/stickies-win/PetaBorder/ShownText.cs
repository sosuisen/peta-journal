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
}

/// <summary>A piece of a shown text that looks the same all over.</summary>
/// <param name="Link">Where a click on the piece leads, when it is part of a link.</param>
/// <param name="Decoration">
/// True for letters that this shell adds, such as the mark of an entry of a list. They are not
/// letters of the shown text that the core counts.
/// </param>
public sealed record ShownRun(string Text, RunStyle Style, RichLink? Link, bool Decoration = false)
{
    /// <summary>
    /// How large the letters are, against the letters of their paragraph. Raised and lowered
    /// letters are smaller, as in the other shells. The OpenType variants of the fonts do not
    /// make them so: Segoe UI and Yu Gothic UI change only the digits, and Noto Sans JP has none.
    /// </summary>
    public double Scale => (Style & (RunStyle.Superscript | RunStyle.Subscript)) != 0 ? 0.75 : 1;
}

/// <summary>
/// One part of an item as the screen draws it, from top to bottom. A <c>switch</c> that leaves
/// out a kind of part stops the build (CS8509).
/// </summary>
public closed record ShownPart;

/// <summary>A line or a block of a shown text.</summary>
/// <param name="Scale">How much larger than the usual text it is: more than 1 for a heading.</param>
/// <param name="Indent">How far to the right it stands, in ems, in a list or a quote.</param>
/// <param name="Start">
/// How many letters of the shown text of the item stand before it, as the core counts them
/// (<c>RichParagraph.Start</c>). None for a line that has only the mark of an entry.
/// </param>
public sealed record ShownParagraph(
    IReadOnlyList<ShownRun> Runs,
    double Scale,
    double Indent,
    int? Start
) : ShownPart
{
    public string Text => string.Concat(Runs.Select(run => run.Text));

    /// <summary>
    /// How many letters of the shown text of the item stand before a touch
    /// <paramref name="place"/> UTF-16 units into <see cref="Text"/>, as the core counts them
    /// (ADR 043). A touch on a decoration is at the letter after it. None for a line that has
    /// only the mark of an entry.
    /// </summary>
    public int? LetterAt(int place)
    {
        if (Start is not { } letters)
            return null;
        var at = 0;
        foreach (var run in Runs)
        {
            var end = at + run.Text.Length;
            if (!run.Decoration)
            {
                if (place <= end)
                    return letters
                        + (int)
                            PetaCore.PetaNative.CharsOfUtf16(
                                run.Text,
                                (ulong)Math.Max(place - at, 0)
                            );
                letters += (int)PetaCore.PetaNative.CharsOfUtf16(run.Text, (ulong)run.Text.Length);
            }
            at = end;
        }
        return letters;
    }
}

/// <summary>A table: its rows, each with a paragraph in each cell.</summary>
public sealed record ShownTable(IReadOnlyList<ShownTableRow> Rows, double Indent) : ShownPart;

/// <param name="Header">True for the row of the titles of the columns.</param>
public sealed record ShownTableRow(bool Header, IReadOnlyList<ShownParagraph> Cells);

/// <summary>An image of the assets folder.</summary>
/// <param name="Path">Where its file is under the Markdown folder, such as <c>assets/a.png</c>.</param>
/// <param name="Width">The width that the page asks for, at a zoom of 1. None when it asks for none.</param>
/// <param name="Height">The height that the page asks for, the same way.</param>
public sealed record ShownPicture(string Path, string Alt, uint? Width, uint? Height, double Indent)
    : ShownPart
{
    /// <summary>What stands for the image until its file is here.</summary>
    public string Words => $"[{(Alt.Length == 0 ? Path : Alt)}]";
}

/// <summary>
/// The text of an item as it is shown: with its decoration, not as Markdown. The core makes the
/// parts of the body and gives them with the row (ADR 042); this puts them in the parts that
/// the screen draws. A <c>RichBlock</c> is a closed record, so a kind of part that the core
/// makes later stops the build until it is drawn here.
/// </summary>
public sealed record ShownText(IReadOnlyList<ShownPart> Parts)
{
    /// <summary>
    /// The letters of the text, with a line break between its parts. The cells of a row of a
    /// table stand in one line, and an image is its words.
    /// </summary>
    public string Text => string.Join('\n', Parts.Select(TextOf));

    /// <summary>The letters of a part, as <see cref="Text"/> has them.</summary>
    public static string TextOf(ShownPart part) =>
        part switch
        {
            ShownParagraph paragraph => paragraph.Text,
            ShownTable table => string.Join(
                '\n',
                table.Rows.Select(row => string.Join(" ", row.Cells.Select(cell => cell.Text)))
            ),
            ShownPicture picture => picture.Words,
        };

    /// <summary>The text of the parts that the core made of an item.</summary>
    public static ShownText Of(RichText rich)
    {
        ArgumentNullException.ThrowIfNull(rich);
        var writer = new PartWriter();
        writer.Blocks(rich.Blocks, indent: 0, quoted: 0);
        return new ShownText(writer.Parts);
    }
}

/// <summary>
/// Puts the parts of an item one below the other. A list and a quote that the core took apart
/// move what is in them to the right, as far as the core says. The sizes and the indents come
/// from the core, so that every shell has the same ones (#795).
/// </summary>
internal sealed class PartWriter
{
    private const string Rule = "────────";

    public List<ShownPart> Parts { get; } = [];

    /// <summary>The mark of the entry of a list that the next part is the first of.</summary>
    private string? lead;

    /// <summary>
    /// The parts <paramref name="indent"/> ems to the right, inside <paramref name="quoted"/>
    /// quotes. The core says how far each list and quote moves what is in it (#795).
    /// </summary>
    public void Blocks(IEnumerable<RichBlock> blocks, double indent, int quoted)
    {
        foreach (var block in blocks)
        {
            Action write = block switch
            {
                RichBlock.Text text => () =>
                {
                    foreach (var paragraph in text.Paragraphs)
                        Paragraph(
                            paragraph,
                            indent + paragraph.Indent,
                            quoted + (int)paragraph.Quoted
                        );
                },
                RichBlock.Code code => () =>
                    Line(
                        [new ShownRun(code.Lines, Quoted(RunStyle.Code, quoted), null)],
                        1,
                        indent,
                        (int)code.Start
                    ),
                RichBlock.Table table => () => Table(table.Rows, indent, quoted),
                RichBlock.Image image => () => Picture(image.Image_, indent, quoted),
                RichBlock.Entries entries => () =>
                    Entries(entries.Entries_, indent + entries.Indent, quoted),
                RichBlock.Quote quote => () =>
                    Blocks(quote.Blocks, indent + quote.Indent, quoted + 1),
            };
            write();
        }
    }

    /// <summary>
    /// The entries of a list, each with its mark. An entry without anything in it still shows
    /// its mark.
    /// </summary>
    private void Entries(IEnumerable<RichEntry> entries, double indent, int quoted)
    {
        foreach (var entry in entries)
        {
            lead = entry.Mark;
            Blocks(entry.Blocks, indent, quoted);
            Lead(indent);
        }
    }

    private void Paragraph(RichParagraph paragraph, double indent, int quoted)
    {
        if (paragraph.Mark is { } mark)
            lead = mark;
        var heading = paragraph.Heading > 0 ? RunStyle.Bold : RunStyle.None;
        var runs = new List<ShownRun>();
        if (paragraph.Rule)
            runs.Add(new ShownRun(Rule, Quoted(RunStyle.None, quoted), null, Decoration: true));
        foreach (var run in paragraph.Runs)
            runs.Add(
                new ShownRun(run.Text, Quoted(heading | StyleOf(run.Looks), quoted), run.Link)
            );
        Line(runs, paragraph.Scale, indent, (int)paragraph.Start);
    }

    /// <summary>
    /// A table, each cell one paragraph. Its first line stands below the mark of the entry
    /// that it is the first of.
    /// </summary>
    private void Table(IEnumerable<RichTableRow> rows, double indent, int quoted)
    {
        Lead(indent);
        var shown = rows.Select(row => new ShownTableRow(
                row.Header,
                [
                    .. row.Cells.Select(cell => new ShownParagraph(
                        [
                            .. cell.Runs.Select(run => new ShownRun(
                                run.Text,
                                Quoted(
                                    (row.Header ? RunStyle.Bold : RunStyle.None)
                                        | StyleOf(run.Looks),
                                    quoted
                                ),
                                run.Link
                            )),
                        ],
                        1,
                        0,
                        (int)cell.Start
                    )),
                ]
            ))
            .ToList();
        if (shown.Any(row => row.Cells.Count > 0))
            Parts.Add(new ShownTable(shown, indent));
    }

    private void Picture(RichImage image, double indent, int quoted)
    {
        Lead(indent);
        Parts.Add(new ShownPicture(image.Path, image.Alt, image.Width, image.Height, indent));
    }

    /// <summary>The mark of an entry, on a line of its own above a part that is not a line.</summary>
    private void Lead(double indent)
    {
        if (lead is not null)
            Line([], 1, indent, null);
    }

    /// <summary>One line, which starts with the mark of the entry that it is the first of.</summary>
    private void Line(List<ShownRun> runs, double scale, double indent, int? start)
    {
        if (lead is not null)
        {
            runs.Insert(0, new ShownRun(lead + " ", RunStyle.None, null, Decoration: true));
            lead = null;
        }
        if (runs.Count > 0)
            Parts.Add(new ShownParagraph(runs, scale, indent, start));
    }

    private static RunStyle Quoted(RunStyle style, int quoted) =>
        quoted > 0 ? style | RunStyle.Quote : style;

    private static RunStyle StyleOf(Looks looks)
    {
        var style = RunStyle.None;
        if (looks.Bold)
            style |= RunStyle.Bold;
        if (looks.Italic)
            style |= RunStyle.Italic;
        if (looks.Struck)
            style |= RunStyle.Strike;
        if (looks.Underlined)
            style |= RunStyle.Underline;
        if (looks.Marked)
            style |= RunStyle.Highlight;
        if (looks.Code)
            style |= RunStyle.Code;
        if (looks.Raised)
            style |= RunStyle.Superscript;
        if (looks.Lowered)
            style |= RunStyle.Subscript;
        if (looks.Hidden)
            style |= RunStyle.Spoiler;
        return style;
    }
}
