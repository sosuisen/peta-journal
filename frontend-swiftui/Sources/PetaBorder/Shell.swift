import AppKit
import Combine
import Foundation
@preconcurrency import PetaTypes

/// What one window shows, kept up to date with the core. The views draw `view` and tell this
/// what the user did. All of it runs on the main thread.
@MainActor
public final class Shell: ObservableObject {
    @Published public private(set) var view: ViewModel {
        didSet { name() }
    }
    /// The name of what is open. For a journal it is the date of the day at the top of the
    /// window, with "(Today)" behind the date of today.
    @Published public private(set) var title = ""
    /// The name that the core gave to what is open.
    private var named = ""
    /// What is open, as the URL that opens it again. `nil` when nothing was opened yet.
    @Published public private(set) var url: String?
    /// What the core asks the user now. The core waits until it is answered.
    @Published public private(set) var question: Question?
    /// The last order of the core to put the cursor somewhere.
    @Published public private(set) var focus: FocusOrder?
    private let border: Border
    private let clipboard: () -> String?
    private let copy: (String) -> Void
    private let browse: (URL) -> Void
    /// The questions that wait behind the one on screen.
    private var later: [Question] = []

    /// Starts a core for the journal server at `server` and opens what `url` names, or nothing
    /// when there is no URL: a page is then to be searched for. `clipboard`
    /// gives the text on the clipboard when the core asks for it, and `copy` puts text there.
    /// `browse` opens an address outside the journal.
    public init(
        server: String, url: String?,
        clipboard: @escaping () -> String? = { NSPasteboard.general.string(forType: .string) },
        copy: @escaping (String) -> Void = { text in
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
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
        self.browse = browse
        view = Shell.must { try border.view() }
        inbox.shell = self
        if let url { open(url) }
    }

    /// Opens what the URL names. A URL that names nothing changes nothing.
    public func open(_ url: String) {
        guard (try? border.open(url)) != nil else { return }
        self.url = url
    }

    /// The user clicked a link of a shown item. A wikilink and a hashtag open their page here,
    /// as in the web frontend; any other link opens in the browser.
    public func follow(_ link: URL) {
        switch ShownText.target(of: link) {
        case .page(let path): open("/pages/\(path)")
        case .tag(let name): open("/pages/\(view.config.tagsDir)\(name)")
        case .out(let address): browse(address)
        case nil: break
        }
    }

    /// Sets the title. The core names every journal by the day it was opened for; the day at
    /// the top of the window is only in the view model.
    private func name() {
        var new = named
        if case .journal(let journal) = view.screen, !journal.displayDate.isEmpty {
            // The days below the one that was opened have their date above their first item
            let top = view.outline.rows.first { $0.dateHeader == journal.displayDate }
            let day = top?.item.name ?? journal.currentDate
            new = day == view.today ? "\(journal.displayDate) (Today)" : journal.displayDate
        }
        // Not set again when it is the same: everything that watches the title would run
        if title != new { title = new }
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

    /// The user put the cursor in the item.
    public func startEditing(_ itemId: String) {
        send(.editRequested(itemId: itemId, cursor: 0))
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
        question = later.isEmpty ? nil : later.removeFirst()
        Shell.must { try border.answer(asked.id, with: output) }
        view = Shell.must { try border.view() }
    }

    private func ask(_ asked: Question) {
        if question == nil {
            question = asked
        } else {
            later.append(asked)
        }
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
        var cursors: [(UInt32, String?)] = []
        for request in requests {
            switch request.effect {
            case .render:
                view = Shell.must { try border.view() }
            case .navigate(let op):
                open(op.url)
            case .ui(.focusEditor(let itemId, _)):
                cursors.append((request.id, itemId))
            case .ui(let op):
                show(op, asked: request.id)
            }
        }
        // After the render: the item to put the cursor in may have come with these very requests
        for (id, itemId) in cursors {
            focus = FocusOrder(number: (focus?.number ?? 0) + 1, itemId: itemId)
            Shell.must { try border.answer(id, with: .done) }
        }
    }

    /// Only some requests wait for an answer. The core takes an answer to any other as an error.
    private func show(_ op: UiOp, asked id: UInt32) {
        switch op {
        case .setTitle(let title):
            named = title
            name()
        case .confirm(let message):
            ask(Question(id: id, message: message, kind: .confirm))
        case .confirmDestructive(let message):
            ask(Question(id: id, message: message, kind: .confirmDestructive))
        case .prompt(let message):
            ask(Question(id: id, message: message, kind: .prompt))
        case .copyText(let text), .copyMarkdown(let text):
            // Markdown goes as plain text: no rich text is made of it here
            copy(text)
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

/// An order to put the cursor in the editor of an item, or of the first item when `itemId` is
/// `nil`. `number` tells two orders for the same item apart.
public struct FocusOrder: Equatable {
    public let number: Int
    public let itemId: String?
}

/// Where the requests of the core find the shell, which is made after the core.
private final class Inbox: @unchecked Sendable {
    weak var shell: Shell?
}
