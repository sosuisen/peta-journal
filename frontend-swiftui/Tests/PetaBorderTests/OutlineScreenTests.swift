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

    /// Presses a key of the keyboard in the editor, as AppKit hands it over.
    private func press(_ keyCode: UInt16, _ characters: String, _ modifiers: NSEvent.ModifierFlags, in editor: NSTextView) {
        let event = NSEvent.keyEvent(
            with: .keyDown, location: .zero, modifierFlags: modifiers, timestamp: 0,
            windowNumber: window.windowNumber, context: nil, characters: characters,
            charactersIgnoringModifiers: characters, isARepeat: false, keyCode: keyCode)!
        editor.keyDown(with: event)
    }

    /// Two items, "first" and "second", with the cursor in the second.
    private func twoItems() async throws {
        let editor = editors()[0]
        type("firstsecond", into: editor)
        editor.setSelectedRange(NSRange(location: 5, length: 0))
        editor.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        try await waitUntil("the cursor is in the new item") {
            self.editors().count == 2 && self.focused === self.editors()[1]
        }
    }

    private static let (returnKey, downKey, upKey, zKey): (UInt16, UInt16, UInt16, UInt16) = (36, 125, 126, 6)

    // Shift+Enterは、項目を分けずに、項目の中で改行する。
    func testShiftEnterBreaksTheLineInsideTheItem() async throws {
        let editor = editors()[0]
        type("ab", into: editor)
        editor.setSelectedRange(NSRange(location: 1, length: 0))

        press(Self.returnKey, "\r", .shift, in: editor)

        XCTAssertEqual(bodies(), ["a\nb"])
    }

    // Control+Enterは、項目をタスクにする。文字は入らない。
    func testControlEnterMakesTheItemATask() async throws {
        let editor = editors()[0]
        type("to do", into: editor)

        press(Self.returnKey, "\r", .control, in: editor)

        try await waitUntil("the item is a task") { self.shell.view.outline.rows[0].task == .notStarted }
        XCTAssertEqual(bodies(), ["to do"])
    }

    // Shift+Option+下矢印は、項目を下へ動かす。カーソルは、その項目に残る。
    func testShiftOptionDownMovesTheItemDown() async throws {
        try await twoItems()
        let first = editors()[0]
        window.makeFirstResponder(first)

        press(Self.downKey, "\u{F701}", [.shift, .option], in: first)

        try await waitUntil("the first item is below") { self.bodies() == ["second", "first"] }
        try await waitUntil("the cursor is still in it") { self.focused?.string == "first" }
    }

    // Command+上矢印は子の項目を隠し、Command+下矢印は表示する。カーソルは、文の中で動かない。
    func testCommandUpHidesTheChildrenAndCommandDownShowsThem() async throws {
        try await twoItems()
        editors()[1].doCommand(by: #selector(NSResponder.insertTab(_:)))
        let first = editors()[0]
        window.makeFirstResponder(first)
        first.setSelectedRange(NSRange(location: 3, length: 0))

        press(Self.upKey, "\u{F700}", .command, in: first)
        try await waitUntil("only the parent is shown") { self.editors().map(\.string) == ["first"] }
        press(Self.downKey, "\u{F701}", .command, in: first)
        try await waitUntil("the child is shown again") { self.editors().map(\.string) == ["first", "second"] }

        XCTAssertEqual(first.selectedRange(), NSRange(location: 3, length: 0))
    }

    // 末尾でのDeleteは、次の項目をつなぐ。
    func testDeleteAtTheEndJoinsTheNextItem() async throws {
        try await twoItems()
        let first = editors()[0]
        window.makeFirstResponder(first)
        first.setSelectedRange(NSRange(location: 5, length: 0))

        first.doCommand(by: #selector(NSResponder.deleteForward(_:)))

        try await waitUntil("there is one editor again") { self.editors().map(\.string) == ["firstsecond"] }
    }

    // メニューの「取り消す」とControl+Zは、コアの操作を元に戻す。Shiftを足すと、やり直す。
    func testUndoOfTheMenuAndControlZTakeBackAChangeOfTheOutline() async throws {
        try await twoItems()
        let second = editors()[1]
        let level = shell.view.outline.rows[1].item.level
        second.doCommand(by: #selector(NSResponder.insertTab(_:)))
        XCTAssertEqual(shell.view.outline.rows[1].item.level, level + 1)

        XCTAssertTrue(second.tryToPerform(Selector(("undo:")), with: nil))
        try await waitUntil("the indent is taken back") { self.shell.view.outline.rows[1].item.level == level }
        press(Self.zKey, "z", [.control, .shift], in: second)
        try await waitUntil("the indent is made again") {
            self.shell.view.outline.rows[1].item.level == level + 1
        }
        press(Self.zKey, "z", .control, in: second)
        try await waitUntil("the indent is taken back again") {
            self.shell.view.outline.rows[1].item.level == level
        }
    }
}
