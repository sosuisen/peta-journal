import CoreGraphics

/// An arrow of a pad, as an arrow key of a desktop.
enum Arrow: CaseIterable {
    case left, up, down, right
}

/// The pads of four arrows at the bottom of the screen.
enum Pad {
    /// How far the middle of a pad reaches from its centre, in halves of the pad.
    private static let middle: CGFloat = 1.0 / 3

    /// The part of a pad of `size` that a touch at `point` falls on. The lines from its corners
    /// through its middle cut it into four parts, one for each arrow. `nil` in the middle, a
    /// third of the pad on each side, where no arrow is meant.
    static func aim(at point: CGPoint, in size: CGSize) -> Arrow? {
        // Measured in halves of the pad, so that the cuts go through its corners
        let across = (point.x - size.width / 2) / (size.width / 2)
        let down = (point.y - size.height / 2) / (size.height / 2)
        if abs(across) < middle, abs(down) < middle { return nil }
        if abs(across) >= abs(down) { return across < 0 ? .left : .right }
        return down < 0 ? .up : .down
    }
}
