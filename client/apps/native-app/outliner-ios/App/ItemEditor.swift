import PetaBorder
import PetaTypes
import SwiftUI
import UIKit
import UniformTypeIdentifiers
import os

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
            if let cursor = asked.cursor {
                field.selectedRange = NSRange(
                    location: Cursor.utf16(characters: Int(cursor), in: field.text), length: 0)
            }
        }
        // The list drops the field of an item that leaves what is seen, as when the keyboard
        // comes up over it, and makes a new one when the item is back
        guard !typist.taken else { return }
        typist.taken = true
        field.takeTheKeyboard()
    }

    /// The core ordered a field to take the keyboard. The field of the edited item answers when
    /// it has the keyboard already: it takes it no more (`Shell.onKeyboardOrder`).
    @MainActor
    static func keyboardOrdered() {
        guard let field = Field.holder, let typist = field.delegate as? Typist, typist.isEdited else { return }
        typist.tookTheKeyboard(field)
    }

    /// Moves the cursor of the field that is edited, as an arrow key of a desktop does. The
    /// pad of the tools lies outside of the field. Nothing happens while no field has the
    /// keyboard.
    @MainActor
    static func move(_ arrow: Arrow) {
        guard let field = Field.holder, let typist = field.delegate as? Typist else { return }
        typist.move(arrow, in: field)
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

        override init(frame: CGRect, textContainer: NSTextContainer?) {
            super.init(frame: frame, textContainer: textContainer)
            // No sticker of the keyboard in the text: the field would show it, and the Markdown
            // would keep only the character in its place (U+FFFC). The keyboard then pastes it
            // as an image
            if #available(iOS 18.0, *) { supportsAdaptiveImageGlyph = false }
        }

        @available(*, unavailable)
        required init?(coder: NSCoder) {
            fatalError("The field is made in code")
        }

        func takeTheKeyboard() {
            wanted = true
            if window != nil { becomeFirstResponder() }
        }

        override func becomeFirstResponder() -> Bool {
            let got = super.becomeFirstResponder()
            if got { Field.holder = self }
            if got, let typist = delegate as? Typist { typist.tookTheKeyboard(self) }
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
            if selectedRange == NSRange(location: 0, length: 0), markedTextRange == nil,
                let typist = delegate as? Typist
            {
                typist.backspaceAtStart(of: text ?? "")
                return
            }
            super.deleteBackward()
        }

        /// The core pastes, so that a list becomes items and what the app copied comes back as
        /// the same items. An image goes to the core when there is no text. Not while text is
        /// chosen: the field puts the text in its place.
        override func paste(_ sender: Any?) {
            let pasteboard = UIPasteboard.general
            if selectedRange.length == 0, let typist = delegate as? Typist {
                if let text = pasteboard.string {
                    typist.paste(text, at: selectedRange.location)
                    return
                }
                if pasteboard.hasImages {
                    paste(itemProviders: pasteboard.itemProviders)
                    return
                }
            }
            super.paste(sender)
        }

        /// The menu offers "Paste" for an image too. The field alone offers it only for text.
        override func canPerformAction(_ action: Selector, withSender sender: Any?) -> Bool {
            if action == #selector(paste(_:)), selectedRange.length == 0, UIPasteboard.general.hasImages {
                return true
            }
            return super.canPerformAction(action, withSender: sender)
        }

        /// An image of the clipboard goes to the core, which saves it among the assets and links
        /// it at the cursor (ADR 071).
        override func paste(itemProviders: [NSItemProvider]) {
            guard let typist = delegate as? Typist,
                let image = itemProviders.first(where: { $0.hasItemConformingToTypeIdentifier(UTType.image.identifier) }
                )
            else { return super.paste(itemProviders: itemProviders) }
            typist.pasteImage(image, at: selectedRange.location)
        }

        override func didMoveToWindow() {
            super.didMoveToWindow()
            guard window != nil, wanted, !isFirstResponder else { return }
            // Not from a field outside of the items, as the search: the user gave the keyboard
            // to it. The field of the item edited before and the keeper give way
            let holder = UIResponder.withKeyboard
            if holder == nil || holder is Field || holder is KeyboardKeeper { becomeFirstResponder() }
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
            if next == nil, let typist, typist.edits, isFirstResponder || (justLost && UIResponder.withKeyboard == nil)
            {
                typist.leaves(at: selectedRange.location)
                KeyboardKeeper.onScreen?.keep(
                    typed: typist.keeperTyping(field: nil),
                    nobodyCame: { typist.nobodyCame() })
                // The list brings the item back to what is seen, and the field takes the
                // keyboard from the keeper again (#792)
                NotificationCenter.default.post(name: .editedFieldLeft, object: nil)
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

        /// True while the edit moves to another item, until the field of that item takes the
        /// keyboard (ADR 087). After Enter, the core takes a moment to split the item, and the
        /// screen another to give the keyboard to the field of the new item. The old field or
        /// the keeper has the keyboard meanwhile. What is typed goes to the core, which holds it
        /// for the field that takes the keyboard.
        private var holding: Bool { shell.holdsKeys }

        /// Whether the item of this field is the one that is edited.
        fileprivate var isEdited: Bool {
            shell.view.outline.rows.first(where: \.isEditing)?.item.id == itemId
        }

        /// Backspace at the start of the field. The core joins the item to the one before, and
        /// the edit goes on there. Not while the edit moves to another item: the key is not
        /// for the item of this field then.
        fileprivate func paste(_ text: String, at cursor: Int) {
            shell.paste(text, into: itemId, at: cursor)
        }

        /// Why a pasted image did not reach the core. Nothing shows on the screen: the paste
        /// only does nothing, and a test that fails at it once has nothing else to read.
        nonisolated private static let log = Logger(subsystem: "com.sosuisha.petajournal", category: "paste")

        /// Writes the image to a file for the core, away from the main thread: it may be large
        /// and come from another app.
        fileprivate func pasteImage(_ image: NSItemProvider, at cursor: Int) {
            guard
                let type = image.registeredTypeIdentifiers.first(where: { UTType($0)?.conforms(to: .image) == true })
            else {
                Typist.log.error("no image type in \(image.registeredTypeIdentifiers, privacy: .public)")
                return
            }
            let (shell, itemId, name) = (shell, itemId, image.suggestedName)
            _ = image.loadDataRepresentation(forTypeIdentifier: type) { data, error in
                guard let data else {
                    Typist.log.error("the image was not read: \(String(describing: error), privacy: .public)")
                    return
                }
                let file: URL
                do {
                    file = try PastedImage.write(data, named: name, type: type)
                } catch {
                    Typist.log.error("the image was not written: \(error, privacy: .public)")
                    return
                }
                Task { @MainActor in shell.pasteImage(file, into: itemId, at: cursor) }
            }
        }

        fileprivate func backspaceAtStart(of text: String) {
            if holding { return shell.holdBackspace() }
            shell.press(.backspace, in: itemId, .cursor(at: 0, in: text))
        }

        func textView(_ field: UITextView, shouldChangeTextIn range: NSRange, replacementText typed: String) -> Bool {
            if holding {
                if !typed.isEmpty {
                    shell.hold(typed)
                } else if range.length > 0 {
                    shell.holdBackspace()
                }
                return false
            }
            // The Enter that confirms a word of the keyboard does not split the item
            guard typed == "\n", field.markedTextRange == nil else { return true }
            pressEnter(in: field, over: range)
            return false
        }

        /// The field took the keyboard: it answers the orders of the core, and plays the keys
        /// that the core held for it. The letters before the first other key go in at once:
        /// the keyboard reads the letters before the cursor for the next key, as for a space
        /// after a word in Japanese, and an empty field would get a wide space. The core hears
        /// of them a moment later, and the other keys are played then.
        fileprivate func tookTheKeyboard(_ field: UITextView) {
            let keys = shell.keyboardTaken()
            KeyboardKeeper.onScreen?.forget()
            guard !keys.isEmpty else { return }
            let line = keys.prefix(while: Typist.isLetter).map(\.key).joined()
            let rest = Array(keys.drop(while: Typist.isLetter))
            if !line.isEmpty {
                let chosen = field.selectedRange
                field.text = ((field.text ?? "") as NSString).replacingCharacters(in: chosen, with: line)
                field.selectedRange = NSRange(location: chosen.location + line.utf16.count, length: 0)
                field.invalidateIntrinsicContentSize()
                untold = true
            }
            // Not now: the field may take the keyboard while the screen is drawn
            DispatchQueue.main.async { [self] in
                if !line.isEmpty {
                    untold = false
                    telling = true
                    shell.type(itemId, field.text, cursor: field.selectedRange.location)
                    telling = false
                }
                play(rest, into: field)
            }
        }

        /// A key that the core holds as a letter, and not as a key such as Enter.
        private static func isLetter(_ key: DeferredKey) -> Bool {
            key.code.isEmpty
        }

        /// Plays keys that the core held into the field, at its cursor. An Enter among them
        /// splits the item, and the core holds the keys after it again, for the new item.
        private func play(_ keys: [DeferredKey], into field: UITextView) {
            for (index, key) in keys.enumerated() {
                if holding {
                    keys[index...].forEach(shell.hold)
                    return
                }
                switch key.key {
                case "Enter": pressEnter(in: field, over: field.selectedRange)
                case "Backspace": field.deleteBackward()
                default: field.insertText(key.key)
                }
            }
        }

        /// Where the keeper sends what is typed into it, which has the keyboard while no field of
        /// an item has it: `added`, after `removed` characters were taken away. While the edit
        /// moves, the core holds it. This typist goes with its field once its item is no longer
        /// edited, as after Enter, so the core is told without it. Else the edit goes on in
        /// this item (`typedIntoTheKeeper`).
        fileprivate func keeperTyping(field: UITextView?) -> (String, Int) -> Void {
            { [shell, weak self, weak field] added, removed in
                guard !shell.holdsKeys else {
                    for _ in 0..<removed { shell.holdBackspace() }
                    return shell.hold(added)
                }
                self?.typedIntoTheKeeper(added, removing: removed, field: field)
            }
        }

        /// What was typed into the keeper while the edit of this item goes on: in `field` when
        /// it is on the screen, and through the core when the field left the screen with the
        /// keyboard (#792), at the cursor where it left. The field shows the text when it is
        /// back.
        private func typedIntoTheKeeper(_ added: String, removing removed: Int, field: UITextView?) {
            if let field, field.window != nil {
                for _ in 0..<removed { field.deleteBackward() }
                return field.insertText(added)
            }
            guard let left = Typist.left, left.itemId == itemId,
                let body = shell.view.outline.rows.first(where: { $0.item.id == itemId })?.item.body
            else { return }
            let text = body as NSString
            let end = min(left.cursor, text.length)
            var start = end
            for _ in 0..<removed where start > 0 {
                start = text.rangeOfComposedCharacterSequence(at: start - 1).location
            }
            let changed = text.replacingCharacters(in: NSRange(location: start, length: end - start), with: added)
            let cursor = start + added.utf16.count
            Typist.left = (itemId, cursor)
            shell.type(itemId, changed, cursor: cursor)
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
            shell.press(.enter, in: itemId, .cursor(at: range.location, in: text))
            // The keeper has the keyboard until the field of the new item takes it. The keyboard
            // may still type into this field without asking its delegate, and the letters
            // would go into the item that is no longer edited
            if holding {
                KeyboardKeeper.onScreen?.keep(
                    while: { [shell] in shell.holdsKeys && !shell.waitsForTheKeyboard },
                    settled: { [weak self, weak field] in
                        // The edit goes on in this item, as when Enter in an empty last child
                        // outdents it, or when the server refused to split it. The field takes
                        // the keyboard back, and answers the core
                        guard let self, let field, field.window != nil, self.isEdited else { return false }
                        return field.becomeFirstResponder()
                    },
                    typed: keeperTyping(field: field),
                    nobodyCame: { [weak self] in self?.nobodyCame() })
            }
        }

        func textViewDidChange(_ field: UITextView) {
            field.invalidateIntrinsicContentSize()
            // The core hears of a word that the keyboard composes once it is committed
            guard field.markedTextRange == nil else { return shell.composing() }
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
                let edited = shell.view.outline.rows.contains { $0.isEditing && $0.item.id == id }
                shell.finish(id)
                // The keyboard went down, as by a drag of the page: the item stays selected,
                // so that the tools still act on it. Not when another field took the
                // keyboard, as the search field
                if edited, UIResponder.withKeyboard == nil { shell.select(id) }
            }
        }

        /// Moves the cursor of `field` for `arrow`. At the edge of the text the cursor leaves
        /// the item, as the arrow keys of the web frontend do.
        fileprivate func move(_ arrow: Arrow, in field: UITextView) {
            if holding { return }
            if field.markedTextRange != nil {
                // A word that the keyboard composes ends here, and the core has not heard of it
                field.unmarkText()
                telling = true
                shell.type(itemId, field.text, cursor: field.selectedRange.location)
                telling = false
            }
            let chosen = field.selectedRange
            if let place = Caret.moved(arrow, in: field, from: chosen) {
                field.selectedRange = NSRange(location: place, length: 0)
                return
            }
            // The cursor is at the edge of the text: the core moves it to the next item
            let from = arrow == .left || arrow == .up ? chosen.location : NSMaxRange(chosen)
            let key: EditorKey =
                switch arrow {
                case .left: .left
                case .up: .up
                case .right: .right
                case .down: .down
                }
            shell.press(key, in: itemId, .cursor(at: from, in: field.text ?? ""))
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

extension Notification.Name {
    /// The field of the item that is edited left the screen, and the keeper has the keyboard.
    static let editedFieldLeft = Notification.Name("editedFieldLeft")
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
    /// Where what is typed into the keeper goes: the letters added, after the number of
    /// characters taken away.
    private var typed: ((String, Int) -> Void)?
    /// The text of the keeper that `typed` was told of.
    private var told = ""
    /// Which time the keeper took the keyboard. A wait of an earlier time ends without a word:
    /// the keeper waits for another field now.
    private var turn = 0

    /// Takes the keyboard until a field takes it. `waiting` is true while the field may still
    /// come later than `patience`, as while the core splits an item. When the core no longer
    /// moves the edit, `settled` may give the keyboard back to a field, and says whether it did.
    ///
    /// What is typed goes into the keeper, as into a field: the keyboard reads the letters
    /// before the cursor, as for a space after a word in Japanese. The keeper keeps none of it:
    /// `typed` hears of each change.
    fileprivate func keep(
        while waiting: @escaping () -> Bool = { false },
        settled: @escaping () -> Bool = { false },
        typed: @escaping (String, Int) -> Void,
        nobodyCame: @escaping () -> Void
    ) {
        self.nobodyCame = nobodyCame
        self.typed = typed
        forget()
        turn += 1
        becomeFirstResponder()
        wait(turn, while: waiting, settled: settled)
    }

    /// Lets go of the letters that the keyboard reads before the cursor: a field has the
    /// keyboard now.
    fileprivate func forget() {
        told = ""
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
    /// keyboard goes. The end of the edit lets go of what the core held for nobody
    private func giveUp(_ turn: Int) {
        DispatchQueue.main.asyncAfter(deadline: .now() + KeyboardKeeper.patience) { [weak self] in
            guard let self, self.turn == turn, self.isFirstResponder else { return }
            self.forget()
            self.resignFirstResponder()
            self.nobodyCame?()
            self.nobodyCame = nil
        }
    }

    func textViewDidChange(_ keeper: UITextView) {
        // Not while the keyboard composes a word: the word is not there yet
        guard keeper.markedTextRange == nil else { return }
        let now = keeper.text ?? ""
        let same = zip(told, now).prefix { $0 == $1 }.count
        let removed = told.count - same
        let added = String(now.dropFirst(same))
        told = now
        if removed > 0 || !added.isEmpty { typed?(added, removed) }
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

extension EditorState {
    /// A field with a cursor `at` UTF-16 units into `text`, and nothing chosen. The cursor is
    /// on the first and the last line: the field only hands a key to the core at the edge of
    /// its text.
    static func cursor(at: Int, in text: String) -> EditorState {
        EditorState(
            text: text, selectionStart: UInt64(at), selectionEnd: UInt64(at), onFirstLine: true,
            onLastLine: true)
    }
}

/// Where the cursor of a field goes for an arrow.
enum Caret {
    /// Where the cursor goes in `field` for `arrow`, from `chosen`, in UTF-16 units. `nil`
    /// when it leaves the item: at the start for Left, on the first line for Up, at the end
    /// for Right, and on the last line for Down. Chosen text gives way to a cursor at its start
    /// or its end. Left and Right step over a whole character, as an emoji.
    @MainActor
    static func moved(_ arrow: Arrow, in field: UITextView, from chosen: NSRange) -> Int? {
        let text = (field.text ?? "") as NSString
        if chosen.length > 0, arrow == .left || arrow == .right {
            return arrow == .left ? chosen.location : NSMaxRange(chosen)
        }
        let at = arrow == .left || arrow == .up ? chosen.location : NSMaxRange(chosen)
        switch arrow {
        case .left: return at == 0 ? nil : text.rangeOfComposedCharacterSequence(at: at - 1).location
        case .right: return at >= text.length ? nil : NSMaxRange(text.rangeOfComposedCharacterSequence(at: at))
        case .up: return onLine(field, from: at, up: true)
        case .down: return onLine(field, from: at, up: false)
        }
    }

    /// The place on the line above or below the line of `at`, under the same spot of the
    /// screen. `nil` on the first line for up, and on the last for down.
    @MainActor
    private static func onLine(_ field: UITextView, from at: Int, up: Bool) -> Int? {
        let start = field.beginningOfDocument
        guard let here = field.position(from: start, offset: at) else { return nil }
        let caret = field.caretRect(for: here)
        let edge = field.caretRect(for: up ? start : field.endOfDocument)
        if abs(caret.midY - edge.midY) < caret.height / 2 { return nil }
        let spot = CGPoint(x: caret.midX, y: caret.midY + (up ? -caret.height : caret.height))
        guard let found = field.closestPosition(to: spot) else { return nil }
        return field.offset(from: start, to: found)
    }
}
