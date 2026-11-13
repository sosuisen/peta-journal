import PetaBorder
import PetaTypes
import SwiftUI
import UIKit

/// The Markdown of the item that is edited (ADR 031). The core has the text: this field shows
/// it, and tells the core of every change.
///
/// It is a field of UIKit: it tells where the cursor is when Enter is pressed, and whether
/// the keyboard still composes a word.
struct ItemEditor: UIViewRepresentable {
    let row: Row
    let shell: Shell

    func makeCoordinator() -> Typist {
        Typist(shell: shell)
    }

    func makeUIView(context: Context) -> Field {
        let field = Field()
        field.delegate = context.coordinator
        field.isScrollEnabled = false
        field.backgroundColor = .clear
        field.textContainerInset = .zero
        field.textContainer.lineFragmentPadding = 0
        field.font = .preferredFont(forTextStyle: .body)
        field.adjustsFontForContentSizeCategory = true
        field.accessibilityIdentifier = "editor"
        field.text = row.item.body
        // A finger shows no place in the Markdown: the cursor is at the end, where the
        // writing goes on
        field.selectedRange = NSRange(location: field.text.utf16.count, length: 0)
        // A field that comes back to the screen goes on where the one before it was
        if let left = Typist.left, left.itemId == row.item.id {
            field.selectedRange = NSRange(location: min(left.cursor, field.text.utf16.count), length: 0)
            // It takes the keyboard without a new request of the core: the core asked for
            // the keyboard once, for the field that left
            context.coordinator.taken = false
        }
        Typist.left = nil
        return field
    }

    func updateUIView(_ field: Field, context: Context) {
        let typist = context.coordinator
        let id = row.item.id
        let body = row.item.body
        typist.itemId = id
        // What the core holds wins, as after an undo. Not while the keyboard composes a word:
        // the core has not heard of that word yet, and the field would lose it
        // Nor while the field tells the core of its text: the screen may be drawn in the
        // middle of that, as while the page moves, with the text that the core held before
        if !typist.telling, field.markedTextRange == nil, field.text != body {
            let cursor = min(field.selectedRange.location, body.utf16.count)
            field.text = body
            field.selectedRange = NSRange(location: cursor, length: 0)
            field.invalidateIntrinsicContentSize()
        }
        if let asked = shell.view.outline.focusRequest, asked.itemId == id, asked.seq != typist.answered {
            typist.answered = asked.seq
            typist.taken = false
            if asked.seq != shell.touched, let cursor = asked.cursor {
                field.selectedRange = NSRange(location: units(field.text, characters: Int(cursor)), length: 0)
            }
        }
        // The list drops the field of an item that leaves what is seen, as when the keyboard
        // comes up over it, and makes a new one when the item is back
        guard !typist.taken else { return }
        typist.taken = true
        field.takeTheKeyboard()
        // Not now: the field tells the shell of the text, and the screen is being drawn
        DispatchQueue.main.async { typist.typeWhatWasHeld(into: field) }
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView field: Field, context: Context) -> CGSize? {
        guard let width = proposal.width, width.isFinite, width > 0 else { return nil }
        let fits = field.sizeThatFits(CGSize(width: width, height: .greatestFiniteMagnitude))
        return CGSize(width: width, height: fits.height)
    }

    /// A field that takes the keyboard as soon as it is on the screen.
    final class Field: UITextView {
        private var wanted = false
        /// True when the field left the screen with the keyboard. The user did not end the
        /// edit then.
        fileprivate private(set) var left = false

        func takeTheKeyboard() {
            wanted = true
            if window != nil { becomeFirstResponder() }
        }

        override func didMoveToWindow() {
            super.didMoveToWindow()
            if window != nil, wanted, !isFirstResponder { becomeFirstResponder() }
        }

        override func willMove(toWindow next: UIWindow?) {
            // The field leaves the screen with the keyboard, as after Enter: the field of the
            // new item is not there yet. Nobody would have the keyboard, and what is typed
            // meanwhile would be lost
            if next == nil, isFirstResponder {
                left = true
                let typist = delegate as? Typist
                typist?.leaves(at: selectedRange.location)
                KeyboardKeeper.onScreen?.keep { typist?.nobodyCame() }
            }
            super.willMove(toWindow: next)
        }
    }

    /// Tells the shell what the user does in the field.
    @MainActor
    final class Typist: NSObject, UITextViewDelegate {
        private let shell: Shell
        var itemId = ""
        /// The focus request of the core that the field has followed.
        var answered: UInt64?
        /// True while the field tells the core of its text.
        fileprivate(set) var telling = false
        /// False while the field has still to take the keyboard.
        var taken = true

        /// Where the cursor was in the field that left the screen with the keyboard, and of
        /// which item the field was.
        fileprivate static var left: (itemId: String, cursor: Int)?

        fileprivate func leaves(at cursor: Int) {
            Typist.left = (itemId, cursor)
        }

        /// No field took the keyboard after the field of this item left the screen. The edit
        /// of the item ends, when the core has not ended or moved it by itself.
        fileprivate func nobodyCame() {
            if shell.view.outline.rows.first(where: \.isEditing)?.item.id == itemId { shell.finish(itemId) }
        }

        init(shell: Shell) {
            self.shell = shell
        }

        /// What was typed while the edit moved to another item. After Enter, the core takes
        /// a moment to split the item, and says so (`keyRelayHolding`). Then the screen takes
        /// a moment to give the keyboard to the field of the new item: the old field still
        /// has it, and is no longer of the item that is edited. A letter that went into the
        /// old item would be lost. The field that gets the keyboard next gets the letters.
        fileprivate static var held = ""

        /// True while what is typed is not for the item of this field.
        private var moving: Bool {
            let outline = shell.view.outline
            return outline.keyRelayHolding || outline.rows.first(where: \.isEditing)?.item.id != itemId
        }

        func textView(_ field: UITextView, shouldChangeTextIn range: NSRange, replacementText typed: String) -> Bool {
            if moving {
                Typist.held += typed
                return false
            }
            if !Typist.held.isEmpty {
                // What was held comes first. It moves the cursor, so the place that this
                // change names is no longer right: the letters go in after the others
                Typist.held += typed
                typeWhatWasHeld(into: field)
                return false
            }
            // The Enter that confirms a word of the keyboard does not split the item
            guard typed == "\n", field.markedTextRange == nil else { return true }
            pressEnter(in: field, over: range)
            return false
        }

        /// Types what was held into the field, at its cursor. An Enter among it splits the
        /// item, and what came after it waits for the field of the new item.
        func typeWhatWasHeld(into field: UITextView) {
            let typed = Typist.held
            if typed.isEmpty || moving { return }
            Typist.held = ""
            guard let enter = typed.firstIndex(of: "\n") else { return field.insertText(typed) }
            if enter > typed.startIndex { field.insertText(String(typed[..<enter])) }
            Typist.held = String(typed[typed.index(after: enter)...])
            pressEnter(in: field, over: field.selectedRange)
        }

        private func pressEnter(in field: UITextView, over range: NSRange) {
            var text = field.text ?? ""
            if range.length > 0, let chosen = Range(range, in: text) {
                // Enter over chosen text takes its place: the text goes first
                text.removeSubrange(chosen)
                field.text = text
                shell.type(itemId, text)
            }
            // The core splits the item: the field takes no line break
            shell.enter(itemId, cursor: characters(text, units: range.location))
        }

        func textViewDidBeginEditing(_ field: UITextView) {
            // Not now: the field may get the keyboard while the screen is drawn
            DispatchQueue.main.async { self.typeWhatWasHeld(into: field) }
        }

        func textViewDidChange(_ field: UITextView) {
            field.invalidateIntrinsicContentSize()
            guard field.markedTextRange == nil else { return }
            telling = true
            shell.type(itemId, field.text)
            telling = false
        }

        func textViewDidEndEditing(_ field: UITextView) {
            // A field that left the screen did not end the edit: the keeper has the keyboard
            // for the field that comes next
            if (field as? Field)?.left == true { return }
            let id = itemId
            // The field may lose the keyboard while the screen is drawn, and the shell tells
            // the screen of every change
            DispatchQueue.main.async { [shell] in shell.finish(id) }
        }
    }
}

/// The core counts a cursor in characters, and a field of UIKit counts in UTF-16 units.
private func characters(_ text: String, units: Int) -> Int {
    let utf16 = text.utf16
    let end = utf16.index(utf16.startIndex, offsetBy: min(max(units, 0), utf16.count))
    return String(utf16[..<end])?.unicodeScalars.count ?? text.unicodeScalars.count
}

private func units(_ text: String, characters: Int) -> Int {
    let scalars = text.unicodeScalars
    let end = scalars.index(scalars.startIndex, offsetBy: min(max(characters, 0), scalars.count))
    return String(scalars[..<end]).utf16.count
}

/// Has the keyboard between two fields, and keeps what is typed meanwhile for the field that
/// gets the keyboard next. It is on the screen and cannot be seen.
struct KeyboardKeeping: UIViewRepresentable {
    func makeUIView(context: Context) -> KeyboardKeeper {
        let keeper = KeyboardKeeper()
        KeyboardKeeper.onScreen = keeper
        return keeper
    }

    func updateUIView(_ keeper: KeyboardKeeper, context: Context) {}
}

final class KeyboardKeeper: UITextView, UITextViewDelegate {
    fileprivate static weak var onScreen: KeyboardKeeper?

    /// How long the keeper waits for a field to take the keyboard.
    private static let patience: TimeInterval = 1

    init() {
        super.init(frame: .zero, textContainer: nil)
        delegate = self
        alpha = 0
        isAccessibilityElement = false
        accessibilityElementsHidden = true
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("not made from a file")
    }

    /// What is done when no field took the keyboard in time.
    private var nobodyCame: (() -> Void)?

    fileprivate func keep(nobodyCame: @escaping () -> Void) {
        self.nobodyCame = nobodyCame
        becomeFirstResponder()
        // No field came, as when the item that is edited was scrolled off the screen: the
        // keyboard goes, and what was typed for nobody with it
        DispatchQueue.main.asyncAfter(deadline: .now() + KeyboardKeeper.patience) { [weak self] in
            guard let self, self.isFirstResponder else { return }
            ItemEditor.Typist.held = ""
            self.resignFirstResponder()
            self.nobodyCame?()
            self.nobodyCame = nil
        }
    }

    func textView(_ keeper: UITextView, shouldChangeTextIn range: NSRange, replacementText typed: String) -> Bool {
        ItemEditor.Typist.held += typed
        return false
    }
}
