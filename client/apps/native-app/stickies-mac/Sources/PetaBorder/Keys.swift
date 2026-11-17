import AppKit
import Foundation
import PetaCore
import PetaTypes

public enum Keyboard {
    /// The key with the keys held down with it, as the core knows it (ADR 052). What the chord
    /// means is decided in the core.
    public static func chord(_ key: Key, holding held: NSEvent.ModifierFlags = []) -> Chord {
        Chord(
            key: key, shift: held.contains(.shift), ctrl: held.contains(.control),
            alt: held.contains(.option), cmd: held.contains(.command))
    }
}

extension Shell {
    /// The user pressed a key in the editor of an item that the app named itself, as the text
    /// view does for `moveUp:`.
    public func pressed(_ key: EditorKey, in itemId: String, _ editor: EditorState) -> Bool {
        pressed(.named(itemId: itemId, key: key, editor: editor))
    }

    /// The user pressed a key while items are selected, that the app named itself, as the Edit
    /// menu does for Command+C.
    public func selection(_ key: SelectionKey) {
        send(.selectionKey(key))
    }
}
