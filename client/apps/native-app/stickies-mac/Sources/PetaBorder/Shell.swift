import AppKit
import Combine
import Foundation
import PetaCore
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
    private let copy: (String, String?) -> Void
    /// The address of the journal server. It serves the web frontend too, so a link that
    /// another app is to open starts with it.
    private let server: String
    private let browse: (URL) -> Void
    /// The images of the pages, from the journal server.
    public let images: AssetImages

    /// Starts a core for the journal server at `server` and opens what `url` names, or nothing
    /// when there is no URL: a page is then to be searched for. `clipboard`
    /// gives the text on the clipboard when the core asks for it, and `copy` puts text there,
    /// with the same as HTML when there is any.
    /// `browse` opens an address outside the journal.
    public init(
        server: String, url: String?,
        clipboard: @escaping () -> String? = { NSPasteboard.general.string(forType: .string) },
        copy: @escaping (String, String?) -> Void = { text, html in
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
            // An app that takes rich text reads this, and keeps lists, bold and links
            if let html { NSPasteboard.general.setString(html, forType: .html) }
        },
        browse: @escaping (URL) -> Void = { NSWorkspace.shared.open($0) }
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
        self.copy = copy
        self.server = server
        self.browse = browse
        images = AssetImages(server: server)
        view = Shell.must { try border.view() }
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

    /// The user clicked a link of a shown item. A link to a page, and to the page of a tag,
    /// opens the page in this window. With Shift held it opens the page in the browser: the
    /// journal server serves the web frontend too. A link out of the journal, and to a file of
    /// the assets folder, opens in the browser.
    public func follow(_ link: RichLink, inBrowser: Bool = false) {
        switch link {
        case .page(let url):
            if inBrowser {
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

    /// The user changed the text of the item. The core saves it a moment later.
    public func typed(_ body: String, into itemId: String) {
        let nowMs = UInt64(Date().timeIntervalSince1970 * 1000)
        send(.bodyChanged(itemId: itemId, body: body, nowMs: nowMs))
    }

    /// The user typed into the item, and the cursor is `cursor` UTF-16 units into `body`. The
    /// core closes an opening just typed (ADR 057), and saves the body a moment later.
    public func typed(_ body: String, into itemId: String, cursor: Int) {
        let nowMs = UInt64(Date().timeIntervalSince1970 * 1000)
        let characters = charsOfUtf16(text: body, units: UInt64(max(cursor, 0)))
        send(.typed(itemId: itemId, body: body, cursor: characters, nowMs: nowMs))
    }

    /// Answers the confirmation on screen.
    public func answer(yes: Bool) {
        answer(.bool(yes))
    }

    /// Answers the prompt on screen. `nil` is the answer of a user who closed it.
    public func answer(text: String?) {
        answer(.text(text))
    }

    private func answer(_ output: UiOutput) {
        guard let asked = question else { return }
        question = nil
        Shell.must { try border.answer(asked.id, with: output) }
        view = Shell.must { try border.view() }
    }

    private func ask(_ asked: Question) {
        question = asked
    }

    /// Sends what the user did to the core.
    func send(_ event: Event) {
        Shell.must { try border.send(event) }
        // Not left to the render request. It comes a turn of the main queue later, and a text
        // field that reads its old text back after a key loses the key. Some events, such as
        // the start of an edit, send no render request at all
        view = Shell.must { try border.view() }
    }

    private func handle(_ requests: [Request]) {
        var cursors: [UInt32] = []
        for request in requests {
            switch request.effect {
            case .render:
                view = Shell.must { try border.view() }
            case .navigate(let op):
                open(op.url)
            case .ui(.focusEditor):
                cursors.append(request.id)
            case .ui(let op):
                show(op, asked: request.id)
            }
        }
        // After the render: the item to put the cursor in may have come with these very requests
        // The order itself is in the view model, as `outline.focusOrder`. It is read again
        // here: no render request has to come with the order
        if !cursors.isEmpty { view = Shell.must { try border.view() } }
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
        case .prompt(let message):
            ask(Question(id: id, message: message, kind: .prompt))
        case .copyText(let text):
            copy(text, nil)
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

/// Something the core asks the user and waits for.
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
        /// A line of text.
        case prompt
    }
}

/// Where the requests of the core find the shell, which is made after the core.
private final class Inbox: @unchecked Sendable {
    weak var shell: Shell?
}
