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

    init(shell: Shell, scroll: NSScrollView?) {
        self.shell = shell
        self.scroll = scroll
    }

    /// The mouse is at the place of the window.
    func moved(to place: NSPoint) {
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
