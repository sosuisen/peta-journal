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
        // The cursor is at the end, where the writing goes on, until the core asks for the
        // place that a touch on the text showed
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
        if !typist.telling, !typist.untold, field.markedTextRange == nil, field.text != body {
            let cursor = min(field.selectedRange.location, body.utf16.count)
            field.text = body
            field.selectedRange = NSRange(location: cursor, length: 0)
            field.invalidateIntrinsicContentSize()
        }
        if let asked = shell.view.outline.focusRequest, asked.itemId == id, asked.seq != typist.answered {
            typist.answered = asked.seq
            typist.taken = false
            if asked.seq != shell.touched, let cursor = asked.cursor {
                field.selectedRange = NSRange(location: Cursor.utf16(characters: Int(cursor), in: field.text), length: 0)
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
        /// The field that has the keyboard, if one has.
        fileprivate static weak var holder: Field?
        /// True from the moment the field lost the keyboard until the main queue comes round.
        /// A field that the list takes off the screen loses the keyboard just before it goes
        private var justLost = false

        func takeTheKeyboard() {
            wanted = true
            if window != nil { becomeFirstResponder() }
        }

        override func becomeFirstResponder() -> Bool {
            let got = super.becomeFirstResponder()
            if got { Field.holder = self }
            if got, let typist = delegate as? Typist { typist.placeWhatWasHeld(into: self) }
            return got
        }

        override func resignFirstResponder() -> Bool {
            let had = isFirstResponder
            let gone = super.resignFirstResponder()
            if gone, Field.holder === self { Field.holder = nil }
            if gone, had {
                justLost = true
                DispatchQueue.main.async { [weak self] in self?.justLost = false }
            }
            return gone
        }

        override func deleteBackward() {
            // Backspace at the start changes nothing in the field, so the field tells its
            // delegate of no change: the key itself is heard. Not while text is chosen, or
            // while the keyboard composes a word
            if selectedRange == NSRange(location: 0, length: 0), markedTextRange == nil, let typist = delegate as? Typist {
                typist.backspaceAtStart()
                return
            }
            super.deleteBackward()
        }

        override func didMoveToWindow() {
            super.didMoveToWindow()
            if window != nil, wanted, !isFirstResponder { becomeFirstResponder() }
        }

        override func willMove(toWindow next: UIWindow?) {
            // The field leaves the screen with the keyboard, as after Enter: the field of the
            // new item is not there yet. Nobody would have the keyboard, and what is typed
            // meanwhile would be lost
            // The list also takes the field of the edited item off the screen when the
            // keyboard comes up over the item. The system takes the keyboard from the field
            // first, on a device. The keeper has the keyboard until the item is back in what
            // is seen. Not when the user gave the keyboard to another field, as the search
            let typist = delegate as? Typist
            if next == nil, let typist, typist.edits, isFirstResponder || (justLost && UIResponder.withKeyboard == nil) {
                typist.leaves(at: selectedRange.location)
                KeyboardKeeper.onScreen?.keep { typist.nobodyCame() }
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
        /// True while the field has letters that the core has not heard of yet.
        fileprivate(set) var untold = false
        /// False while the field has still to take the keyboard.
        var taken = true

        /// Where the cursor was in the field that left the screen with the keyboard, and of
        /// which item the field was.
        fileprivate static var left: (itemId: String, cursor: Int)?

        /// Whether an item is edited now, or the core moves the edit to another item.
        fileprivate var edits: Bool {
            let outline = shell.view.outline
            return outline.keyRelayHolding || outline.rows.contains(where: \.isEditing)
        }

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

        /// Backspace at the start of the field. The core joins the item to the one before, and
        /// the edit goes on there. Not while the edit moves to another item: the key is not
        /// for the item of this field then.
        fileprivate func backspaceAtStart() {
            if !moving { shell.backspaceAtStart(itemId) }
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

        /// Puts what was held into the field at once, as the field takes the keyboard, up to an
        /// Enter among it. The keyboard reads the letters before the cursor for the next key,
        /// as for a space after a word in Japanese: an empty field would get a wide space. The
        /// core hears of the letters a moment later, and the rest goes in then.
        fileprivate func placeWhatWasHeld(into field: UITextView) {
            let line = Typist.held.prefix { $0 != "\n" }
            guard !line.isEmpty, !moving else { return }
            Typist.held.removeFirst(line.count)
            KeyboardKeeper.onScreen?.forget()
            let chosen = field.selectedRange
            field.text = ((field.text ?? "") as NSString).replacingCharacters(in: chosen, with: String(line))
            field.selectedRange = NSRange(location: chosen.location + String(line).utf16.count, length: 0)
            field.invalidateIntrinsicContentSize()
            untold = true
            // Not now: the field may take the keyboard while the screen is drawn
            DispatchQueue.main.async { [self] in
                untold = false
                telling = true
                shell.type(itemId, field.text, cursor: field.selectedRange.location)
                telling = false
                typeWhatWasHeld(into: field)
            }
        }

        /// Types what was held into the field, at its cursor. An Enter among it splits the
        /// item, and what came after it waits for the field of the new item.
        func typeWhatWasHeld(into field: UITextView) {
            let typed = Typist.held
            if typed.isEmpty || moving { return }
            Typist.held = ""
            KeyboardKeeper.onScreen?.forget()
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
            shell.enter(itemId, cursor: Cursor.characters(utf16: range.location, in: text))
            // The keeper has the keyboard until the field of the new item takes it. The keyboard
            // may still type into this field without asking its delegate, and the letters
            // would go into the item that is no longer edited
            if moving {
                KeyboardKeeper.onScreen?.keep(
                    while: { [shell] in shell.view.outline.keyRelayHolding },
                    settled: { [weak self, weak field] in
                        // The core did not split the item, as when the server refused: the
                        // edit goes on in this field
                        guard let self, let field, field.window != nil, !self.moving else { return false }
                        field.becomeFirstResponder()
                        self.typeWhatWasHeld(into: field)
                        return true
                    }
                ) { [weak self] in self?.nobodyCame() }
            }
        }

        func textViewDidBeginEditing(_ field: UITextView) {
            // Not now: the field may get the keyboard while the screen is drawn
            DispatchQueue.main.async { self.typeWhatWasHeld(into: field) }
        }

        func textViewDidChange(_ field: UITextView) {
            field.invalidateIntrinsicContentSize()
            guard field.markedTextRange == nil else { return }
            telling = true
            shell.type(itemId, field.text, cursor: field.selectedRange.location)
            telling = false
        }

        func textViewDidEndEditing(_ field: UITextView) {
            let id = itemId
            // Not now: the field may lose the keyboard while the screen is drawn, and the
            // shell tells the screen of every change
            DispatchQueue.main.async { [shell] in
                // The list took the field off the screen, and the edit goes on: the field is
                // back with the keyboard, or the keeper has the keyboard for it
                if Field.holder != nil || KeyboardKeeper.onScreen?.isFirstResponder == true { return }
                shell.finish(id)
            }
        }
    }
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

    /// How often the keeper looks whether the core still moves the edit.
    private static let look: TimeInterval = 0.05

    /// What is done when no field took the keyboard in time.
    private var nobodyCame: (() -> Void)?
    /// What was held for the next field before the keeper took the keyboard.
    private var heldBefore = ""
    /// Which time the keeper took the keyboard. A wait of an earlier time ends without a word:
    /// the keeper waits for another field now.
    private var turn = 0

    /// Takes the keyboard until a field takes it. `waiting` is true while the field may still
    /// come later than `patience`, as while the core splits an item. When the core no longer
    /// moves the edit, `settled` may give the keyboard back to a field, and says whether it did.
    ///
    /// What is typed goes into the keeper, as into a field: the keyboard reads the letters
    /// before the cursor, as for a space after a word in Japanese. The field that gets the
    /// keyboard next gets them.
    fileprivate func keep(
        while waiting: @escaping () -> Bool = { false },
        settled: @escaping () -> Bool = { false },
        nobodyCame: @escaping () -> Void
    ) {
        self.nobodyCame = nobodyCame
        heldBefore = ItemEditor.Typist.held
        text = ""
        turn += 1
        becomeFirstResponder()
        wait(turn, while: waiting, settled: settled)
    }

    /// Lets go of what was typed into the keeper: a field has it now.
    fileprivate func forget() {
        heldBefore = ""
        text = ""
    }

    private func wait(_ turn: Int, while waiting: @escaping () -> Bool, settled: @escaping () -> Bool) {
        guard waiting() else {
            if !settled() { giveUp(turn) }
            return
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + KeyboardKeeper.look) { [weak self] in
            guard let self, self.turn == turn, self.isFirstResponder else { return }
            self.wait(turn, while: waiting, settled: settled)
        }
    }

    /// No field came, as when the item that is edited was scrolled off the screen: the
    /// keyboard goes, and what was typed for nobody with it
    private func giveUp(_ turn: Int) {
        DispatchQueue.main.asyncAfter(deadline: .now() + KeyboardKeeper.patience) { [weak self] in
            guard let self, self.turn == turn, self.isFirstResponder else { return }
            ItemEditor.Typist.held = ""
            self.forget()
            self.resignFirstResponder()
            self.nobodyCame?()
            self.nobodyCame = nil
        }
    }

    func textViewDidChange(_ keeper: UITextView) {
        // Not while the keyboard composes a word: the word is not there yet
        guard keeper.markedTextRange == nil else { return }
        ItemEditor.Typist.held = heldBefore + (keeper.text ?? "")
    }
}

extension UIResponder {
    private static weak var found: UIResponder?

    /// Who has the keyboard now, if anyone has.
    static var withKeyboard: UIResponder? {
        found = nil
        UIApplication.shared.sendAction(#selector(tellOfTheKeyboard), to: nil, from: nil, for: nil)
        return found
    }

    @objc private func tellOfTheKeyboard() {
        UIResponder.found = self
    }
}
