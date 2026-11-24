using PetaTypes;

namespace PetaBorder;

/// <summary>How a row of the outline is drawn, as far as the view model decides it.</summary>
public static class RowLook
{
    /// <summary>
    /// How much larger than the usual text a heading of each level is, from the first to the
    /// fifth. The web frontend has the same sizes.
    /// </summary>
    public static IReadOnlyList<double> HeadingScales { get; } = [1.6, 1.4, 1.2, 1.1, 1];

    /// <summary>
    /// The level of the heading when the whole body is one heading line, and 0 otherwise. Such
    /// an item is edited at the size of its heading, in bold: the text does not jump when the
    /// cursor goes in.
    /// </summary>
    public static int EditedHeading(Row row)
    {
        ArgumentNullException.ThrowIfNull(row);
        return row.HeadingLine ? row.HeadingLevel : 0;
    }

    /// <summary>
    /// How much larger than the usual text the Markdown of a heading of this level is. 1 for a
    /// level that is no heading.
    /// </summary>
    public static double HeadingScale(int level) =>
        level is >= 1 and <= 5 ? HeadingScales[level - 1] : 1;

    /// <summary>
    /// The bullet before the text is seen. The core shows none before an empty item that is
    /// not edited, as in the other shells (#1042). A <paramref name="bare"/> sticky shows only
    /// its text.
    /// </summary>
    public static bool ShowsBullet(Row row, bool bare)
    {
        ArgumentNullException.ThrowIfNull(row);
        return row.ShowBullet && !bare;
    }

    /// <summary>
    /// How much larger than the usual text the first line of the item is drawn. The marks
    /// before the text stand in the middle of that line. An item that is edited shows its
    /// Markdown, which is large only when it is one heading line.
    /// </summary>
    public static double FirstLineScale(Row row)
    {
        ArgumentNullException.ThrowIfNull(row);
        return HeadingScale(row.IsEditing ? EditedHeading(row) : row.HeadingLevel);
    }
}
