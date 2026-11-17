import AppKit
import PetaBorder

/// A drag that starts on the text of an item, in the list of the items. It tells the core where
/// the mouse is, and scrolls the list as `EdgeScroll` says. Both count from the top of the part
/// of the list that can be seen, down; AppKit counts in the window, up.
@MainActor
final class ListDrag {
    private let shell: Shell
    /// The list of the items. The rows of its table are the rows of the outline.
    private weak var scroll: NSScrollView?
    /// Where the mouse is in the window. The rows scroll under it while it stays still.
    private var place = NSPoint.zero

    /// The mouse went down on `text`, the text of the item.
    init(shell: Shell, itemId: String, text: NSView) {
        self.shell = shell
        scroll = text.enclosingScrollView
        let box = text.convert(text.bounds, to: nil)
        shell.dragged(.pressed(itemId: itemId, top: y(of: box.maxY), bottom: y(of: box.minY)))
    }

    /// Items are selected: the drag left the text.
    var selects: Bool {
        shell.view.outline.selectionMode
    }

    /// The mouse is at the place of the window.
    func moved(to place: NSPoint) {
        self.place = place
        tell()
    }

    /// Items are selected, and the mouse is in a band at the top or the bottom of the list:
    /// steps of the scroll are to come.
    var scrolls: Bool {
        selects && step != 0
    }

    /// One step of the scroll. The item that comes under the mouse joins the selection: the
    /// mouse does not move, so no move tells it.
    func scrollOneStep() {
        guard let scroll, let document = scroll.documentView, scrolls else { return }
        let clip = scroll.contentView
        let end = max(0, document.frame.height - clip.bounds.height)
        let top = min(max(clip.bounds.minY + step, 0), end)
        clip.scroll(to: NSPoint(x: clip.bounds.minX, y: top))
        scroll.reflectScrolledClipView(clip)
        tell()
    }

    func released() {
        shell.dragged(.released)
    }

    private var step: CGFloat {
        EdgeScroll.step(y: y(of: place.y), top: 0, bottom: shown.height)
    }

    private func tell() {
        shell.dragged(.moved(y: y(of: place.y), itemUnder: item(at: place)))
    }

    /// The part of the list that can be seen, in the window.
    private var shown: NSRect {
        scroll.map { $0.convert($0.bounds, to: nil) } ?? .zero
    }

    /// How far below the top of what can be seen of the list a height of the window is.
    private func y(of height: CGFloat) -> Double {
        shown.maxY - height
    }

    /// The height of the window that is `y` below the top of what can be seen of the list.
    private func height(of y: Double) -> CGFloat {
        shown.maxY - y
    }

    /// The item under the place of the window, if one is there.
    private func item(at place: NSPoint) -> String? {
        guard let table = scroll.flatMap({ Self.table(in: $0) }) else { return nil }
        let y = EdgeScroll.placeInList(y: y(of: place.y), top: 0, bottom: shown.height)
        let inTable = table.convert(NSPoint(x: place.x, y: height(of: y)), from: nil)
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
