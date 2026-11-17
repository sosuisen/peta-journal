import PetaCore

/// The scroll of the list while a drag selection holds the mouse at its top or bottom edge.
/// The rows beyond the edge cannot be reached otherwise. The rules are the core's, the same in
/// every shell. The places are from the top down.
public enum EdgeScroll {
    /// How long one step takes, in seconds.
    public static let interval = Double(edgeStepMs()) / 1000

    /// How far the list scrolls in one step with the mouse at `y`, for a list from `top` to
    /// `bottom`. Below zero is up. Zero when the mouse is not in a band.
    public static func step(y: Double, top: Double, bottom: Double) -> Double {
        edgeStep(y: y, top: top, bottom: bottom)
    }

    /// Where to look for the item under the mouse at `y`, in a list that can be seen from `top`
    /// to `bottom`. A place beyond it counts as the nearest place in it.
    public static func placeInList(y: Double, top: Double, bottom: Double) -> Double {
        coreSaysPlace(y: y, top: top, bottom: bottom)
    }
}

/// The place of the core: the module's name is also the name of a class in it, so the method
/// above cannot name the function of the core directly.
private func coreSaysPlace(y: Double, top: Double, bottom: Double) -> Double {
    placeInList(y: y, top: top, bottom: bottom)
}
