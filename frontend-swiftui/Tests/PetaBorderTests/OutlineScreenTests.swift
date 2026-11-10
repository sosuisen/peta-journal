import AppKit
import PetaBorder
import PetaScreens
import PetaTypes
import SwiftUI
import XCTest

/// The screen in a real window, with the keys given to its editors as AppKit gives them. These
/// need a real journal server, so they are skipped without `PETA_E2E_SERVER`.
@MainActor
final class OutlineScreenTests: XCTestCase {
    private var server: JournalServer!
    private var shell: Shell!
    private var window: NSWindow!

    override func setUp() async throws {
        server = try JournalServer(port: 18094)
        shell = Shell(server: server.address, url: "/pages/screen", clipboard: { nil })
        try await waitUntil("the page is shown") { !self.shell.view.outline.rows.isEmpty }
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
            backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: OutlineScreen(shell: shell))
        window.orderFrontRegardless()
        try await waitUntil("the editor of the item is there") { self.editors().count == 1 }
    }

    override func tearDown() async throws {
        window?.close()
        server?.stop()
    }

    private func waitUntil(_ what: String, _ done: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(10)
        while !done() {
            if Date() > deadline {
                XCTFail("never happened: \(what)")
                throw CancellationError()
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    /// The editors of the items, from the top of the window down.
    private func editors() -> [NSTextView] {
        var found: [NSTextView] = []
        func look(_ view: NSView) {
            if let editor = view as? NSTextView { found.append(editor) }
            view.subviews.forEach(look)
        }
        if let content = window.contentView { look(content) }
        return found.sorted {
            $0.convert($0.bounds, to: nil).maxY > $1.convert($1.bounds, to: nil).maxY
        }
    }

    /// The editor with the cursor, if an editor has it.
    private var focused: NSTextView? {
        window.firstResponder as? NSTextView
    }

    private func bodies() -> [String] {
        shell.view.outline.rows.map(\.item.body)
    }

    /// Puts the cursor into the editor and types.
    private func type(_ text: String, into editor: NSTextView) {
        window.makeFirstResponder(editor)
        editor.insertText(text, replacementRange: editor.selectedRange())
    }

    // 入力した文字は、コアに届く。
    func testWhatIsTypedReachesTheCore() async throws {
        type("typed 日本語", into: editors()[0])

        XCTAssertEqual(bodies(), ["typed 日本語"])
        XCTAssertEqual(shell.view.outline.focusedItemId, shell.view.outline.rows[0].item.id)
    }

    // 変換の途中の文字は、コアに届かない。確定すると届く。
    func testTextStillBeingComposedDoesNotReachTheCore() async throws {
        let editor = editors()[0]
        window.makeFirstResponder(editor)

        editor.setMarkedText("にほんご", selectedRange: NSRange(location: 4, length: 0),
            replacementRange: editor.selectedRange())
        XCTAssertEqual(bodies(), [""])
        editor.insertText("日本語", replacementRange: editor.markedRange())

        XCTAssertEqual(bodies(), ["日本語"])
    }

    // Enterは、カーソルの位置で項目を分ける。カーソルは、新しい項目の先頭に移る。
    func testEnterSplitsTheItemAndTheCursorGoesToTheNewOne() async throws {
        let editor = editors()[0]
        type("firstsecond", into: editor)
        editor.setSelectedRange(NSRange(location: 5, length: 0))

        editor.doCommand(by: #selector(NSResponder.insertNewline(_:)))

        try await waitUntil("there are two editors") { self.editors().map(\.string) == ["first", "second"] }
        try await waitUntil("the cursor is in the new item") { self.focused === self.editors()[1] }
        XCTAssertEqual(focused?.selectedRange(), NSRange(location: 0, length: 0))
        XCTAssertEqual(bodies(), ["first", "second"])
    }

    // 上矢印は、前の項目へ、同じ位置のまま移る。
    func testUpGoesToTheItemBeforeAtTheSamePlace() async throws {
        let editor = editors()[0]
        type("firstsecond", into: editor)
        editor.setSelectedRange(NSRange(location: 5, length: 0))
        editor.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        try await waitUntil("the cursor is in the new item") {
            self.editors().count == 2 && self.focused === self.editors()[1]
        }
        let second = editors()[1]
        second.setSelectedRange(NSRange(location: 3, length: 0))

        second.doCommand(by: #selector(NSResponder.moveUp(_:)))

        try await waitUntil("the cursor is in the first item") { self.focused === self.editors()[0] }
        XCTAssertEqual(focused?.selectedRange(), NSRange(location: 3, length: 0))
    }

    // Tabは、文字を入れずに、項目を字下げする。
    func testTabIndentsTheItemAndTypesNothing() async throws {
        let editor = editors()[0]
        type("firstsecond", into: editor)
        editor.setSelectedRange(NSRange(location: 5, length: 0))
        editor.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        try await waitUntil("the cursor is in the new item") {
            self.editors().count == 2 && self.focused === self.editors()[1]
        }
        let second = editors()[1]
        let left = second.convert(second.bounds, to: nil).minX

        second.doCommand(by: #selector(NSResponder.insertTab(_:)))

        try await waitUntil("the second editor moved to the right") {
            self.editors().count == 2 && self.editors()[1].convert(self.editors()[1].bounds, to: nil).minX > left
        }
        XCTAssertEqual(bodies(), ["first", "second"])
    }

    // 先頭でのBackspaceは、項目を前の項目につなぐ。カーソルは、つなぎ目に置かれる。
    func testBackspaceAtTheStartJoinsTheItems() async throws {
        let editor = editors()[0]
        type("firstsecond", into: editor)
        editor.setSelectedRange(NSRange(location: 5, length: 0))
        editor.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        try await waitUntil("the cursor is in the new item") {
            self.editors().count == 2 && self.focused === self.editors()[1]
        }

        editors()[1].doCommand(by: #selector(NSResponder.deleteBackward(_:)))

        try await waitUntil("there is one editor again") { self.editors().map(\.string) == ["firstsecond"] }
        try await waitUntil("the cursor is at the joint") {
            self.focused === self.editors()[0] && self.focused?.selectedRange().location == 5
        }
    }
}
