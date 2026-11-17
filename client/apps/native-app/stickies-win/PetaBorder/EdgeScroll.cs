namespace PetaBorder;

/// <summary>
/// The scroll of the list while a drag selection holds the mouse at its top or bottom edge.
/// The rows beyond the edge cannot be reached otherwise. The rules are the core's, the same in
/// every shell.
/// </summary>
public static class EdgeScroll
{
    /// <summary>How long one step takes.</summary>
    public static TimeSpan Interval { get; } =
        TimeSpan.FromMilliseconds(PetaCore.PetaNative.EdgeStepMs());

    /// <summary>
    /// How far the list scrolls in one step with the mouse at <paramref name="y"/>, for a list
    /// from <paramref name="top"/> to <paramref name="bottom"/>. Negative is up. 0 when the
    /// mouse is not in a band.
    /// </summary>
    public static double Step(double y, double top, double bottom) =>
        PetaCore.PetaNative.EdgeStep(y, top, bottom);

    /// <summary>
    /// Where to look for the item under the mouse at <paramref name="y"/>, in a list that can
    /// be seen from <paramref name="top"/> to <paramref name="bottom"/>. A place beyond it
    /// counts as the nearest place in it: the mouse of a drag may leave the window.
    /// </summary>
    public static double PlaceInList(double y, double top, double bottom) =>
        PetaCore.PetaNative.PlaceInList(y, top, bottom);
}
