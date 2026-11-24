import AppKit
import Combine
import Foundation
import PetaCore
import PetaShared
@preconcurrency import PetaTypes

/// What one window shows, kept up to date with the core. The views draw `view` and tell this
/// what the user did. All of it runs on the main thread.
@MainActor
public final class Shell: ObservableObject {
    @Published public private(set) var view: ViewModel {
        // Not set again when it is the same: everything that watches the title would run
        didSet { if title != view.title { title = view.title } }
    }
    /// The name of what is open, as the core gives it in the view model. For a journal it is
    /// the date of the day at the top of the window, with "(Today)" behind the date of today.
    @Published public private(set) var title = ""
    /// What is open, as the URL that opens it again. `nil` when nothing was opened yet.
    @Published public private(set) var url: String?
    /// What the core asks the user now. The core waits until it is answered, and asks the
    /// next question only after that.
    @Published public private(set) var question: Question?
    private let border: Border
    private let clipboard: () -> String?
    private let image: () -> URL?
    private let copy: (String, String?) -> Void
    /// The address of the journal server. It serves the web frontend too, so a link that
    /// another app is to open starts with it.
    private let server: String
    private let browse: (URL) -> Void
    private let openSticky: (String) -> Void
    /// The images of the pages, from the journal server.
    public let images: AssetImages

    /// Starts a core for the journal server at `server` and opens what `url` names, or nothing
    /// when there is no URL: a page is then to be searched for. `clipboard`
    /// gives the text on the clipboard when the core asks for it, and `copy` puts text there,
    /// with the same as HTML when there is any. `image` writes the image on the clipboard to a
    /// file of its own and gives the file, or `nil` when there is no image.
    /// `browse` opens an address outside the journal, and `openSticky` asks for a new sticky
    /// with what a URL names.
    public init(
        server: String, url: String?,
        clipboard: @escaping () -> String? = { NSPasteboard.general.string(forType: .string) },
        image: @escaping () -> URL? = { PastedImage.write(from: .general) },
        copy: @escaping (String, String?) -> Void = { text, html in
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
            // An app that takes rich text reads this, and keeps lists, bold and links
            if let html { NSPasteboard.general.setString(html, forType: .html) }
        },
        browse: @escaping (URL) -> Void = { NSWorkspace.shared.open($0) },
        openSticky: @escaping (String) -> Void = { _ in }
    ) {
        let inbox = Inbox()
        let border = Border(server: server) { requests in
            // The core calls from its own threads. The main queue keeps the requests in order
            DispatchQueue.main.async {
                MainActor.assumeIsolated { inbox.shell?.handle(requests) }
            }
        }
        self.border = border
        self.clipboard = clipboard
        self.image = image
        self.copy = copy
        self.server = server
        self.browse = browse
        self.openSticky = openSticky
        images = AssetImages(server: server)
        view = ViewModel(first: Shell.must { try border.changes() })
        title = view.title
        inbox.shell = self
        if let url { open(url) }
    }

    /// Opens what the URL names. A URL that names nothing changes nothing.
    public func open(_ url: String) {
        guard (try? border.open(url)) != nil else { return }
        self.url = url
    }

    /// Back to the page that was open before, as the core keeps the history (ADR 035).
    /// Nothing happens when `view.history.canGoBack` is false.
    public func goBack() {
        turn(.back(today: Shell.today()))
    }

    /// Forward to the page that was open before `goBack`.
    public func goForward() {
        turn(.forward(today: Shell.today()))
    }

    private func turn(_ event: Event) {
        send(event)
        // The URL of the page that the core opened from its history
        if let shown = view.history.url { url = shown }
    }

    /// The local date, as `YYYY-MM-DD`.
    private static func today() -> String {
        let day = Calendar.current.dateComponents([.year, .month, .day], from: Date())
        return String(format: "%04d-%02d-%02d", day.year ?? 0, day.month ?? 0, day.day ?? 0)
    }

    /// The user clicked a link of a shown item, with the keys `held`. A link to a page, and to
    /// the page of a tag, opens the page in this window. With Command held it opens in a new
    /// sticky, and with Shift in the browser: the journal server serves the web frontend too. A
    /// link out of the journal, and to a file of the assets folder, opens in the browser.
    public func follow(_ link: RichLink, holding held: NSEvent.ModifierFlags = []) {
        switch link {
        case .page(let url):
            if held.contains(.command) {
                openSticky(url)
            } else if held.contains(.shift) {
                browse(pageAddress(appUrl: server, url: url))
            } else {
                open(url)
            }
        case .web(let address): browse(address)
        case .file(let path): if let address = images.address(of: path) { browse(address) }
        }
    }

    private func browse(_ address: String) {
        if let url = URL(string: address) { browse(url) }
    }

    /// The window was scrolled: the row at `top` is the first that can be seen, and the rows
    /// of `shown` are drawn. A journal tells the day at the top by this, and loads the days
    /// before when the end of what it has comes near.
    public func scrolled(top: Int, shown: ClosedRange<Int>) {
        send(
            .outlineScrolled(
                scrolledFromTop: top > 0, topVisibleIndex: UInt64(top), renderedStart: UInt64(shown.lowerBound),
                renderedEnd: UInt64(shown.upperBound + 1)))
    }

    /// The day may have changed while the core could not look at the clock, as when the Mac
    /// woke: the journal of today opens the new day at once (#796).
    public func dayMayHaveChanged() {
        send(.dateMayHaveChanged)
    }

    /// The user clicked the mark of a task: its state goes one step on, as with the key.
    public func clickedTask(of itemId: String) {
        send(.taskCycle(itemId: itemId))
    }

    /// The user clicked the shown text of the item after `letter` of its letters, as
    /// `RichParagraph.start` counts them. The item is edited with the cursor at that place in
    /// its Markdown (ADR 043). The core ends the edit of another item and a selection (ADR 057).
    public func touched(_ itemId: String, letter: Int) {
        send(.editRequested(itemId: itemId, place: .shownLetter(UInt64(letter))))
    }

    /// The user clicked something in the sticky that is no item, such as the title bar or a
    /// button. The core ends a selection, unless the items are being selected by a drag.
    public func clickedOutsideItems() {
        send(.selectionClickOutside)
    }

    /// The user put the cursor in the item, `cursor` characters into its Markdown. The core
    /// ends the edit of another item and a selection (ADR 057).
    public func startEditing(_ itemId: String, at cursor: Int = 0) {
        send(.editRequested(itemId: itemId, place: .markdown(UInt64(cursor))))
    }

    /// The cursor left the item. The core saves what was typed, and the item is shown with its
    /// decoration again.
    public func finishEditing(_ itemId: String) {
        send(.editFinished(itemId: itemId))
    }

    /// The window closes, or the app ends. The text typed and still waiting for its save is
    /// saved before this returns: the moment later would never come (#766).
    public func close() {
        border.close()
    }

    /// The user changed the text of the item. The core saves it a moment later.
    public func typed(_ body: String, into itemId: String) {
        typed(.changed(itemId: itemId, body: body))
    }

    /// The user typed into the item, and the cursor is `cursor` UTF-16 units into `body`. The
    /// core closes an opening just typed (ADR 057), and saves the body a moment later.
    public func typed(_ body: String, into itemId: String, cursor: Int) {
        typed(.typed(itemId: itemId, body: body, cursor: UInt64(max(cursor, 0))))
    }

    /// True while the core moves the edit to another item, as after Enter until the item is
    /// split. What is typed then is for the item the edit moves to, not for the editor that
    /// still has the cursor.
    public var holdsKeys: Bool { view.outline.keyRelayHolding }

    /// The user pressed a key while the core moves the edit. The core holds it, and gives it
    /// back with the order to put the cursor in the next item (`takeKeysToReplay`).
    public func hold(_ key: DeferredKey) {
        send(.keyHeld(key))
    }

    /// The keys the core held, to be played into the editor that takes the cursor now. They
    /// are given out once.
    public func takeKeysToReplay() -> [DeferredKey] {
        defer { keysToReplay = [] }
        return keysToReplay
    }

    private var keysToReplay: [DeferredKey] = []

    /// The user pasted into the item, with the cursor `cursor` UTF-16 units into `body`. An
    /// image on the clipboard goes to the core as a file, which links it at the cursor. Else the
    /// text on the clipboard goes to the core, which asks the server to cut it into items (ADR
    /// 011). A raw paste puts the text into the item as it is. Nothing happens when the
    /// clipboard has neither.
    public func paste(into itemId: String, body: String, cursor: Int, raw: Bool) {
        if let file = image() {
            pasteImage(file, into: itemId, body: body, cursor: cursor)
            return
        }
        guard let text = clipboard() else { return }
        // AppKit counts in UTF-16 units, the core in characters (ADR 043)
        let chars = charsOfUtf16(text: body, units: UInt64(max(cursor, 0)))
        // The core reads no HTML yet: it pastes the text
        send(.pasteText(itemId: itemId, text: text, html: "", cursor: chars, raw: raw))
    }

    /// The user pasted the image in `file` into the item, with the cursor `cursor` UTF-16 units
    /// into `body`. The core uploads the image to the server, puts a link to it at the cursor,
    /// and deletes the file (ADR 071).
    public func pasteImage(_ file: URL, into itemId: String, body: String, cursor: Int) {
        let chars = charsOfUtf16(text: body, units: UInt64(max(cursor, 0)))
        border.pasteImage(file, into: itemId, cursor: chars)
    }

    /// The core adds the time, and counts the cursor in its own characters.
    private func typed(_ typing: Typing) {
        Shell.must { try border.typed(typing) }
        // As after `send`: a text view that reads its old text back after a key loses the key
        refresh()
    }

    /// Answers the confirmation on screen.
    /// A notice waits for no answer: it only goes away.
    public func answer(yes: Bool) {
        guard let asked = question else { return }
        question = later.isEmpty ? nil : later.removeFirst()
        if asked.kind != .notice { Shell.must { try border.answer(asked.id, with: .bool(yes)) } }
        refresh()
    }

    /// What came while another question was shown, in the order it came.
    private var later: [Question] = []

    /// A notice may come at any time: it waits behind what is shown, which may be a question
    /// that the core waits for.
    private func ask(_ asked: Question) {
        if question == nil { question = asked } else { later.append(asked) }
    }

    /// Sends what the user did to the core.
    func send(_ event: Event) {
        Shell.must { try border.send(event) }
        // Not left to the render request. It comes a turn of the main queue later, and a text
        // field that reads its old text back after a key loses the key. Some events, such as
        // the start of an edit, send no render request at all
        refresh()
    }

    /// The user pressed a key. The core decides what it means (ADR 052). False when it means
    /// nothing to the outline there, and the editor does with it what it does in any text.
    public func pressed(_ key: KeyPress) -> Bool {
        guard Shell.must({ try border.press(key) }) else { return false }
        // As after `send`: a text view that reads its old text back after a key loses the key
        refresh()
        return true
    }

    /// A step of a drag of the mouse that started on the text of an item. The core decides when
    /// the drag leaves the text and selects items, and which items.
    public func dragged(_ step: DragMove) {
        guard Shell.must({ try border.drag(step) }) else { return }
        refresh()
    }

    private func handle(_ requests: [Request]) {
        var cursors: [UInt32] = []
        var replay: [DeferredKey] = []
        for request in requests {
            switch request.effect {
            case .render:
                refresh()
            case .navigate(let op):
                open(op.url)
            case .ui(.focusEditor(_, let keys)):
                cursors.append(request.id)
                replay += keys
            case .ui(let op):
                show(op, asked: request.id)
            }
        }
        // After the render: the item to put the cursor in may have come with these very requests
        // The order itself is in the view model, as `outline.focusOrder`. It is read again
        // here: no render request has to come with the order
        if !cursors.isEmpty {
            // Not added to the keys of an order before: the editor of that order took them, or
            // no editor took the cursor and they are for no item now
            keysToReplay = replay
            refresh()
        }
        for id in cursors {
            Shell.must { try border.answer(id, with: .done) }
        }
    }

    /// Only some requests wait for an answer. The core takes an answer to any other as an error.
    private func show(_ op: UiOp, asked id: UInt32) {
        switch op {
        case .confirm(let message):
            ask(Question(id: id, message: message, kind: .confirm))
        case .confirmDestructive(let message):
            ask(Question(id: id, message: message, kind: .confirmDestructive))
        case .alert(let message):
            // As when a page of a name that cannot be made is made from the search box
            ask(Question(id: id, message: message, kind: .notice))
        // No prompt is shown. The core asks one only for a new page without a name, and this
        // app has no way to make one: its search box makes a page with the typed name
        case .copyMarkdown(let markdown, let ast):
            // The HTML is for other apps (ADR 027). Its links to pages open the web frontend,
            // as those of a copy made there: this app has no address that another app opens
            let html = copiedHtml(ast: ast, appUrl: server, tagsDir: view.config.tagsDir)
            copy(markdown, html.isEmpty ? nil : html)
        case .readClipboardText:
            Shell.must { try border.answer(id, with: .text(clipboard())) }
        default:
            break
        }
    }

    /// Brings `view` up to date with what changed in the core (ADR 076). Nothing is published
    /// when nothing changed, so the views that watch the shell are not drawn again for it.
    private func refresh() {
        let changes = Shell.must { try border.changes() }
        guard !changes.rows.isEmpty || !changes.panels.isEmpty else { return }
        view.apply(changes)
    }

    /// Runs what crosses the border. The types on both sides are generated from one source, so
    /// a failure here is a build that mixes two versions.
    private static func must<T>(_ crossing: () throws -> T) -> T {
        do {
            return try crossing()
        } catch {
            fatalError("The border cannot be crossed: \(error)")
        }
    }
}

/// Something the core asks the user and waits for, or tells the user.
public struct Question: Equatable, Identifiable {
    /// The number of the request of the core.
    public let id: UInt32
    public let message: String
    public let kind: Kind

    public enum Kind {
        /// Yes or no.
        case confirm
        /// Yes or no to something hard to undo: the dialog must answer no by default.
        case confirmDestructive
        /// Only OK: the core tells something, as why a page cannot be made, and waits for no
        /// answer.
        case notice
    }
}

/// Where the requests of the core find the shell, which is made after the core.
private final class Inbox: @unchecked Sendable {
    weak var shell: Shell?
}
