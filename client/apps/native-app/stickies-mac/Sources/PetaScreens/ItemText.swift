import AppKit
import PetaBorder
import PetaCore
import PetaTypes
import SwiftUI

/// The text of one item, in a text view of AppKit. A text field of SwiftUI does not tell where
/// its cursor is, and the keys of the outline depend on that: Enter splits the item at the
/// cursor, and an arrow leaves the item only from its first or last line.
///
/// The item that is edited shows its Markdown. Every other item shows its text with its
/// decoration (ADR 041), and cannot be typed into. The core says which item is edited.
struct ItemText: NSViewRepresentable {
    let item: Item
    /// The AST of the body of the item, that the text with its decoration is drawn from.
    let rich: RichText
    /// The core counts the item as edited: it has, or just had, the cursor.
    let isEditing: Bool
    /// The text of the item with its decoration, when it was made already. It is handed over,
    /// and not looked up here: SwiftUI draws a row again only when what it is given changes.
    let shown: NSAttributedString?
    let texts: ShownTexts
    /// What `texts` counts. It is here only to have the row drawn again when it goes up.
    let changes: Int
    /// The level of the heading when the item is one heading line, and 0 otherwise.
    var heading = 0
    let request: FocusRequest?
    let order: FocusOrder?
    let shell: Shell
    let cursors: Cursors
    /// How large the text is shown: 1 is the usual size.
    var zoom: Double = 1

    func makeCoordinator() -> Editor {
        Editor(shell: shell, itemId: item.id, cursors: cursors, texts: texts)
    }

    func makeNSView(context: Context) -> ItemTextView {
        let view = ItemTextView.make()
        view.delegate = context.coordinator
        view.string = item.body
        view.onCursor = { [editor = context.coordinator, weak view] in
            if let view { editor.gotCursor(in: view) }
        }
        view.onCursorLeft = { [editor = context.coordinator] in editor.lostCursor() }
        view.onLink = { [shell] link, held in shell.follow(link, holding: held) }
        view.onKey = { [editor = context.coordinator] key, view in editor.pressed(key, in: view) }
        view.onChord = { [editor = context.coordinator] chord, view in editor.pressed(chord, in: view) }
        view.onPress = { [editor = context.coordinator] view in editor.dragStarts(on: view) }
        view.onPaste = { [editor = context.coordinator] raw, view in editor.paste(raw: raw, in: view) }
        view.onTypedElsewhere = { [editor = context.coordinator] text in editor.typedElsewhere(text) }
        view.onKeyElsewhere = { [editor = context.coordinator] key in editor.pressedElsewhere(key) }
        return view
    }

    func updateNSView(_ view: ItemTextView, context: Context) {
        let font = ItemTextView.markdownFont(heading: heading, zoom: zoom)
        context.coordinator.body = item.body
        context.coordinator.font = font
        // The view with the cursor is Markdown at once, before the core counts its item as
        // edited: what is typed right after a click must not be lost
        if isEditing || view.window?.firstResponder === view {
            edit(view, font: font)
        } else {
            show(view)
        }
        context.coordinator.putCursor(in: view, request: request, order: order)
    }

    /// The Markdown of the item, to type into.
    private func edit(_ view: ItemTextView, font: NSFont) {
        guard view.isMarkdown else {
            view.showMarkdown(item.body, font: font)
            return
        }
        // Text that is still being composed is not in the core yet, and must not be lost to
        // what the core has
        if !view.hasMarkedText(), view.string != item.body {
            let cursor = min(view.selectedRange().location, (item.body as NSString).length)
            view.string = item.body
            view.setSelectedRange(NSRange(location: cursor, length: 0))
        }
        if view.font != font {
            view.font = font
            // What is typed next has the font too, also in a text that is still empty
            view.typingAttributes[.font] = font
            // The text is as high as its lines, which are of another height now
            view.invalidateIntrinsicContentSize()
        }
    }

    /// The text of the item with its decoration. Until it is made, the item shows its body.
    private func show(_ view: ItemTextView) {
        view.isEditable = false
        guard let text = shown else {
            texts.want(item.id, rich: rich, zoom: zoom)
            if view.shown == nil, view.string != item.body { view.string = item.body }
            return
        }
        guard view.shown !== text else { return }
        view.shown = text
        view.textStorage?.setAttributedString(text)
        view.invalidateIntrinsicContentSize()
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
    var order: UInt64 = 0
}

/// Hears one text view and tells the shell.
@MainActor
final class Editor: NSObject, NSTextViewDelegate {
    private let shell: Shell
    private let itemId: String
    private let cursors: Cursors
    private let texts: ShownTexts
    /// The Markdown of the item and the font of its letters, as the screen last drew them.
    var body = ""
    var font = NSFont.systemFont(ofSize: NSFont.systemFontSize)

    init(shell: Shell, itemId: String, cursors: Cursors, texts: ShownTexts) {
        self.shell = shell
        self.itemId = itemId
        self.cursors = cursors
        self.texts = texts
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
        #selector(NSResponder.moveUpAndModifySelection(_:)): .selectUp,
        #selector(NSResponder.moveDownAndModifySelection(_:)): .selectDown,
    ]

    func textDidChange(_ notification: Notification) {
        guard let view = notification.object as? ItemTextView, !view.hasMarkedText() else { return }
        shell.typed(view.string, into: itemId, cursor: view.selectedRange().location)
    }

    /// AppKit asks before it carries out a key. While text is composed the keys go to the input
    /// method and never come here, so the Enter that ends a composition splits nothing.
    func textView(_ view: NSTextView, doCommandBy selector: Selector) -> Bool {
        guard let key = Editor.keys[selector], let view = view as? ItemTextView else { return false }
        return pressed(key, in: view)
    }

    /// Tells the shell a key that the text view named, with the text and the cursor as they
    /// are now.
    func pressed(_ key: EditorKey, in view: ItemTextView) -> Bool {
        let lines = [.up, .down, .selectUp, .selectDown].contains(key)
        return shell.pressed(.named(itemId: itemId, key: key, editor: view.state(measuringLines: lines)))
    }

    /// Tells the shell a key of the keyboard, which the core finds in the keys of the Mac. The
    /// lines are not measured: Up and Down that the core does not take here come back from
    /// AppKit as `moveUp:` and `moveDown:`, which measure them.
    func pressed(_ chord: Chord, in view: ItemTextView) -> Bool {
        shell.pressed(.inEditor(itemId: itemId, chord: chord, editor: view.state(measuringLines: false)))
    }

    /// The user pasted: the core pastes the text of the clipboard at the cursor.
    func paste(raw: Bool, in view: ItemTextView) {
        shell.paste(into: itemId, body: view.string, cursor: view.selectedRange().location, raw: raw)
    }

    /// Text typed while the core moves the edit to another item. The core holds each letter,
    /// and gives them to the editor that takes the cursor next.
    func typedElsewhere(_ text: String) -> Bool {
        guard shell.holdsKeys else { return false }
        for letter in text { shell.hold(HeldKeys.letter(letter)) }
        return true
    }

    /// A key that is not a letter, pressed while the core moves the edit to another item. The
    /// core holds it with the letters, in the order they were pressed.
    func pressedElsewhere(_ key: DeferredKey) -> Bool {
        guard shell.holdsKeys else { return false }
        shell.hold(key)
        return true
    }

    /// Types the keys that the core held into the view, which took the cursor. Not while the
    /// screen is drawn: a letter changes what the screen draws.
    private func replayHeldKeys(into view: ItemTextView) {
        let keys = shell.takeKeysToReplay()
        guard !keys.isEmpty else { return }
        // A played Enter splits the item again: the keys after it are held again, and come to
        // the editor of the item after it
        DispatchQueue.main.async {
            for key in keys { HeldKeys.play(key, into: view) }
        }
    }

    /// The mouse went down on the text view: a drag from it may select items.
    func dragStarts(on view: ItemTextView) -> ListDrag {
        ListDrag(shell: shell, itemId: itemId, text: view)
    }

    /// The cursor left the text view: the edit of the item is over.
    func lostCursor() {
        shell.finishEditing(itemId)
        // The core may have ended the edit before, while the cursor was still here. Then
        // nothing changes for the row now, and it would go on to show Markdown
        texts.redraw()
    }

    /// The text view got the cursor, by a click or by an order of the core.
    func gotCursor(in view: ItemTextView) {
        if !view.isMarkdown { view.showMarkdown(body, font: font) }
        // Only the user puts the cursor here while items are selected. The core ends the
        // selection then (ADR 057), and still names this item as the one with the cursor
        let selecting = shell.view.outline.selectionMode
        // The core may name this item as the one with the cursor and still not count it as
        // edited: the cursor left it for something that is no item, and came back
        let edited = shell.view.outline.rows.contains { $0.item.id == itemId && $0.isEditing }
        guard selecting || !edited || shell.view.outline.focusedItemId != itemId else { return }
        if let letter = view.touched {
            // The core answers with an order to put the cursor at the place in the Markdown
            // that was clicked, which `putCursor` carries out
            shell.touched(itemId, letter: letter)
            return
        }
        shell.startEditing(
            itemId, at: Int(charsOfUtf16(text: view.string, units: UInt64(view.selectedRange().location))))
        // The core answers with an order to put the cursor there. The window gives the view
        // the cursor before the click arrives in `mouseDown`, which puts it where the user
        // clicked: the order is not carried out
        if let request = shell.view.outline.focusRequest, request.itemId == itemId {
            cursors.request = request.seq
        }
    }

    /// Carries out an order of the core that is new and names this item.
    func putCursor(
        in view: ItemTextView, request: FocusRequest?, order: FocusOrder?, tries: Int = 10
    ) {
        let asked = request.flatMap { $0.itemId == itemId && $0.seq != cursors.request ? $0 : nil }
        // The core names the item of every order it gives: none is for the first editor alone
        let ordered = order.flatMap { $0.number != cursors.order && $0.itemId == itemId ? $0 : nil }
        guard asked != nil || ordered != nil else { return }
        guard let window = view.window else {
            // The editor of a new item is not in its window yet when the order comes with it
            guard tries > 0 else { return }
            DispatchQueue.main.async { [self] in
                putCursor(in: view, request: request, order: order, tries: tries - 1)
            }
            return
        }
        if let asked { cursors.request = asked.seq }
        if let ordered { cursors.order = ordered.number }
        // A cursor in an editor would end the selection
        guard !shell.view.outline.selectionMode else { return }
        window.makeFirstResponder(view)
        if let cursor = asked?.cursor {
            let place = Int(utf16OfChars(text: view.string, chars: cursor))
            view.setSelectedRange(NSRange(location: place, length: 0))
        }
        replayHeldKeys(into: view)
    }
}

/// A plain text view that is as high as its text.
final class ItemTextView: NSTextView {
    var onCursor: (() -> Void)?
    var onCursorLeft: (() -> Void)?
    /// A link of the shown text was clicked, with the keys that were held.
    var onLink: ((RichLink, NSEvent.ModifierFlags) -> Void)?
    /// The link under the mouse, when the window last asked which view is there. It asks right
    /// before it gives the cursor to the view that is clicked.
    private var aimed: RichLink?
    /// The text with its decoration that the view shows, when it does not show Markdown.
    var shown: NSAttributedString?
    /// How many letters of the shown text, as the core counts them, stand before the click
    /// that gave the view the cursor. `nil` when the cursor came another way.
    private(set) var touched: Int?
    /// The mouse went down on the view. What is returned follows the drag.
    var onPress: ((ItemTextView) -> ListDrag)?
    /// A key of the outline that the view named itself, as Undo of the Edit menu. True when it
    /// was taken.
    var onKey: ((EditorKey, ItemTextView) -> Bool)?
    /// A key of the keyboard, which the core may take (ADR 052). True when it was taken.
    var onChord: ((Chord, ItemTextView) -> Bool)?
    /// The user pasted, as it is when the flag is true. The core pastes, and not the text
    /// view: a Markdown list becomes items.
    var onPaste: ((Bool, ItemTextView) -> Void)?
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

    /// The font of the Markdown of an item. An item that is one heading line has the size of
    /// its heading, in bold, as the web frontend has: the text keeps its size when the cursor
    /// goes in and out.
    static func markdownFont(heading: Int, zoom: Double) -> NSFont {
        let size = NSFont.systemFontSize * CGFloat(zoom)
        guard (1...5).contains(heading) else { return .systemFont(ofSize: size) }
        return .boldSystemFont(ofSize: size * CGFloat(Row.headingScales[heading - 1]))
    }

    /// The view shows the Markdown of its item, and can be typed into.
    var isMarkdown: Bool {
        shown == nil && isEditable
    }

    /// Shows the Markdown of the item in place of its text with decoration. The cursor stays as
    /// far into the text as it was: in the text with decoration that is not the same place,
    /// but one near it.
    func showMarkdown(_ body: String, font: NSFont) {
        let cursor = min(selectedRange().location, (body as NSString).length)
        let plain: [NSAttributedString.Key: Any] = [
            .font: font, .foregroundColor: NSColor.labelColor,
        ]
        shown = nil
        isEditable = true
        // The whole text with its attributes: the letters alone would keep the decoration
        textStorage?.setAttributedString(NSAttributedString(string: body, attributes: plain))
        typingAttributes = plain
        setSelectedRange(NSRange(location: cursor, length: 0))
        invalidateIntrinsicContentSize()
    }

    /// The link of the shown text at a place of the view. Markdown has no links.
    private func link(at place: NSPoint) -> RichLink? {
        guard shown != nil, let layoutManager, let textContainer, let textStorage else { return nil }
        let inText = NSPoint(x: place.x - textContainerOrigin.x, y: place.y - textContainerOrigin.y)
        let glyph = layoutManager.glyphIndex(for: inText, in: textContainer)
        // Beside the text the nearest letter is named: only a click on the letter counts
        let box = layoutManager.boundingRect(forGlyphRange: NSRange(location: glyph, length: 1), in: textContainer)
        guard box.contains(inText) else { return nil }
        let letter = layoutManager.characterIndexForGlyph(at: glyph)
        guard letter < textStorage.length else { return nil }
        return ShownText.link(in: textStorage, at: letter)
    }

    override func hitTest(_ point: NSPoint) -> NSView? {
        let hit = super.hitTest(point)
        aimed = hit === self ? link(at: convert(point, from: superview)) : nil
        return hit
    }

    /// A click on a link does not put the cursor in the item: the item would turn into
    /// Markdown before the click arrives, and the link would be gone.
    override var acceptsFirstResponder: Bool {
        aimed == nil && super.acceptsFirstResponder
    }

    override func mouseDown(with event: NSEvent) {
        if let link = link(at: convert(event.locationInWindow, from: nil)) {
            onLink?(link, event.modifierFlags)
            return
        }
        // The click gave the view the cursor on the shown text, and the core puts it at the
        // place in the Markdown. AppKit would put it at the same point of the Markdown, which is
        // another letter
        let onShownText = touched != nil
        touched = nil
        // A text view follows the mouse by itself until the button comes up, and so never
        // tells that the mouse left it
        if event.clickCount == 1, event.modifierFlags.intersection([.shift, .control, .option, .command]).isEmpty {
            if !onShownText { setSelectedRange(NSRange(location: letter(at: event.locationInWindow), length: 0)) }
            follow()
            return
        }
        if !onShownText { super.mouseDown(with: event) }
    }

    /// Follows the mouse until the button comes up. Once it leaves the text up or down, the
    /// items it goes over are selected.
    private func follow() {
        guard let window, let drag = onPress?(self) else { return }
        // Where the letters that the drag selects begin: the cursor at the first move. On the
        // shown text the core puts it at the place that was clicked, a moment after the click
        var anchor: Int?
        while true {
            // While the list scrolls, a step passes without a move of the mouse
            let wait = drag.scrolls ? Date(timeIntervalSinceNow: EdgeScroll.interval) : .distantFuture
            guard
                let next = window.nextEvent(
                    matching: [.leftMouseDragged, .leftMouseUp], until: wait, inMode: .eventTracking, dequeue: true)
            else {
                drag.scrollOneStep()
                continue
            }
            if next.type == .leftMouseUp { break }
            let place = next.locationInWindow
            drag.moved(to: place)
            if !drag.selects, isMarkdown {
                let from = anchor ?? selectedRange().location
                anchor = from
                let to = letter(at: place)
                setSelectedRange(NSRange(location: min(from, to), length: abs(to - from)))
            }
        }
        drag.released()
    }

    /// The place in the text nearest to the place of the window.
    private func letter(at place: NSPoint) -> Int {
        characterIndexForInsertion(at: convert(place, from: nil))
    }

    /// The letter of the shown text under the click that is going on. The window gives the
    /// cursor to the view before the click arrives in `mouseDown`.
    private func clickedLetter() -> Int? {
        guard let shown, let event = NSApp.currentEvent, event.type == .leftMouseDown, event.window === window
        else { return nil }
        let index = characterIndexForInsertion(at: convert(event.locationInWindow, from: nil))
        return ShownText.letter(in: shown, at: index)
    }

    override func becomeFirstResponder() -> Bool {
        touched = clickedLetter()
        let became = super.becomeFirstResponder()
        if became { onCursor?() }
        return became
    }

    override func resignFirstResponder() -> Bool {
        let resigned = super.resignFirstResponder()
        if resigned { onCursorLeft?() }
        return resigned
    }

    /// Text that is typed, and is not for this item: the core is moving the edit to another
    /// item, as after Enter. True when it was taken. Text still being composed is not asked
    /// about: it belongs to the input method until it is done.
    var onTypedElsewhere: ((String) -> Bool)?
    /// A key that is not a letter, pressed while the core moves the edit to another item. True
    /// when it was taken.
    var onKeyElsewhere: ((DeferredKey) -> Bool)?

    override func insertText(_ string: Any, replacementRange: NSRange) {
        let text = (string as? NSAttributedString)?.string ?? (string as? String) ?? ""
        if !hasMarkedText(), onTypedElsewhere?(text) == true { return }
        super.insertText(string, replacementRange: replacementRange)
    }

    /// The keys of the keyboard go to the core first, which knows what they mean on a Mac
    /// (ADR 052). A key that it does not take goes on to AppKit, whose commands, such as
    /// `moveUp:` for Control+P, come back through `doCommandBy`.
    override func keyDown(with event: NSEvent) {
        // While text is composed every key belongs to the input method
        if !hasMarkedText() {
            if let key = HeldKeys.of(event), onKeyElsewhere?(key) == true { return }
            if take(event) { return }
        }
        super.keyDown(with: event)
    }

    private func take(_ event: NSEvent) -> Bool {
        let held = event.modifierFlags.intersection([.shift, .control, .option, .command])
        if event.keyCode == 36, held == [.shift] {
            // A line break inside the item. Enter alone splits the item
            insertText("\n", replacementRange: selectedRange())
            return true
        }
        if event.keyCode == 9, held == [.shift, .command] {
            // The text goes into the item as it is, as in the web frontend
            onPaste?(true, self)
            return true
        }
        guard let chord = Chord.of(event) else { return false }
        if case .letter = chord.key, held.isSubset(of: [.shift]) {
            // A letter that is typed: it means nothing to the outline, and the text would
            // cross to the core with every letter
            return false
        }
        return onChord?(chord, self) ?? false
    }

    /// "Undo" and "Redo" of the Edit menu, with their keys Command+Z and Shift+Command+Z. The
    /// history is the one of the core: the text view keeps none.
    @objc func undo(_ sender: Any?) {
        _ = onKey?(.undo, self)
    }

    @objc func redo(_ sender: Any?) {
        _ = onKey?(.redo, self)
    }

    override func paste(_ sender: Any?) {
        onPaste?(false, self)
    }

    /// "Paste and Match Style" of the Edit menu. The item has no style to match: the text goes in
    /// as it is.
    override func pasteAsPlainText(_ sender: Any?) {
        onPaste?(true, self)
    }

    func height(forWidth width: CGFloat) -> CGFloat {
        guard let container = textContainer, let layout = layoutManager else { return 17 }
        container.containerSize = NSSize(width: width, height: CGFloat.greatestFiniteMagnitude)
        layout.ensureLayout(for: container)
        let line = layout.defaultLineHeight(for: font ?? .systemFont(ofSize: NSFont.systemFontSize))
        return max(layout.usedRect(for: container).height, line)
    }

    /// The editor as the core reads a key. The lines are only measured for a key that asks
    /// for them: the measure lays out the text.
    func state(measuringLines: Bool) -> EditorState {
        let selected = selectedRange()
        let (first, last) = measuringLines ? cursorLines() : (false, false)
        return EditorState(
            text: string, selectionStart: UInt64(selected.location),
            selectionEnd: UInt64(selected.location + selected.length),
            onFirstLine: first, onLastLine: last)
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
