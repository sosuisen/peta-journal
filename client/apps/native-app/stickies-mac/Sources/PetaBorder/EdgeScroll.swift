/// The scroll of the list while a drag selection holds the mouse at its top or bottom edge.
/// The rows beyond the edge cannot be reached otherwise. The numbers are those of the web
/// frontend. The places are from the top down.
public enum EdgeScroll {
    /// The band inside each edge of the list where the mouse scrolls it.
    private static let band = 40.0
    /// The scroll of one step when the mouse is at the very edge, or beyond it.
    private static let fastest = 24.0
    /// How long one step takes, in seconds.
    public static let interval = 0.016

    /// How far the list scrolls in one step with the mouse at `y`, for a list from `top` to
    /// `bottom`. Below zero is up. Zero when the mouse is not in a band.
    public static func step(y: Double, top: Double, bottom: Double) -> Double {
        if y < top + band { return -depth(top + band - y) }
        if y > bottom - band { return depth(y - (bottom - band)) }
        return 0
    }

    /// The deeper the mouse is in the band, the faster.
    private static func depth(_ inside: Double) -> Double {
        min(inside / band, 1) * fastest
    }
}
