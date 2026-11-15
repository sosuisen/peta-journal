namespace PetaBorder;

/// <summary>
/// The scroll of the list while a drag selection holds the mouse at its top or bottom edge.
/// The rows beyond the edge cannot be reached otherwise. The numbers are those of the web
/// frontend.
/// </summary>
public static class EdgeScroll
{
    /// <summary>The band inside each edge of the list where the mouse scrolls it.</summary>
    private const double Band = 40;

    /// <summary>The scroll of one step when the mouse is at the very edge, or beyond it.</summary>
    private const double Fastest = 24;

    /// <summary>
    /// How far the list scrolls in one step with the mouse at <paramref name="y"/>, for a list
    /// from <paramref name="top"/> to <paramref name="bottom"/>. Negative is up. 0 when the
    /// mouse is not in a band.
    /// </summary>
    public static double Step(double y, double top, double bottom)
    {
        if (y < top + Band)
            return -Depth(top + Band - y);
        if (y > bottom - Band)
            return Depth(y - (bottom - Band));
        return 0;
    }

    /// <summary>The deeper the mouse is in the band, the faster.</summary>
    private static double Depth(double inside) => Math.Min(inside / Band, 1) * Fastest;
}
