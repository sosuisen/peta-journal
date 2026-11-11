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
    /// What the shell put on the clipboard.
    private var copied: String?

    override func setUp() async throws {
        server = try JournalServer(port: 18094)
        shell = Shell(server: server.address, url: "/pages/screen", clipboard: { nil }, copy: { [weak self] in self?.copied = $0 })
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
    private func press(_ keyCode: UInt16, _ characters: String, _ modifiers: NSEvent.ModifierFlags, in editor: NSResponder) {
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

    /// Two items: the first has text in bold, and the cursor is in the second, which is empty.
    private func boldItemAndTheCursorInTheNext() async throws {
        let editor = editors()[0]
        type("**bold** text", into: editor)
        editor.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        try await waitUntil("there are two editors") { self.editors().count == 2 }
        try await waitUntil("the cursor is in the new item") { self.focused === self.editors()[1] }
    }

    private func isBold(_ editor: NSTextView, at place: Int) -> Bool {
        let font = editor.textStorage?.attribute(.font, at: place, effectiveRange: nil) as? NSFont
        return font?.fontDescriptor.symbolicTraits.contains(.bold) ?? false
    }

    // 編集していない項目は、装飾付きで出る。Markdownの記号は出ない。
    func testAnItemThatIsNotEditedIsShownWithItsDecoration() async throws {
        try await boldItemAndTheCursorInTheNext()

        try await waitUntil("the first item is shown without the signs of Markdown") {
            self.editors()[0].string == "bold text"
        }
        XCTAssertTrue(isBold(editors()[0], at: 0))
        XCTAssertFalse(isBold(editors()[0], at: 6))
        XCTAssertEqual(bodies(), ["**bold** text", ""])
    }

    // 装飾付きの項目にカーソルを入れると、Markdownに戻って編集できる。カーソルが出ると、また装飾付きで出る。
    func testAnItemThatGetsTheCursorIsMarkdownAgainAndIsDecoratedWhenTheCursorLeaves() async throws {
        try await boldItemAndTheCursorInTheNext()
        try await waitUntil("the first item is decorated") { self.editors()[0].string == "bold text" }

        window.makeFirstResponder(editors()[0])

        try await waitUntil("the first item is Markdown") { self.editors()[0].string == "**bold** text" }
        XCTAssertFalse(isBold(editors()[0], at: 2))
        let editor = editors()[0]
        editor.setSelectedRange(NSRange(location: (editor.string as NSString).length, length: 0))
        editor.insertText("!", replacementRange: editor.selectedRange())
        try await waitUntil("what is typed reaches the core") { self.bodies() == ["**bold** text!", ""] }

        window.makeFirstResponder(editors()[1])

        try await waitUntil("the first item is decorated again") { self.editors()[0].string == "bold text!" }
    }

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

    // タスクの前にも、バレットが出る。バレットのあとに、タスクの状態が並ぶ。
    func testATaskHasABulletBeforeItsState() async throws {
        let editor = editors()[0]
        type("to do", into: editor)
        XCTAssertEqual(shell.view.outline.rows[0].marks, ["•"])

        press(Self.returnKey, "\r", .control, in: editor)

        try await waitUntil("the item is a task") { self.shell.view.outline.rows[0].task == .notStarted }
        XCTAssertEqual(shell.view.outline.rows[0].marks, ["•", "☐"])
    }

    // 進行中のタスクは、横線の入った箱で出る。完了したタスクは、チェックの入った箱で出る。
    func testARunningTaskIsABoxWithALineAndADoneTaskABoxWithACheck() async throws {
        let editor = editors()[0]
        type("to do", into: editor)
        press(Self.returnKey, "\r", .control, in: editor)
        try await waitUntil("the item is a task") { self.shell.view.outline.rows[0].task == .notStarted }

        press(Self.returnKey, "\r", .control, in: editors()[0])
        try await waitUntil("the task is running") { self.shell.view.outline.rows[0].task == .running }
        XCTAssertEqual(shell.view.outline.rows[0].marks, ["•", "⊟"])

        press(Self.returnKey, "\r", .control, in: editors()[0])
        try await waitUntil("the task is done") { self.shell.view.outline.rows[0].task == .done }
        XCTAssertEqual(shell.view.outline.rows[0].marks, ["•", "☑"])
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

    private static let (escapeKey, backspaceKey, cKey): (UInt16, UInt16, UInt16) = (53, 51, 8)

    private func selected() -> [String] {
        shell.view.outline.rows.filter(\.isSelected).map(\.item.body)
    }

    /// Two items, with the second one selected by Shift+Up in its editor. The keys go to what
    /// is returned, as no editor has the cursor while items are selected.
    private func secondOfTwoSelected() async throws -> NSResponder {
        try await twoItems()
        editors()[1].doCommand(by: #selector(NSResponder.moveUpAndModifySelection(_:)))
        try await waitUntil("the second item is selected and no editor has the cursor") {
            self.selected() == ["second"] && self.focused == nil && self.window.firstResponder !== self.window
        }
        return try XCTUnwrap(window.firstResponder)
    }

    // Shift+上は、項目を選ぶ。もう一度押すと選択が広がり、Escapeで選択が消える。
    func testShiftUpSelectsItemsAndEscapeEndsTheSelection() async throws {
        let keys = try await secondOfTwoSelected()

        press(Self.upKey, "\u{F700}", .shift, in: keys)
        XCTAssertEqual(selected(), ["first", "second"])
        press(Self.escapeKey, "\u{1B}", [], in: keys)

        XCTAssertEqual(selected(), [])
        XCTAssertFalse(shell.view.outline.selectionMode)
    }

    // 選んでいるあいだのBackspaceは、選んだ項目を消す。
    func testBackspaceDeletesTheSelectedItems() async throws {
        let keys = try await secondOfTwoSelected()

        press(Self.backspaceKey, "\u{7F}", [], in: keys)

        try await waitUntil("the second item is gone") { self.bodies() == ["first"] }
    }

    // 選んでいるあいだのShift+Option+上は、選んだ項目を上へ動かす。
    func testShiftOptionUpMovesTheSelectedItems() async throws {
        let keys = try await secondOfTwoSelected()

        press(Self.upKey, "\u{F700}", [.shift, .option], in: keys)

        try await waitUntil("the second item is on top") { self.bodies() == ["second", "first"] }
    }

    // 選んでいるあいだのCommand+Cと、メニューの「コピー」は、選んだ項目をクリップボードに載せる。
    func testCommandCAndCopyOfTheMenuCopyTheSelectedItems() async throws {
        let keys = try await secondOfTwoSelected()

        press(Self.cKey, "c", .command, in: keys)
        try await waitUntil("the clipboard has the item") { self.copied == "- second" }
        copied = nil
        XCTAssertTrue(keys.tryToPerform(Selector(("copy:")), with: nil))

        try await waitUntil("the clipboard has the item again") { self.copied == "- second" }
    }

    // 選んでいるあいだに項目の文字をクリックすると、選択が消え、その項目に入力できる。
    func testTheCursorInAnEditorEndsTheSelection() async throws {
        _ = try await secondOfTwoSelected()
        let second = editors()[1]

        window.makeFirstResponder(second)
        second.insertText("!", replacementRange: NSRange(location: 6, length: 0))

        XCTAssertFalse(shell.view.outline.selectionMode)
        XCTAssertEqual(bodies(), ["first", "second!"])
        XCTAssertTrue(shell.view.outline.rows[1].isEditing)
    }

    /// The search box, when it is on screen.
    private func searchField() -> NSTextField? {
        var found: NSTextField?
        func look(_ view: NSView) {
            if let field = view as? NSTextField, field.accessibilityIdentifier() == "search" { found = field }
            view.subviews.forEach(look)
        }
        if let content = window.contentView { look(content) }
        return found
    }

    // 検索欄に入力すると、ページの候補が出る。下矢印で選び、Enterで開くと、検索欄は閉じる。
    func testTypingInTheSearchBoxFindsAPageAndEnterOpensIt() async throws {
        shell.open("/pages/apple-pie")
        try await waitUntil("the second page is shown") { self.shell.title == "apple-pie" }
        shell.open("/pages/screen")
        try await waitUntil("the first page is shown again") { self.shell.title == "screen" }
        window.contentView = NSHostingView(rootView: OutlineScreen(shell: shell, searching: true))
        try await waitUntil("the search box has the cursor") { self.searchField()?.currentEditor() != nil }
        let typing = try XCTUnwrap(searchField()?.currentEditor() as? NSTextView)

        typing.insertText("apple", replacementRange: typing.selectedRange())
        XCTAssertEqual(shell.view.search.query, "apple")
        try await waitUntil("the page is found") {
            self.shell.view.search.results.first?.title == "apple-pie"
        }
        typing.doCommand(by: #selector(NSResponder.moveDown(_:)))
        typing.doCommand(by: #selector(NSResponder.insertNewline(_:)))

        try await waitUntil("the page found is shown") { self.shell.title == "apple-pie" }
        try await waitUntil("the search box is closed") { self.searchField() == nil }
    }

    // 検索結果がウィンドウに入りきらないときも、検索欄はウィンドウの中にとどまる。
    func testTheSearchBoxStaysInsideTheWindowWhenTheResultsDoNotFit() async throws {
        for number in 1...30 {
            shell.open("/pages/berry-\(number)")
            try await waitUntil("page \(number) is shown") { self.shell.title == "berry-\(number)" }
        }
        window.contentView = NSHostingView(rootView: OutlineScreen(shell: shell, searching: true))
        try await waitUntil("the search box has the cursor") { self.searchField()?.currentEditor() != nil }
        let typing = try XCTUnwrap(searchField()?.currentEditor() as? NSTextView)

        typing.insertText("berry", replacementRange: typing.selectedRange())
        try await waitUntil("the pages are found") { self.shell.view.search.results.count > 13 }
        // The screen follows the core a moment later
        try await Task.sleep(nanoseconds: 300_000_000)

        let field = try XCTUnwrap(searchField())
        XCTAssertLessThanOrEqual(field.convert(field.bounds, to: nil).maxY, window.contentLayoutRect.maxY)
    }

    // ページのない画面には、検索欄が最初から出ていて、カーソルがそこにある。
    func testAScreenWithoutAPageStartsWithTheSearchBox() async throws {
        let empty = Shell(server: server.address, url: nil, clipboard: { nil })

        window.contentView = NSHostingView(rootView: OutlineScreen(shell: empty))

        try await waitUntil("the search box has the cursor") { self.searchField()?.currentEditor() != nil }
        XCTAssertEqual(editors().filter { $0 is NSTextView && !$0.isFieldEditor }.count, 0)
    }

    // ズームすると、項目の文字が大きくなる。等倍に戻すと、元の大きさに戻る。
    func testTheZoomMakesTheTextOfTheItemsLarger() async throws {
        let look = StickyLook()
        window.contentView = NSHostingView(rootView: OutlineScreen(shell: shell, look: look))
        try await waitUntil("the editor of the item is there") { self.editors().count == 1 }
        let size = NSFont.systemFontSize
        XCTAssertEqual(editors()[0].font?.pointSize, size)
        type("zoomed", into: editors()[0])
        let height = editors()[0].frame.height

        for _ in 0..<10 { look.zoomIn() }

        try await waitUntil("the text is twice as large") { self.editors()[0].font?.pointSize == size * 2 }
        try await waitUntil("the editor is higher") { self.editors()[0].frame.height > height * 1.5 }
        XCTAssertEqual(bodies(), ["zoomed"])
        look.resetZoom()
        try await waitUntil("the text is as large as before") { self.editors()[0].font?.pointSize == size }
    }
}
