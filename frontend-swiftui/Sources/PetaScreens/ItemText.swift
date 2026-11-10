import AppKit
import PetaBorder
import PetaTypes
import SwiftUI

/// The text of one item, in a text view of AppKit. A text field of SwiftUI does not tell where
/// its cursor is, and the keys of the outline depend on that: Enter splits the item at the
/// cursor, and an arrow leaves the item only from its first or last line.
struct ItemText: NSViewRepresentable {
    let item: Item
    /// The first item of the page: the cursor goes here when the core names no item.
    let isFirst: Bool
    let request: FocusRequest?
    let order: FocusOrder?
    let shell: Shell
    let cursors: Cursors

    func makeCoordinator() -> Editor {
        Editor(shell: shell, itemId: item.id, cursors: cursors)
    }

    func makeNSView(context: Context) -> ItemTextView {
        let view = ItemTextView.make()
        view.delegate = context.coordinator
        view.string = item.body
        view.onCursor = { [editor = context.coordinator] in editor.gotCursor() }
        view.onKey = { [editor = context.coordinator] key, view in editor.pressed(key, in: view) }
        return view
    }

    func updateNSView(_ view: ItemTextView, context: Context) {
        // Text that is still being composed is not in the core yet, and must not be lost to
        // what the core has
        if !view.hasMarkedText(), view.string != item.body {
            let cursor = min(view.selectedRange().location, (item.body as NSString).length)
            view.string = item.body
            view.setSelectedRange(NSRange(location: cursor, length: 0))
        }
        context.coordinator.putCursor(in: view, request: request, order: order, isFirst: isFirst)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, nsView: ItemTextView, context: Context) -> CGSize? {
        let width = proposal.width ?? 400
        return CGSize(width: width, height: nsView.height(forWidth: width))
    }
}

/// Which orders to put the cursor somewhere were already carried out. It is one for the whole
/// screen, not one for each editor: the list makes the editor of an item again when the item
/// scrolls back into view, and that new editor must not take the cursor for an old order.
@MainActor
final class Cursors: ObservableObject {
    var request: UInt64 = 0
    var order = 0
}

/// Hears one text view and tells the shell.
@MainActor
final class Editor: NSObject, NSTextViewDelegate {
    private let shell: Shell
    private let itemId: String
    private let cursors: Cursors

    init(shell: Shell, itemId: String, cursors: Cursors) {
        self.shell = shell
        self.itemId = itemId
        self.cursors = cursors
    }

    private static let keys: [Selector: EditorKey] = [
        #selector(NSResponder.insertNewline(_:)): .enter,
        #selector(NSResponder.insertTab(_:)): .tab,
        #selector(NSResponder.insertBacktab(_:)): .backTab,
        #selector(NSResponder.moveUp(_:)): .up,
        #selector(NSResponder.moveDown(_:)): .down,
        #selector(NSResponder.moveLeft(_:)): .left,
        #selector(NSResponder.moveRight(_:)): .right,
        #selector(NSResponder.deleteBackward(_:)): .backspace,
        #selector(NSResponder.deleteForward(_:)): .delete,
    ]

    func textDidChange(_ notification: Notification) {
        guard let view = notification.object as? ItemTextView, !view.hasMarkedText() else { return }
        shell.typed(view.string, into: itemId)
    }

    /// AppKit asks before it carries out a key. While text is composed the keys go to the input
    /// method and never come here, so the Enter that ends a composition splits nothing.
    func textView(_ view: NSTextView, doCommandBy selector: Selector) -> Bool {
        guard let key = Editor.keys[selector], let view = view as? ItemTextView else { return false }
        return pressed(key, in: view)
    }

    /// Tells the shell a key, with the text and the cursor as they are now.
    func pressed(_ key: EditorKey, in view: ItemTextView) -> Bool {
        let selected = view.selectedRange()
        let (first, last) = view.cursorLines()
        let state = EditorState(
            text: view.string, selection: selected.location..<(selected.location + selected.length),
            onFirstLine: first, onLastLine: last)
        return shell.pressed(key, in: itemId, state)
    }

    /// The text view got the cursor, by a click or by an order of the core.
    func gotCursor() {
        guard shell.view.outline.focusedItemId != itemId else { return }
        shell.startEditing(itemId)
        // The core answers with an order to put the cursor at the start of this item. The
        // cursor is already here, and where the user clicked
        if let request = shell.view.outline.focusRequest, request.itemId == itemId {
            cursors.request = request.seq
        }
    }

    /// Carries out an order of the core that is new and names this item.
    func putCursor(
        in view: ItemTextView, request: FocusRequest?, order: FocusOrder?, isFirst: Bool, tries: Int = 10
    ) {
        let asked = request.flatMap { $0.itemId == itemId && $0.seq != cursors.request ? $0 : nil }
        let ordered = order.flatMap {
            $0.number != cursors.order && ($0.itemId == itemId || ($0.itemId == nil && isFirst)) ? $0 : nil
        }
        guard asked != nil || ordered != nil else { return }
        guard let window = view.window else {
            // The editor of a new item is not in its window yet when the order comes with it
            guard tries > 0 else { return }
            DispatchQueue.main.async { [self] in
                putCursor(in: view, request: request, order: order, isFirst: isFirst, tries: tries - 1)
            }
            return
        }
        if let asked { cursors.request = asked.seq }
        if let ordered { cursors.order = ordered.number }
        window.makeFirstResponder(view)
        if let cursor = asked?.cursor {
            let place = Cursor.utf16(characters: cursor, in: view.string)
            view.setSelectedRange(NSRange(location: place, length: 0))
        }
    }
}

/// A plain text view that is as high as its text.
final class ItemTextView: NSTextView {
    var onCursor: (() -> Void)?
    /// A key of the outline that is no command of a text view. True when it was taken.
    var onKey: ((EditorKey, ItemTextView) -> Bool)?
    /// The text view does not keep its text storage alive when it is built by hand.
    private var storage: NSTextStorage?

    static func make() -> ItemTextView {
        // Built by hand to get TextKit 1, whose layout manager tells the lines of the text
        let storage = NSTextStorage()
        let layout = NSLayoutManager()
        storage.addLayoutManager(layout)
        let container = NSTextContainer(size: NSSize(width: 400, height: CGFloat.greatestFiniteMagnitude))
        container.widthTracksTextView = true
        container.lineFragmentPadding = 0
        layout.addTextContainer(container)
        let view = ItemTextView(frame: NSRect(x: 0, y: 0, width: 400, height: 17), textContainer: container)
        view.storage = storage
        view.font = .systemFont(ofSize: NSFont.systemFontSize)
        view.textColor = .labelColor
        view.drawsBackground = false
        view.isRichText = false
        view.textContainerInset = .zero
        // The core keeps the history of the outline, not the text view
        view.allowsUndo = false
        view.isAutomaticQuoteSubstitutionEnabled = false
        view.isAutomaticDashSubstitutionEnabled = false
        view.isAutomaticTextReplacementEnabled = false
        view.isAutomaticSpellingCorrectionEnabled = false
        return view
    }

    override func becomeFirstResponder() -> Bool {
        let became = super.becomeFirstResponder()
        if became { onCursor?() }
        return became
    }

    /// The keys with Control, Option or Command that mean something to the outline. AppKit has
    /// no command for them, or one that means something else.
    override func keyDown(with event: NSEvent) {
        // While text is composed every key belongs to the input method
        if !hasMarkedText(), take(event) { return }
        super.keyDown(with: event)
    }

    private func take(_ event: NSEvent) -> Bool {
        let held = event.modifierFlags.intersection([.shift, .control, .option, .command])
        let (returnKey, left, right, down, up, z): (UInt16, UInt16, UInt16, UInt16, UInt16, UInt16) =
            (36, 123, 124, 125, 126, 6)
        switch (event.keyCode, held) {
        case (returnKey, [.shift]):
            // A line break inside the item. Enter alone splits the item
            insertText("\n", replacementRange: selectedRange())
            return true
        case (returnKey, [.control]): return onKey?(.taskCycle, self) ?? false
        // Taken also when there is nothing to fold: otherwise AppKit would put the cursor at
        // the start or the end of the text
        case (up, [.command]):
            _ = onKey?(.fold, self)
            return true
        case (down, [.command]):
            _ = onKey?(.unfold, self)
            return true
        case (up, [.shift, .option]): return onKey?(.moveUp, self) ?? false
        case (down, [.shift, .option]): return onKey?(.moveDown, self) ?? false
        case (left, [.shift, .option]): return onKey?(.moveOut, self) ?? false
        case (right, [.shift, .option]): return onKey?(.moveIn, self) ?? false
        case (z, [.control]): return onKey?(.undo, self) ?? false
        case (z, [.control, .shift]): return onKey?(.redo, self) ?? false
        default: return false
        }
    }

    /// "Undo" and "Redo" of the Edit menu, with their keys Command+Z and Shift+Command+Z. The
    /// history is the one of the core: the text view keeps none.
    @objc func undo(_ sender: Any?) {
        _ = onKey?(.undo, self)
    }

    @objc func redo(_ sender: Any?) {
        _ = onKey?(.redo, self)
    }

    override func validateUserInterfaceItem(_ item: NSValidatedUserInterfaceItem) -> Bool {
        if item.action == #selector(undo(_:)) || item.action == #selector(redo(_:)) { return true }
        return super.validateUserInterfaceItem(item)
    }

    func height(forWidth width: CGFloat) -> CGFloat {
        guard let container = textContainer, let layout = layoutManager else { return 17 }
        container.containerSize = NSSize(width: width, height: CGFloat.greatestFiniteMagnitude)
        layout.ensureLayout(for: container)
        let line = layout.defaultLineHeight(for: font ?? .systemFont(ofSize: NSFont.systemFontSize))
        return max(layout.usedRect(for: container).height, line)
    }

    /// Whether the cursor is on the first line, and on the last line, of the text as drawn.
    func cursorLines() -> (first: Bool, last: Bool) {
        guard let container = textContainer, let layout = layoutManager else { return (true, true) }
        layout.ensureLayout(for: container)
        let length = (string as NSString).length
        let glyphs = layout.numberOfGlyphs
        guard glyphs > 0 else { return (true, true) }
        func top(ofLineAt place: Int) -> CGFloat {
            if place >= length {
                // After a line break at the end there is a line with no glyph in it
                let extra = layout.extraLineFragmentRect
                if extra.height > 0 { return extra.minY }
                return layout.lineFragmentRect(forGlyphAt: glyphs - 1, effectiveRange: nil).minY
            }
            let glyph = layout.glyphIndexForCharacter(at: place)
            return layout.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil).minY
        }
        let mine = top(ofLineAt: selectedRange().location)
        return (mine == top(ofLineAt: 0), mine == top(ofLineAt: length))
    }
}
