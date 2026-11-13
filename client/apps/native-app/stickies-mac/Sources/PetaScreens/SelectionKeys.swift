import AppKit
import PetaBorder
import PetaTypes
import SwiftUI

/// Takes the keys while items are selected. No editor has the cursor then, and a key must
/// not change the text of an item.
struct SelectionKeys: NSViewRepresentable {
    let shell: Shell
    /// Read so that the view is asked again when a selection starts.
    let selecting: Bool

    func makeNSView(context: Context) -> SelectionKeysView {
        let view = SelectionKeysView()
        view.shell = shell
        return view
    }

    func updateNSView(_ view: SelectionKeysView, context: Context) {
        guard selecting, let window = view.window, window.firstResponder !== view else { return }
        window.makeFirstResponder(view)
    }
}

final class SelectionKeysView: NSView {
    weak var shell: Shell?

    override var acceptsFirstResponder: Bool { true }

    override func keyDown(with event: NSEvent) {
        guard let shell, shell.view.outline.selectionMode else { return super.keyDown(with: event) }
        // A key that means nothing to a selection does nothing: there is no text to type into
        if let key = SelectionKeysView.key(of: event) { shell.selection(key) }
    }

    /// The keys of a selection. They are those of the web frontend, with Command for the
    /// clipboard and for undo as everywhere on the Mac.
    static func key(of event: NSEvent) -> SelectionKey? {
        let held = event.modifierFlags.intersection([.shift, .control, .option, .command])
        let (x, c, z, tab, backspace, escape, delete): (UInt16, UInt16, UInt16, UInt16, UInt16, UInt16, UInt16) =
            (7, 8, 6, 48, 51, 53, 117)
        let (left, right, down, up): (UInt16, UInt16, UInt16, UInt16) = (123, 124, 125, 126)
        switch (event.keyCode, held) {
        case (escape, []): return .escape
        case (backspace, []), (delete, []): return .delete
        case (up, [.shift]): return .extendUp
        case (down, [.shift]): return .extendDown
        case (up, [.shift, .option]): return .moveUp
        case (down, [.shift, .option]): return .moveDown
        case (left, [.shift, .option]), (tab, [.shift]): return .outdent
        case (right, [.shift, .option]), (tab, []): return .indent
        case (c, [.command]): return .copy
        case (x, [.command]): return .cut
        case (z, [.control]), (z, [.command]): return .undo
        case (z, [.control, .shift]), (z, [.command, .shift]): return .redo
        default: return nil
        }
    }

    /// "Copy", "Cut", "Undo" and "Redo" of the Edit menu. The menu takes their keys before
    /// they come to `keyDown`.
    @objc func copy(_ sender: Any?) {
        shell?.selection(.copy)
    }

    @objc func cut(_ sender: Any?) {
        shell?.selection(.cut)
    }

    @objc func undo(_ sender: Any?) {
        shell?.selection(.undo)
    }

    @objc func redo(_ sender: Any?) {
        shell?.selection(.redo)
    }
}
