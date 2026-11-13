import AppKit
import Combine
import PetaBorder

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

    /// The colors the menu of a sticky offers. They are the colors of the cards of Petasti
    /// (sosuisen/petasti, `cardColors`), without its transparent one: a sticky is a paper here.
    public static let colors: [(name: String, hex: String)] = [
        ("Yellow", StickySettings.yellow), ("Red", "#FFCAC4"), ("Green", "#EDFFC4"), ("Blue", "#D1E1FF"),
        ("Orange", "#FFDD9E"), ("Purple", "#F8D0FF"), ("White", "#FFFFFF"), ("Gray", "#E0E0E0"),
        ("Light Gray", "#F8F8F8"),
    ]

    public init(color: String = StickySettings.yellow, alwaysOnTop: Bool = false, zoom: Double = 1) {
        self.color = color
        self.alwaysOnTop = alwaysOnTop
        self.zoom = StickyLook.tenths(zoom)
    }

    public func zoomIn() {
        zoom = StickyLook.tenths(zoom + 0.1)
    }

    public func zoomOut() {
        zoom = StickyLook.tenths(zoom - 0.1)
    }

    public func resetZoom() {
        zoom = 1
    }

    /// The zoom in whole tenths, between three tenths and three, as in the stickies app for
    /// Windows. Whole tenths, as steps of 0.1 do not add up to them.
    private nonisolated static func tenths(_ zoom: Double) -> Double {
        min(30, max(3, (zoom * 10).rounded())) / 10
    }

    /// The color that `#RRGGBB` names. Anything else is the yellow of a new sticky.
    public nonisolated static func color(of hex: String) -> NSColor {
        let digits = hex.dropFirst()
        guard hex.hasPrefix("#"), digits.count == 6, let value = UInt32(digits, radix: 16) else {
            return color(of: StickySettings.yellow)
        }
        return NSColor(
            srgbRed: CGFloat((value >> 16) & 0xFF) / 255, green: CGFloat((value >> 8) & 0xFF) / 255,
            blue: CGFloat(value & 0xFF) / 255, alpha: 1)
    }
}
