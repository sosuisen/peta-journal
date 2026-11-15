import AppKit
import PetaBorder

/// A drag that left the text of an item: the items it goes over are selected.
@MainActor
final class ItemDrag {
    private let shell: Shell
    /// The list of the items. The rows of its table are the rows of the outline.
    private weak var scroll: NSScrollView?
    /// The item last told to the shell, so that a move inside one item tells nothing.
    private var reported: String?
    /// Where the mouse is in the window. The rows scroll under it while it stays still.
    private var place = NSPoint.zero

    /// The band inside the top and the bottom of the list where the mouse scrolls it.
    private static let edge: CGFloat = 40
    /// How far one frame scrolls with the mouse at the very edge; less further inside the band.
    private static let fastest: CGFloat = 24
    /// How long a frame of the scroll is, in seconds.
    static let frame: TimeInterval = 0.016

    init(shell: Shell, scroll: NSScrollView?) {
        self.shell = shell
        self.scroll = scroll
    }

    /// The mouse is at the place of the window.
    func moved(to place: NSPoint) {
        self.place = place
        report()
    }

    /// The mouse is in a band at the top or the bottom of the list: frames are to come.
    var scrolls: Bool {
        step != 0
    }

    /// One frame of the scroll, and the item that came under the mouse with it.
    func scrollOneFrame() {
        guard let scroll, let document = scroll.documentView, step != 0 else { return }
        let clip = scroll.contentView
        let end = max(0, document.frame.height - clip.bounds.height)
        let top = min(max(clip.bounds.minY + step, 0), end)
        clip.scroll(to: NSPoint(x: clip.bounds.minX, y: top))
        scroll.reflectScrolledClipView(clip)
        report()
    }

    /// How far a frame scrolls down, or up when it is below zero. Zero outside the bands.
    private var step: CGFloat {
        guard let scroll else { return 0 }
        // In the window, where up is up. The list scrolls down to show what is lower
        let shown = scroll.convert(scroll.bounds, to: nil)
        func speed(_ inside: CGFloat) -> CGFloat { min(inside / Self.edge, 1) * Self.fastest }
        if place.y > shown.maxY - Self.edge { return -speed(place.y - (shown.maxY - Self.edge)) }
        if place.y < shown.minY + Self.edge { return speed(shown.minY + Self.edge - place.y) }
        return 0
    }

    private func report() {
        guard let itemId = item(at: place), itemId != reported else { return }
        reported = itemId
        shell.dragged(over: itemId)
    }

    func ended() {
        shell.dragEnded()
    }

    /// The item under the place of the window, if one is there.
    private func item(at place: NSPoint) -> String? {
        guard let table = scroll.flatMap({ Self.table(in: $0) }) else { return nil }
        let inTable = table.convert(place, from: nil)
        // Only the height counts: beside the text of an item is still its row
        let row = table.row(at: NSPoint(x: table.bounds.midX, y: inTable.y))
        let rows = shell.view.outline.rows
        return rows.indices.contains(row) ? rows[row].item.id : nil
    }

    private static func table(in view: NSView) -> NSTableView? {
        if let table = view as? NSTableView { return table }
        for child in view.subviews {
            if let table = table(in: child) { return table }
        }
        return nil
    }
}
