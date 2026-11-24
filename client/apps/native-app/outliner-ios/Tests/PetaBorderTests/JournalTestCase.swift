import Foundation
import PetaBorder
import PetaTypes
import XCTest

/// A test with a real core under the shell, on folders of its own.
@MainActor
class JournalTestCase: XCTestCase {
    private(set) var root: URL!
    var shell: Shell!
    /// What the shell put on the clipboard, the last last.
    var copied: [String] = []
    /// The addresses that the shell opened in the browser.
    var browsed: [URL] = []

    override func setUp() async throws {
        root = FileManager.default.temporaryDirectory.appendingPathComponent("peta-shell-\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: markdown.appendingPathComponent("docs"), withIntermediateDirectories: true)
    }

    override func tearDown() async throws {
        shell?.close()
        shell = nil
        try? FileManager.default.removeItem(at: root)
    }

    var markdown: URL { root.appendingPathComponent("markdown") }

    /// Puts a doc page into the Markdown folder, as a download does. A file without the lines
    /// that the core writes above each item is one item, with the whole text as its body.
    func download(_ name: String, _ text: String) throws {
        try Data(text.utf8).write(to: markdown.appendingPathComponent("docs/\(name)"))
    }

    /// The line that the core writes above an item in a file, with the properties of the item.
    func above(_ id: Character, _ properties: String = "") -> String {
        "<!-- @ 2020-01-01-00-00-00-AAAAAA\(id) idx=\"a\(id)\"\(properties) -->\n"
    }

    /// Opens the app on the folders, and waits until the files are taken in.
    func start() throws {
        shell = try Shell(
            dataDir: root.appendingPathComponent("data"), markdownDir: markdown,
            copy: { [weak self] in self?.copied.append($0) },
            browse: { [weak self] in self?.browsed.append($0) })
        takeIn()
    }

    /// Lets the core take in the files, and write the pages that changed in the app.
    func takeIn() {
        shell.sync()
        waitUntil("the files are taken in") { !self.shell.syncing }
    }

    /// Does the work of the main thread until the condition holds.
    func waitUntil(_ what: String, _ holds: () -> Bool, file: StaticString = #filePath, line: UInt = #line) {
        let deadline = Date().addingTimeInterval(10)
        while !holds() {
            if Date() > deadline { return XCTFail("Waited in vain until \(what)", file: file, line: line) }
            RunLoop.current.run(until: Date().addingTimeInterval(0.005))
        }
    }

    /// Searches for the word and chooses the first result of the kind.
    func pick(_ kind: SearchKind, searchingFor query: String) {
        pick(searchingFor: query) { $0.kind == kind }
    }

    /// Searches for the word and chooses the first result that is wanted.
    func pick(searchingFor query: String, _ wanted: @escaping (SearchEntry) -> Bool) {
        shell.search(query)
        waitUntil("the search finds a result for \(query)") { self.shell.view.search.results.contains(where: wanted) }
        if let found = shell.view.search.results.first(where: wanted) { shell.pick(found) }
    }

    /// Opens the page `notes`, which a test has put into the Markdown folder.
    func openNotes() throws {
        try start()
        pick(.page, searchingFor: "notes")
        waitUntil("the page shows its items") { !self.shell.view.outline.rows.isEmpty }
    }

}
