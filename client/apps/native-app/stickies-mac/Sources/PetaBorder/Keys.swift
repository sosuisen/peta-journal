import Foundation
import PetaCore
import PetaTypes

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
