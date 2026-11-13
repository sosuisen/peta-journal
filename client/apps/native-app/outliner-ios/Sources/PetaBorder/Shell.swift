import Combine
import Foundation
import PetaSync
import PetaTypes

/// What the app shows, kept up to date with the core. The screen draws `view` and tells this
/// what the user did. All of it runs on the main thread, but the taking in of files.
@MainActor
public final class Shell: ObservableObject {
    /// What the URL of a page starts with, before the path of the page.
    public static let pages = "/pages/"
    /// The URL of the journal of today, whatever the date is.
    public static let todayUrl = "/today"

    @Published public private(set) var view: ViewModel
    /// True while files are taken in. The pages of an earlier sync can be read meanwhile.
    @Published public private(set) var syncing = false
    /// The page that a link named and that is not there, until another page opens.
    @Published public private(set) var missing: String?
    /// The file that a link named and that is not here, until the next link is touched.
    @Published public private(set) var missingFile: String?
    /// What the core asks the user before it does what is hard to undo, or nil.
    @Published public private(set) var question: Question?
    /// True when `restore` opened no screen and was not to make the journal of today. A new
    /// app waits for the sign-in: a journal made before the first download would be a second
    /// file for a day that Dropbox may hold already.
    @Published public private(set) var unopened = false
    /// The focus request that a touch on an item made. The core asks for the cursor at the
    /// start of the item then. A finger shows no place in the Markdown, so the screen puts the
    /// cursor at the end instead, where the writing goes on.
    public private(set) var touched: UInt64?

    private let markdownDir: URL
    private let onEdit: () -> Void
    private let border: Border
    /// Takes in the files, one sync after the other, and closes the core after the last.
    private let worker = DispatchQueue(label: "peta-journal.take-in")
    private var closed = false

    /// Opens the journal of the app. `dataDir` keeps its database, its index and its settings,
    /// and `markdownDir` is the folder the files from Dropbox are put in. `onEdit` is called
    /// each time the user changes a page: the app syncs a moment after the last change (ADR
    /// 031). Throws when the journal cannot be opened.
    public init(dataDir: URL, markdownDir: URL, onEdit: @escaping () -> Void = {}) throws {
        self.markdownDir = markdownDir
        self.onEdit = onEdit
        let inbox = Inbox()
        // The main queue keeps the requests in order. The first ones may come before the shell
        // is there, and are still handled after this initializer
        let border = try Border(dataDir: dataDir, markdownDir: markdownDir) { requests in
            DispatchQueue.main.async {
                MainActor.assumeIsolated { inbox.shell?.handle(requests) }
            }
        }
        self.border = border
        view = Shell.must { try border.view() }
        inbox.shell = self
    }

    // MARK: - Files

    /// The file of an image of a page. `path` is its path under the Markdown folder, such as
    /// `assets/picture.png`. `nil` when no download has brought it yet.
    public func image(_ path: String) -> URL? {
        guard let found = findWithoutCase(root: markdownDir, path: path) else { return nil }
        var folder: ObjCBool = false
        let there = FileManager.default.fileExists(atPath: found.path, isDirectory: &folder)
        return there && !folder.boolValue ? found : nil
    }

    /// The user touched a link to a file of the assets folder. `path` is its path under the
    /// Markdown folder. Answers with the file to open, or `nil` when it is not here.
    public func attached(_ path: String) -> URL? {
        let file = image(path)
        missingFile = file == nil ? path : nil
        return file
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
            MainActor.assumeIsolated { if self.unopened, !self.closed { self.restore() } }
        }
    }

    // MARK: - Opening

    /// The user touched a link to a page. `url` is the URL of the page, such as
    /// `/pages/notes`. A link to a page that is not there opens nothing, and `missing` names
    /// the page: a touch by mistake must not put an empty page into Dropbox (ADR 031).
    public func follow(_ url: String) {
        open(url, make: false)
    }

    /// The user wants the page that `missing` names: it is made and opens.
    public func make() {
        guard let page = missing else { return }
        open(Shell.pages + page, make: true)
        onEdit()
    }

    /// The user wants the journal of today: it is made when it is not there, and opens.
    public func today() {
        open(Shell.todayUrl, make: true)
        onEdit()
    }

    /// Opens the screen that the app showed when it was used last. The journal of today opens
    /// when there is none, as in a new app, and when its page has gone since. Without `make`,
    /// the journal of today is not made, and no screen opens then.
    public func restore(make: Bool = true) {
        let before = border.lastScreen()
        if let before { open(before, make: false) }
        let none = before == nil || missing != nil
        if none, make { today() }
        unopened = none && !make
    }

    private func open(_ url: String, make: Bool) {
        missingFile = nil
        missing = make ? border.openOrMake(url) : border.open(url)
        if missing == nil { unopened = false }
        show()
    }

    // MARK: - Editing

    /// The user touched an item: it is edited (ADR 031). The edit of another item ends.
    public func edit(_ itemId: String) {
        if let other = view.outline.focusedItemId, other != itemId { finish(other) }
        send(.editRequested(itemId: itemId, cursor: 0))
        touched = view.outline.focusRequest?.seq
    }

    /// The user changed the Markdown of the item. The core saves it a moment later.
    public func type(_ itemId: String, _ body: String) {
        change(.bodyChanged(itemId: itemId, body: body, nowMs: now()))
    }

    /// The user pressed Enter in the item. `cursor` counts the characters before it, as the
    /// core counts them. The core splits the item there, and the edit goes on in the new one.
    public func enter(_ itemId: String, cursor: Int) {
        change(.enterPressed(itemId: itemId, cursor: UInt64(cursor), nowMs: now()))
    }

    /// The user pressed Backspace at the start of the item. The core joins the item to the one
    /// before it, and the edit goes on there, where the two texts meet. Nothing happens in the
    /// first item of a page.
    public func backspaceAtStart(_ itemId: String) {
        change(.backspaceAtStart(itemId: itemId))
    }

    /// The user left the editor of the item: what was typed is saved.
    public func finish(_ itemId: String) {
        send(.editFinished(itemId: itemId))
    }

    private func now() -> UInt64 {
        UInt64(Date().timeIntervalSince1970 * 1000)
    }

    /// Makes the item a child of the item above it.
    public func indent(_ itemId: String) {
        change(.indent(itemId: itemId))
    }

    /// Moves the item one level up, out of its parent.
    public func outdent(_ itemId: String) {
        change(.outdent(itemId: itemId))
    }

    /// Swaps the item with the one above it on its level.
    public func moveUp(_ itemId: String) {
        change(.moveUp(itemId: itemId))
    }

    /// Swaps the item with the one below it on its level.
    public func moveDown(_ itemId: String) {
        change(.moveDown(itemId: itemId))
    }

    /// Moves the task on: no task, not started, running, done, and no task again.
    public func cycleTask(_ itemId: String) {
        change(.taskCycle(itemId: itemId))
    }

    /// Deletes the item.
    public func delete(_ itemId: String) {
        change(.deleteItem(itemId: itemId))
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
        if entry.kind == .today || entry.kind == .create { onEdit() }
    }

    // MARK: - The core

    /// Sends an event that changes a page.
    private func change(_ event: Event) {
        send(event)
        onEdit()
    }

    private func send(_ event: Event) {
        // A field that loses the focus while the screen is taken down still tells of it
        if closed { return }
        Shell.must { try border.send(event) }
        // Not left to the render request. It comes a turn of the main queue later, and a text
        // field that reads its old text back after a key loses the key
        show()
    }

    private func show() {
        let next = Shell.must { try border.view() }
        // Not set again when it is the same: everything that watches the view would be drawn
        if next != view { view = next }
    }

    private func handle(_ requests: [Request]) {
        // The app was closed while the requests waited for the main queue
        if closed { return }
        for request in requests {
            switch request.effect {
            case .render:
                show()
            case .navigate(let op):
                // The core names what it wants open, as after a search result was chosen. The
                // user asked for it, so a page that is not there is made
                open(op.url, make: true)
            case .ui(let op):
                ask(request.id, op)
            }
        }
    }

    /// Does what the core asks of the screen, and answers when the core waits for an answer.
    private func ask(_ id: UInt32, _ op: UiOp) {
        switch op {
        case .focusEditor:
            // The row that is edited takes the focus by itself when it is drawn. The core
            // waits for this answer before it ends a move or a split
            show()
            Shell.must { try border.answer(id, with: .done) }
        case .confirm(let text):
            put(Question(text: text, destructive: false, id: id))
        case .confirmDestructive(let text):
            put(Question(text: text, destructive: true, id: id))
        case .prompt, .readClipboardText:
            Shell.must { try border.answer(id, with: .text(nil)) }
        default:
            // The others wait for no answer, such as the title to set and the item to scroll to
            break
        }
    }

    /// Shows a question of the core. The core asks the next one only after this is answered.
    private func put(_ next: Question) {
        question = next
    }

    /// The user answered `question`: the core goes on with what it asked about, or leaves it.
    public func answer(yes: Bool) {
        guard let asked = question else { return }
        question = nil
        Shell.must { try border.answer(asked.id, with: .bool(yes)) }
        show()
    }

    /// Lets go of the journal. A sync that runs ends first, on its own thread.
    public func close() {
        closed = true
        worker.async { [border] in border.close() }
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
/// something, so no is the answer to choose by default.
public struct Question: Equatable {
    public let text: String
    public let destructive: Bool
    let id: UInt32
}

/// Where the requests of the core find the shell, which is made after the core.
private final class Inbox: @unchecked Sendable {
    weak var shell: Shell?
}
