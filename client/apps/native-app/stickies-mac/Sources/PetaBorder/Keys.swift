import Foundation
import PetaCore
import PetaTypes

/// A key that means something to the outline, and not only to the text of an item.
public enum EditorKey {
    case enter, tab, backTab, up, down, left, right, backspace, delete
    /// Makes the item a task, or steps the state of the task.
    case taskCycle
    /// Hides the children of the item, and shows them again.
    case fold, unfold
    /// Moves the item among the items: up and down, and in and out by one level.
    case moveUp, moveDown, moveIn, moveOut
    case undo, redo
    /// Shift with Up or Down. On the first or the last line it selects the item; on any other
    /// line it selects text, as in any editor.
    case selectUp, selectDown
}

/// The editor of an item as a key finds it.
public struct EditorState {
    public var text: String
    /// What is selected, in UTF-16 units as AppKit counts. Empty when there is only a cursor.
    public var selection: Range<Int>
    /// The cursor is on the first, or the last, line of the text as it is drawn.
    public var onFirstLine: Bool
    public var onLastLine: Bool

    public init(text: String, selection: Range<Int>, onFirstLine: Bool, onLastLine: Bool) {
        self.text = text
        self.selection = selection
        self.onFirstLine = onFirstLine
        self.onLastLine = onLastLine
    }
}

extension Shell {
    /// The user pressed a key in the editor of an item. False when the key means nothing to the
    /// outline here, and the editor does with it what it does in any text.
    public func pressed(_ key: EditorKey, in itemId: String, _ editor: EditorState) -> Bool {
        guard let event = event(of: key, in: itemId, editor) else { return false }
        if case .shiftSelect = event {
            // The edit ends first, which saves what was typed. The core does not save it when
            // a selection starts, and a move of the selected item would bring back the text
            // the server has
            send(.editFinished(itemId: itemId))
        }
        send(event)
        return true
    }

    /// The user pressed a key while items are selected. No editor has the cursor then.
    public func selection(_ key: SelectionKey) {
        send(.selectionKey(key))
    }

    /// A drag of the mouse that starts on the text of an item, and may select items.
    public func itemDrag() -> ItemDrag {
        ItemDrag { [unowned self] in send($0) }
    }

    /// The user turned to something else, such as the text of an item: nothing stays selected.
    public func leaveSelection() {
        send(.selectionClickOutside)
    }

    private func event(of key: EditorKey, in itemId: String, _ editor: EditorState) -> Event? {
        // The core counts in characters, and AppKit in UTF-16 units (ADR 043)
        let cursor = charsOfUtf16(text: editor.text, units: UInt64(editor.selection.lowerBound))
        let onlyCursor = editor.selection.isEmpty
        let atStart = onlyCursor && editor.selection.lowerBound == 0
        let atEnd = onlyCursor && editor.selection.lowerBound == editor.text.utf16.count
        switch key {
        case .enter:
            let nowMs = UInt64(Date().timeIntervalSince1970 * 1000)
            return .enterPressed(itemId: itemId, cursor: cursor, nowMs: nowMs)
        case .tab, .moveIn:
            return .indent(itemId: itemId)
        case .backTab, .moveOut:
            return .outdent(itemId: itemId)
        case .up:
            return editor.onFirstLine ? .navigateUp(itemId: itemId, cursor: cursor) : nil
        case .down:
            return editor.onLastLine ? .navigateDown(itemId: itemId, cursor: cursor) : nil
        case .left:
            // A place far past any text: the core puts the cursor at the end of the item before
            return atStart ? .navigateUp(itemId: itemId, cursor: UInt64(Int.max)) : nil
        case .right:
            return atEnd ? .navigateDown(itemId: itemId, cursor: 0) : nil
        case .backspace:
            return atStart ? .backspaceAtStart(itemId: itemId) : nil
        case .delete:
            return atEnd ? .deleteAtEnd(itemId: itemId) : nil
        case .taskCycle:
            return .taskCycle(itemId: itemId)
        case .selectUp:
            return editor.onFirstLine ? .shiftSelect(itemId: itemId) : nil
        case .selectDown:
            return editor.onLastLine ? .shiftSelect(itemId: itemId) : nil
        case .fold, .unfold:
            return .setCollapsed(itemId: itemId, collapsed: key == .fold)
        case .moveUp:
            return .moveUp(itemId: itemId)
        case .moveDown:
            return .moveDown(itemId: itemId)
        case .undo:
            return .undo
        case .redo:
            return .redo
        }
    }
}
