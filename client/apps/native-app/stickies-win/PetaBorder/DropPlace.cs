namespace PetaBorder;

/// <summary>Where the cursor stands before a character of a text box, in pixels.</summary>
public readonly record struct Caret(double Left, double Top, double Bottom);

/// <summary>
/// The place in a text box where a file was dropped. The text box of WinUI tells where each
/// place is, but not which place is at a point.
/// </summary>
public static class DropPlace
{
    /// <summary>
    /// The place nearest the point (<paramref name="x"/>, <paramref name="y"/>):
    /// <paramref name="carets"/> holds one caret for each place of the text, from 0 to its end.
    /// The line comes first, then the place on it.
    /// </summary>
    public static int Nearest(IReadOnlyList<Caret> carets, double x, double y)
    {
        ArgumentNullException.ThrowIfNull(carets);
        return Enumerable
            .Range(0, carets.Count)
            .MinBy(place => (LineDistance(carets[place], y), Math.Abs(carets[place].Left - x)));
    }

    /// <summary>How far the point is above or below the line of the caret: 0 on the line.</summary>
    private static double LineDistance(Caret caret, double y) =>
        Math.Max(0, Math.Max(caret.Top - y, y - caret.Bottom));
}
