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
    private var clicks: Any?

    override var acceptsFirstResponder: Bool { true }

    deinit {
        if let clicks { NSEvent.removeMonitor(clicks) }
    }

    // The clicks of the title bar and of the buttons do not come to this view. The window
    // shows them all to a monitor first
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if let clicks { NSEvent.removeMonitor(clicks) }
        clicks = nil
        guard window != nil else { return }
        clicks = NSEvent.addLocalMonitorForEvents(matching: .leftMouseDown) { [weak self] event in
            self?.pressed(event)
            return event
        }
    }

    /// A press on anything but an item ends a selection, as the web frontend does. A press on
    /// an item is left to the item: it starts an edit or a drag, and the core ends the
    /// selection then.
    private func pressed(_ event: NSEvent) {
        guard let shell, let window, event.window === window, shell.view.outline.selectionMode else {
            return
        }
        if !isOnItem(event, in: window) { shell.clickedOutsideItems() }
    }

    private func isOnItem(_ event: NSEvent, in window: NSWindow) -> Bool {
        guard let content = window.contentView, let frame = content.superview else { return false }
        // `hitTest` takes the place in the view around the content
        var hit = content.hitTest(frame.convert(event.locationInWindow, from: nil))
        while let view = hit {
            if view is ItemTextView { return true }
            hit = view.superview
        }
        return false
    }

    override func keyDown(with event: NSEvent) {
        guard let shell, shell.view.outline.selectionMode else { return super.keyDown(with: event) }
        // A key that means nothing to a selection does nothing: there is no text to type into
        if let chord = Chord.of(event) { _ = shell.pressed(.inSelection(chord: chord)) }
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

extension Chord {
    /// The key of an event with the keys held down with it, as the core knows it (ADR 052).
    /// Nil for a key that the core does not know.
    static func of(_ event: NSEvent) -> Chord? {
        guard let key = Key.of(event) else { return nil }
        return Keyboard.chord(key, holding: event.modifierFlags)
    }
}

extension Key {
    static func of(_ event: NSEvent) -> Key? {
        switch event.keyCode {
        case 36, 76: return .enter
        case 48: return .tab
        case 126: return .up
        case 125: return .down
        case 123: return .left
        case 124: return .right
        case 51: return .backspace
        case 117: return .delete
        case 53: return .escape
        default:
            // The letter of the layout the user types with, as Command+Z follows it
            guard let typed = event.charactersIgnoringModifiers?.lowercased(), typed.count == 1,
                typed.first?.isLetter == true
            else { return nil }
            return .letter(typed)
        }
    }
}
