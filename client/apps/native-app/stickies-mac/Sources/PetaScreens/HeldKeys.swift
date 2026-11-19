import AppKit
import PetaTypes

/// The keys that the core holds while it moves the edit to another item, as after Enter until
/// the item is split. They are played into the editor of the item the edit moves to.
enum HeldKeys {
    /// The keys that are not letters, by the names the core keeps them under (the `key` of a
    /// keyboard event of the web), with their key codes and characters on a Mac.
    private static let named: [(name: String, codes: [UInt16], character: String)] = [
        ("Enter", [36, 76], "\r"),
        ("Tab", [48], "\t"),
        ("Backspace", [51], "\u{7f}"),
        ("Delete", [117], String(UnicodeScalar(NSDeleteFunctionKey)!)),
        ("ArrowLeft", [123], String(UnicodeScalar(NSLeftArrowFunctionKey)!)),
        ("ArrowRight", [124], String(UnicodeScalar(NSRightArrowFunctionKey)!)),
        ("ArrowDown", [125], String(UnicodeScalar(NSDownArrowFunctionKey)!)),
        ("ArrowUp", [126], String(UnicodeScalar(NSUpArrowFunctionKey)!)),
    ]

    /// A letter that is typed.
    static func letter(_ letter: Character) -> DeferredKey {
        DeferredKey(key: String(letter), code: "", shiftKey: false, ctrlKey: false, altKey: false, metaKey: false)
    }

    /// A key of the keyboard that is not a letter, or nil for a letter: letters reach the text
    /// view as text, which may first be composed by the input method.
    static func of(_ event: NSEvent) -> DeferredKey? {
        guard let key = named.first(where: { $0.codes.contains(event.keyCode) }) else { return nil }
        let held = event.modifierFlags
        return DeferredKey(
            key: key.name, code: key.name, shiftKey: held.contains(.shift), ctrlKey: held.contains(.control),
            altKey: held.contains(.option), metaKey: held.contains(.command))
    }

    /// Plays a key into the view at its cursor: a letter as text, another key as if it was
    /// pressed there.
    @MainActor
    static func play(_ key: DeferredKey, into view: NSTextView) {
        guard let named = named.first(where: { $0.name == key.key }) else {
            view.insertText(key.key, replacementRange: view.selectedRange())
            return
        }
        var flags: NSEvent.ModifierFlags = []
        if key.shiftKey { flags.insert(.shift) }
        if key.ctrlKey { flags.insert(.control) }
        if key.altKey { flags.insert(.option) }
        if key.metaKey { flags.insert(.command) }
        guard
            let event = NSEvent.keyEvent(
                with: .keyDown, location: .zero, modifierFlags: flags, timestamp: 0,
                windowNumber: view.window?.windowNumber ?? 0, context: nil, characters: named.character,
                charactersIgnoringModifiers: named.character, isARepeat: false, keyCode: named.codes[0])
        else { return }
        view.keyDown(with: event)
    }
}
