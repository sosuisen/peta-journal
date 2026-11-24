import AppKit
import Combine
import PetaBorder
import PetaCore

/// How one sticky looks. The views of the sticky change it, and the desk keeps it.
@MainActor
public final class StickyLook: ObservableObject {
    /// The color of the paper, as `#RRGGBB`.
    @Published public var color: String
    /// The window stays above the windows of other apps.
    @Published public var alwaysOnTop: Bool
    /// How large the page is shown: 1 is the usual size.
    @Published public private(set) var zoom: Double
    /// The search box is on screen. This is not kept: the box is closed in the next run.
    @Published public var searching = false
    /// The sticky is worked on: its window has the focus, and it is not rolled up. Its tools
    /// are shown then. This is not kept.
    @Published public var toolsShown = true

    /// The colors the menu of a sticky offers, as `client-stickies` has them, and then no paper
    /// at all.
    public static let colors: [(name: String, hex: String)] = (stickyColors() + [stickyTransparent()]).map {
        ($0.name, $0.hex)
    }

    public init(color: String = StickySettings.yellow, alwaysOnTop: Bool = false, zoom: Double = 1) {
        self.color = color
        self.alwaysOnTop = alwaysOnTop
        self.zoom = stickyZoom(zoom: zoom, steps: 0)
    }

    public func zoomIn() {
        zoom = stickyZoom(zoom: zoom, steps: 1)
    }

    public func zoomOut() {
        zoom = stickyZoom(zoom: zoom, steps: -1)
    }

    public func resetZoom() {
        zoom = 1
    }

    /// The color that `#RRGGBB` names. Anything else is the yellow of a new sticky.
    public nonisolated static func color(of hex: String) -> NSColor {
        let rgb = stickyRgb(hex: hex)
        return NSColor(
            srgbRed: CGFloat(rgb.red) / 255, green: CGFloat(rgb.green) / 255, blue: CGFloat(rgb.blue) / 255,
            alpha: 1)
    }
}
