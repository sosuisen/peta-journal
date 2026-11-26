import Combine
import Foundation
import PetaCore
import PetaShared
import PetaTypes

/// What the app shows, kept up to date with the core. The screen draws `view` and tells this
/// what the user did. All of it runs on the main thread, but the taking in of files.
@MainActor
public final class Shell: ObservableObject {
    @Published public private(set) var view: ViewModel
    /// True while files are taken in. The pages of an earlier sync can be read meanwhile.
    @Published public private(set) var syncing = false
    /// The file that a link named and that is not here, until the next link is touched.
    @Published public private(set) var missingFile: String?
    /// The file of the assets folder that a link opened, shown over the screen until the user
    /// closes it.
    @Published public var file: URL?
    /// What the core asks the user before it does what is hard to undo, or nil.
    @Published public private(set) var question: Question?
    /// True when `restore` opened no screen and was not to make the journal of today. A new
    /// app waits for the sign-in: a journal made before the first download would be a second
    /// file for a day that Dropbox may hold already.
    @Published public private(set) var unopened = false

    private let border: Border
    /// Puts text on the clipboard of the system.
    private let copy: (String) -> Void
    /// Opens an address in the browser of the system.
    private let browse: (URL) -> Void
    /// Takes in the files, one sync after the other, and closes the core after the last.
    private let worker = DispatchQueue(label: "peta-journal.take-in")
    private var closed = false

    /// Opens the journal of the app. `dataDir` keeps its database, its index and its settings,
    /// and `markdownDir` is the folder the files from Dropbox are put in. The core tells the
    /// sync when a page changed (ADR 055). `copy` puts text on the clipboard of the system, and
    /// `browse` opens an address in its browser. Throws when the journal cannot be opened.
    public init(
        dataDir: URL, markdownDir: URL, copy: @escaping (String) -> Void = { _ in },
        browse: @escaping (URL) -> Void = { _ in }
    ) throws {
        let inbox = Inbox()
        // The main queue keeps the requests in order. The first ones may come before the shell
        // is there, and are still handled after this initializer
        let border = try Border(dataDir: dataDir, markdownDir: markdownDir) { requests in
            DispatchQueue.main.async {
                MainActor.assumeIsolated { inbox.shell?.handle(requests) }
            }
        }
        self.border = border
        self.copy = copy
        self.browse = browse
        view = ViewModel(first: Shell.must { try border.changes() })
        inbox.shell = self
    }

    // MARK: - Opening

    /// The user touched a link of a shown item. The core decides how it opens (ADR 083). A
    /// touch holds no keys, so a page opens here, and is made when it is not there, as in the
    /// other shells.
    public func follow(_ link: RichLink) {
        send(.linkClicked(link: link, held: Held(shift: false, ctrl: false, cmd: false)))
    }

    /// The user wants the journal of today: it is made when it is not there, and opens.
    public func today() {
        told { $0.today() }
    }

    /// The day may have changed while the core could not look at the clock, as when the app
    /// was in the back: the journal of today opens the new day at once (#796).
    public func dayMayHaveChanged() {
        send(.dateMayHaveChanged)
    }

    /// The user chose "Delete" in the menu of the item. The core asks first when another item
    /// shows this one.
    public func delete(_ itemId: String) {
        send(.deleteItem(itemId: itemId))
    }

    /// The user chose "Send to Stash" in the menu of the item: it goes with its children under
    /// the heading `# Stash` at the end of the page.
    public func sendToStash(_ itemId: String) {
        send(.sendToStash(itemId: itemId))
    }

    /// The user chose "Remove Transclusion" in the menu of an item that shows another. The
    /// item stays, empty, and the item it showed stays where it is.
    public func removeTransclusion(_ itemId: String) {
        send(.removeTransclusion(itemId: itemId))
    }

    /// The page was scrolled: the row at `top` is the first that can be seen, and the rows of
    /// `shown` are drawn. A journal loads the days before when the end of what it has comes
    /// near.
    public func scrolled(top: Int, shown: ClosedRange<Int>) {
        send(
            .outlineScrolled(
                scrolledFromTop: top > 0, topVisibleIndex: UInt64(top), renderedStart: UInt64(shown.lowerBound),
                renderedEnd: UInt64(shown.upperBound + 1)))
    }

    /// Opens the screen that the app showed when it was used last. The journal of today opens
    /// when there is none, as in a new app, and when its page has gone since. Without `make`,
    /// the journal of today is not made, and no screen opens then.
    public func restore(make: Bool = true) {
        told { $0.restore(make: make) }
    }

    /// Back to the page that was open before, as the core keeps the history (ADR 035). Nothing
    /// happens when `view.history.canGoBack` is false.
    public func back() {
        told { $0.back() }
    }

    /// Forward to the page that was open before `back`.
    public func forward() {
        told { $0.forward() }
    }

    /// Shows what the core told after it was asked to open a screen. The core decides what
    /// opens (ADR 056).
    private func told(_ asking: (PetaCore) -> PetaScreen) {
        // A field that loses the focus while the screen is taken down still tells of it
        if closed { return }
        let screen = asking(core)
        missingFile = screen.missingFile
        unopened = screen.unopened
        show()
    }

    // MARK: - Searching

    /// The user typed into the search field.
    public func search(_ query: String) {
        send(.searchQueryChanged(query))
    }

    /// The user chose a search result: its page opens. The results for the journal of today
    /// and for a new page make their page.
    public func pick(_ entry: SearchEntry) {
        send(.searchResultClicked(position: entry.position))
    }

    // MARK: - Editing

    /// The user touched an item: it is edited (ADR 031). The core ends the edit of another
    /// item and a selection (ADR 057). `letter` is how many letters of the shown text of the
    /// item stand before the touch, as `RichParagraph.start` counts them: the cursor goes to
    /// that place in the Markdown. Without it the cursor is at the end, where the writing goes
    /// on.
    public func edit(_ itemId: String, letter: Int? = nil) {
        let place: EditPlace = letter.map { .shownLetter(UInt64($0)) } ?? .unplaced
        send(.editRequested(itemId: itemId, place: place))
    }

    /// The user changed the Markdown of the item. The core saves it a moment later.
    public func type(_ itemId: String, _ body: String) {
        typed(.changed(itemId: itemId, body: body))
    }

    /// The user typed into the item, and the cursor is `cursor` UTF-16 units into `body`. The
    /// core closes an opening just typed (ADR 057), and saves the body a moment later.
    public func type(_ itemId: String, _ body: String, cursor: Int) {
        typed(.typed(itemId: itemId, body: body, cursor: UInt64(max(cursor, 0))))
    }

    /// The core adds the time, and counts the cursor in its own characters.
    private func typed(_ typing: Typing) {
        // As `send`: a field that loses the focus while the screen is taken down still tells
        if closed { return }
        Shell.must { try border.typed(typing) }
        show()
    }

    /// The keyboard composes a word in an item: the user has not ended the input. The core does
    /// not hear of the word until it is committed, and the sync with the cloud waits (#884).
    public func composing() {
        onComposing()
    }

    /// Told when the keyboard composes a word in an item, as `composing` says.
    public var onComposing: () -> Void = {}

    /// The user pressed a key in the editor of the item: Enter, Backspace, or an arrow of the
    /// cursor pad. The core decides what it means (ADR 052): Enter splits the item, Backspace
    /// at the start joins it to the item before, and an arrow at the edge of the text leaves
    /// the item. False when the key means nothing to the outline there, and the field does
    /// with it what it does in any text. `editor` counts in UTF-16 units, as UIKit does.
    @discardableResult
    public func press(_ key: EditorKey, in itemId: String, _ editor: EditorState) -> Bool {
        if closed { return false }
        guard Shell.must({ try border.press(.named(itemId: itemId, key: key, editor: editor)) }) else {
            return false
        }
        show()
        return true
    }

    /// The user pasted `text` into the edited item, with the cursor `cursor` UTF-16 units into
    /// its Markdown. The core cuts the text into items (ADR 011): its first line goes on after
    /// the cursor, and each line after it is an item below. What the app copied comes back as
    /// the same items.
    public func paste(_ text: String, into itemId: String, at cursor: Int) {
        let body = view.outline.rows.first { $0.item.id == itemId }?.item.body ?? ""
        let place = UInt64(Cursor.characters(utf16: cursor, in: body))
        send(.pasteText(itemId: itemId, text: text, html: "", cursor: place, raw: false))
    }

    /// The user pasted the image in `file` into the edited item, with the cursor `cursor` UTF-16
    /// units into its Markdown. The core saves the image among the assets, puts a link to it at
    /// the cursor, and deletes `file` (ADR 071).
    public func pasteImage(_ file: URL, into itemId: String, at cursor: Int) {
        let body = view.outline.rows.first { $0.item.id == itemId }?.item.body ?? ""
        border.pasteImage(file, into: itemId, cursor: UInt64(Cursor.characters(utf16: cursor, in: body)))
    }

    /// The user left the editor of the item: what was typed is saved. An order to put the
    /// keyboard in a field that no field took is answered: no field takes it now, and the
    /// keys held for it are for no item.
    public func finish(_ itemId: String) {
        send(.editFinished(itemId: itemId))
        _ = keyboardTaken()
    }

    // MARK: - Keys typed while the edit moves

    /// True while the core moves the edit to another item, as after Enter, until the field of
    /// that item takes the keyboard (ADR 087). What is typed then is for that item, not for the
    /// field that still has the keyboard, and goes to the core with `hold`.
    public var holdsKeys: Bool { view.outline.keyRelayHolding }

    /// The user typed `text` while the core moves the edit. The core holds each letter, and a
    /// line break as the key Enter, and gives them back once the field of the item takes the
    /// keyboard (`keyboardTaken`).
    public func hold(_ text: String) {
        for letter in text {
            hold(letter == "\n" ? Shell.named("Enter") : Shell.letter(letter))
        }
    }

    /// The user pressed Backspace while the core moves the edit.
    public func holdBackspace() {
        hold(Shell.named("Backspace"))
    }

    /// Hands a key to the core while it moves the edit, as a played key after an Enter does.
    public func hold(_ key: DeferredKey) {
        send(.keyHeld(key))
    }

    /// A key that is not a letter, by the name the core keeps it under (the `key` of a
    /// keyboard event of the web).
    private static func named(_ name: String) -> DeferredKey {
        DeferredKey(key: name, code: name, shiftKey: false, ctrlKey: false, altKey: false, metaKey: false)
    }

    private static func letter(_ letter: Character) -> DeferredKey {
        DeferredKey(key: String(letter), code: "", shiftKey: false, ctrlKey: false, altKey: false, metaKey: false)
    }

    /// Told when the core orders a field to take the keyboard. The field of the edited item
    /// that has the keyboard already answers then, with `keyboardTaken`.
    public var onKeyboardOrder: () -> Void = {}

    /// True while an order of the core to put the keyboard in a field waits for its answer.
    public var waitsForTheKeyboard: Bool { !unanswered.isEmpty }

    /// The orders to put the keyboard in a field that are not answered yet, the first first.
    private var unanswered: [UInt32] = []
    /// The keys that the core gave with those orders, in the order they were typed.
    private var keysToPlay: [DeferredKey] = []

    /// A field of an item took the keyboard. The orders that wait are answered, and the core
    /// stops holding keys, or gives the keys held since with an order of its own. Returns the
    /// keys to play into the field, once: a letter as text, Enter and Backspace as the keys.
    public func keyboardTaken() -> [DeferredKey] {
        defer { keysToPlay = [] }
        let orders = unanswered
        unanswered = []
        for id in orders { Shell.must { try border.answer(id, with: .done) } }
        // Not now: a field takes the keyboard while the screen is drawn. The keys played after
        // this read the new view, which says whether the core still holds keys
        if !orders.isEmpty {
            DispatchQueue.main.async { [weak self] in self?.show() }
        }
        return keysToPlay
    }

    /// Makes the item a child of the item above it.
    public func indent(_ itemId: String) {
        send(.indent(itemId: itemId))
    }

    /// Moves the item one level up, out of its parent.
    public func outdent(_ itemId: String) {
        send(.outdent(itemId: itemId))
    }

    /// Swaps the item with the one above it on its level.
    public func moveUp(_ itemId: String) {
        send(.moveUp(itemId: itemId))
    }

    /// Swaps the item with the one below it on its level.
    public func moveDown(_ itemId: String) {
        send(.moveDown(itemId: itemId))
    }

    /// Moves the task on: no task, not started, running, done, and no task again.
    public func cycleTask(_ itemId: String) {
        send(.taskCycle(itemId: itemId))
    }

    /// Hides the children of the item, or shows them again.
    public func toggleCollapsed(_ itemId: String) {
        send(.collapseToggle(itemId: itemId))
    }

    /// Selects the item with everything under it (ADR 031). The edit of the item has ended:
    /// the keyboard is down, and the tools act on the selected items.
    public func select(_ itemId: String) {
        send(.shiftSelect(itemId: itemId))
    }

    /// The selection ends: no item is selected.
    public func unselect() {
        send(.selectionKey(.escape))
    }

    /// Moves or indents the selected items, as the keys do on a desktop.
    public func moveSelected(_ key: SelectionKey) {
        send(.selectionKey(key))
    }

    /// Puts the selected items on the clipboard, as Markdown. The core keeps how they stood, so
    /// that they are pasted back as the same items.
    public func copySelected() {
        send(.selectionKey(.copy))
    }

    /// Puts the selected items on the clipboard, as `copySelected` does, and deletes them.
    public func cutSelected() {
        send(.selectionKey(.cut))
    }

    /// Takes back the last change of the page.
    public func undo() {
        send(.undo)
    }

    /// Does again the change that `undo` took back.
    public func redo() {
        send(.redo)
    }

    // MARK: - Files

    /// The file of an image of a page. `path` is its path under the Markdown folder, such as
    /// `assets/picture.png`. `nil` when no download has brought it yet.
    public func image(_ path: String) -> URL? {
        core.image(path: path).map { URL(fileURLWithPath: $0) }
    }

    /// The core ordered the browser to open `address` (ADR 083). An address of the journal
    /// server, which starts with `/`, is a file of the assets folder: the app has no server,
    /// so the file opens in the app. When it is not here, `missingFile` names it.
    private func open(_ address: String) {
        if address.hasPrefix("/") {
            file = core.fileAt(address: address).map { URL(fileURLWithPath: $0) }
            missingFile = core.screen().missingFile
        } else if let url = URL(string: address) {
            browse(url)
        }
    }

    // MARK: - Questions

    /// The user answered `question`: the core goes on with what it asked about, or leaves it.
    /// A notice waits for no answer: it only goes away.
    public func answer(yes: Bool) {
        guard let asked = question else { return }
        question = later.isEmpty ? nil : later.removeFirst()
        if asked.waits { Shell.must { try border.answer(asked.id, with: .bool(yes)) } }
        show()
    }

    /// What came while another question was shown, in the order it came.
    private var later: [Question] = []

    /// Shows a question of the core. The core asks the next one only after this is answered.
    /// A notice may come at any time: it waits behind what is shown, which may be a question
    /// that the core waits for.
    private func put(_ next: Question) {
        if question == nil { question = next } else { later.append(next) }
    }

    // MARK: - Taking in files

    /// Takes in the files of the Markdown folder, after a download has changed them, and
    /// writes the pages that changed in the app. It runs on a thread of its own: the first
    /// sync of a large folder takes long. `then` is called on that thread when the core is
    /// done, or at once when the app has closed.
    public func sync(then: @escaping @Sendable () -> Void = {}) {
        if closed { return then() }
        syncing = true
        worker.async { [border, weak self] in
            border.sync()
            then()
            DispatchQueue.main.async {
                MainActor.assumeIsolated {
                    guard let self, !self.closed else { return }
                    self.syncing = false
                    self.show()
                }
            }
        }
    }

    /// Does what `sync` does, and returns when the core is done. It is for the thread of a
    /// sync with the cloud, which uploads what the core wrote. It must not be the main thread.
    ///
    /// A new app that waited for the sign-in opens its first screen after it: the download has
    /// brought the journal of today, if Dropbox holds one.
    public nonisolated func syncAndWait() {
        let done = DispatchSemaphore(value: 0)
        DispatchQueue.main.async {
            MainActor.assumeIsolated { self.sync { done.signal() } }
        }
        done.wait()
        DispatchQueue.main.async {
            MainActor.assumeIsolated { self.told { $0.openFirstScreen() } }
        }
    }

    /// The app goes to the back, where iOS may stop it. The edit ends, and the text typed and
    /// still waiting for its save is saved before this returns: the moment later, when it
    /// would be saved, may never come (#766). The sync after this writes it to its file.
    public func wentToTheBack() {
        if closed { return }
        border.leave()
    }

    /// Lets go of the journal. A sync that runs ends first, on its own thread.
    public func close() {
        closed = true
        worker.async { [border] in border.close() }
    }

    // MARK: - The core

    /// The core, for the sync to hear from it when a page changed (ADR 055).
    var core: PetaCore { border.petaCore }

    private func send(_ event: Event) {
        // A field that loses the focus while the screen is taken down still tells of it
        if closed { return }
        Shell.must { try border.send(event) }
        // Not left to the render request. It comes a turn of the main queue later, and a text
        // field that reads its old text back after a key loses the key
        show()
    }

    /// Brings `view` up to date with what changed in the core (ADR 076).
    private func show() {
        let changes = Shell.must { try border.changes() }
        // Not set when nothing changed: everything that watches the view would be drawn
        guard !changes.rows.isEmpty || !changes.panels.isEmpty else { return }
        view.apply(changes)
    }

    private func handle(_ requests: [Request]) {
        // The app was closed while the requests waited for the main queue
        if closed { return }
        for request in requests {
            switch request.effect {
            case .render:
                show()
            case .navigate(let op):
                // The core names what it wants open, as after a search result was chosen
                told { $0.navigated(url: op.url) }
            case .ui(let op):
                ask(request.id, op)
            }
        }
    }

    /// Does what the core asks of the screen, and answers when the core waits for an answer.
    private func ask(_ id: UInt32, _ op: UiOp) {
        switch op {
        case .focusEditor(_, let keys):
            // The row that is edited takes the keyboard by itself when it is drawn, a moment
            // later: the field answers then (`keyboardTaken`). The core holds the keys typed
            // meanwhile, and waits for the answer before it ends a move or a split
            unanswered.append(id)
            keysToPlay += keys
            show()
            // The field may have the keyboard already: the drawing before the order asked it to
            // take it, or it kept it, as after a move of its item. No field takes the keyboard
            // anew then, and a view that did not change draws nothing again
            onKeyboardOrder()
        case .confirm(let text):
            put(Question(text: text, destructive: false, id: id))
        case .confirmDestructive(let text):
            put(Question(text: text, destructive: true, id: id))
        case .alert(let text):
            // As when a page of a name that cannot be made is made from the search
            put(Question(text: text, destructive: false, id: id, waits: false))
        case .copyMarkdown(let markdown, _):
            copy(markdown)
        case .openInBrowser(let address):
            open(address)
        case .prompt, .readClipboardText:
            Shell.must { try border.answer(id, with: .text(nil)) }
        default:
            // The others wait for no answer, such as the title to set and the item to scroll to.
            // A new view is never asked for: a touch holds no keys (ADR 083)
            break
        }
    }

    /// Runs what crosses the border. The types on both sides are generated from one source, so
    /// a failure here is a build that mixes two versions.
    @discardableResult
    private static func must<T>(_ crossing: () throws -> T) -> T {
        do {
            return try crossing()
        } catch {
            fatalError("The border cannot be crossed: \(error)")
        }
    }
}

/// A question of the core with the answers yes and no. `destructive` says that yes loses
/// something, so no is the answer to choose by default. A notice of the core, as why a page
/// cannot be made, does not wait for an answer (`waits` is false): it has only OK.
public struct Question: Equatable {
    public let text: String
    public let destructive: Bool
    let id: UInt32
    public var waits = true
}

/// Where the requests of the core find the shell, which is made after the core.
private final class Inbox: @unchecked Sendable {
    weak var shell: Shell?
}
