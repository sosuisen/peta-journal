using PetaTypes;

namespace PetaBorder;

/// <summary>How a row of the outline is drawn, as far as the view model decides it.</summary>
public static class RowLook
{
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
    /// How much larger than the usual text the Markdown in the editor is. It is large only when
    /// the whole body is one heading line. The cursor may go in before the core counts the item
    /// as edited: the scale of the core is then the shown one, and is used only for one heading
    /// line, whose shown first line has the same size (#795).
    /// </summary>
    public static double EditedScale(Row row)
    {
        ArgumentNullException.ThrowIfNull(row);
        return row.IsEditing || row.HeadingLine ? row.FirstLineScale : 1;
    }

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
}
