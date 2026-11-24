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
    /// The text on the clipboard, which a paste reads.
    private var pasteboard: String?
    /// The file that the image on the clipboard was written to, which a paste reads first.
    private var pastedImage: URL?
    /// What the shell opened in the browser.
    private var browsed: URL?
    /// What the shell asked a new sticky for.
    private var askedSticky: String?

    override func setUp() async throws {
        server = try JournalServer(port: 18094)
        shell = Shell(
            server: server.address, url: "/pages/screen", clipboard: { [weak self] in self?.pasteboard },
            image: { [weak self] in self?.pastedImage },
            copy: { [weak self] text, _ in self?.copied = text },
            browse: { [weak self] in self?.browsed = $0 },
            openSticky: { [weak self] in self?.askedSticky = $0 })
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
        // A click on a link takes no events after its own: the button that comes up would end
        // the drag of a later test. `NSApp` is nil when a skipped setUp never made the window
        while NSApplication.shared.nextEvent(
            matching: [.leftMouseUp, .leftMouseDragged], until: .distantPast, inMode: .default, dequeue: true) != nil
        {}
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

    /// Two items: the first has links and is shown with its decoration, and the cursor is in
    /// the second, which is empty.
    private func itemWithLinksAndTheCursorInTheNext() async throws {
        let editor = editors()[0]
        type("[[Idea]] #todo [out](https://example.com/a) plain", into: editor)
        editor.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        try await waitUntil("there are two editors") { self.editors().count == 2 }
        try await waitUntil("the cursor is in the new item") { self.focused === self.editors()[1] }
        try await waitUntil("the first item is decorated") { self.editors()[0].string == "Idea #todo out plain" }
    }

    /// A click on the first letter of the word. The window of a test takes no click: the tests
    /// do not come to the front. So this does what a window does with one: it asks which view
    /// is there, gives it the cursor if the view takes it, and then hands it the click.
    private func click(on word: String, in editor: NSTextView, holding keys: NSEvent.ModifierFlags = []) throws {
        let letter = NSRange(location: (editor.string as NSString).range(of: word).location, length: 1)
        let layout = try XCTUnwrap(editor.layoutManager)
        let container = try XCTUnwrap(editor.textContainer)
        let glyphs = layout.glyphRange(forCharacterRange: letter, actualCharacterRange: nil)
        let box = layout.boundingRect(forGlyphRange: glyphs, in: container)
        let origin = editor.textContainerOrigin
        let place = editor.convert(NSPoint(x: origin.x + box.midX, y: origin.y + box.midY), to: nil)
        func mouse(_ type: NSEvent.EventType) throws -> NSEvent {
            try XCTUnwrap(
                NSEvent.mouseEvent(
                    with: type, location: place, modifierFlags: keys, timestamp: 0,
                    windowNumber: window.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: 1))
        }
        // A text view waits for the button to come up before it goes on
        NSApp.postEvent(try mouse(.leftMouseUp), atStart: false)
        let hit = try XCTUnwrap(window.contentView?.hitTest(place))
        if hit.acceptsFirstResponder { window.makeFirstResponder(hit) }
        hit.mouseDown(with: try mouse(.leftMouseDown))
    }

    // 装飾付きの項目のウィキリンクをクリックすると、そのページが付箋紙で開く。
    func testAClickOnAWikilinkOpensItsPageInTheSticky() async throws {
        try await itemWithLinksAndTheCursorInTheNext()

        try click(on: "Idea", in: editors()[0])

        try await waitUntil("the page opens") { self.shell.url == "/pages/Idea" }
        XCTAssertNil(browsed)
    }

    // 装飾付きの項目のウィキリンクをShift-クリックすると、そのページがブラウザ版で開く。付箋紙のページは変わらない。
    func testAShiftClickOnAWikilinkOpensItsPageInTheBrowser() async throws {
        try await itemWithLinksAndTheCursorInTheNext()

        try click(on: "Idea", in: editors()[0], holding: .shift)

        let address = URL(string: "\(server.address)/pages/Idea")
        try await waitUntil("the page opens in the browser") { self.browsed == address }
        XCTAssertEqual(shell.url, "/pages/screen")
    }

    // 装飾付きの項目のウィキリンクをCommand-クリックすると、そのページを開く新しい付箋紙を頼む。この付箋紙のページは変わらない。
    func testACommandClickOnAWikilinkAsksForANewStickyWithItsPage() async throws {
        try await itemWithLinksAndTheCursorInTheNext()

        try click(on: "Idea", in: editors()[0], holding: .command)

        try await waitUntil("a new sticky is asked for") { self.askedSticky == "/pages/Idea" }
        XCTAssertEqual(shell.url, "/pages/screen")
        XCTAssertNil(browsed)
    }

    // 装飾付きの項目のハッシュタグをクリックすると、タグのフォルダーにあるそのタグのページが付箋紙で開く。
    func testAClickOnAHashtagOpensThePageOfTheTagInTheSticky() async throws {
        try await itemWithLinksAndTheCursorInTheNext()
        let tags = shell.view.config.tagsDir

        try click(on: "#todo", in: editors()[0])

        try await waitUntil("the page of the tag opens") { self.shell.url == "/pages/\(tags)todo" }
        XCTAssertNil(browsed)
    }

    // 装飾付きの項目のハッシュタグをShift-クリックすると、そのタグのページがブラウザ版で開く。付箋紙のページは変わらない。
    func testAShiftClickOnAHashtagOpensThePageOfTheTagInTheBrowser() async throws {
        try await itemWithLinksAndTheCursorInTheNext()

        try click(on: "#todo", in: editors()[0], holding: .shift)

        let address = URL(string: "\(server.address)/pages/\(shell.view.config.tagsDir)todo")
        try await waitUntil("the page of the tag opens in the browser") { self.browsed == address }
        XCTAssertEqual(shell.url, "/pages/screen")
    }

    // 装飾付きの項目の外へのリンクをクリックすると、ブラウザで開く。項目は編集にならず、ページもそのままである。
    func testAClickOnALinkThatLeadsOutOpensItInTheBrowser() async throws {
        try await itemWithLinksAndTheCursorInTheNext()

        try click(on: "out", in: editors()[0])

        try await waitUntil("the link opens in the browser") { self.browsed == URL(string: "https://example.com/a") }
        XCTAssertEqual(shell.url, "/pages/screen")
        XCTAssertEqual(editors()[0].string, "Idea #todo out plain")
        XCTAssertTrue(focused === editors()[1])
    }

    // 装飾付きの項目の、リンクでない文字をクリックすると、その項目がMarkdownになってカーソルが入る。
    func testAClickBesideTheLinksPutsTheCursorInTheItem() async throws {
        try await itemWithLinksAndTheCursorInTheNext()

        try click(on: "plain", in: editors()[0])

        XCTAssertTrue(focused === editors()[0])
        XCTAssertEqual(editors()[0].string, "[[Idea]] #todo [out](https://example.com/a) plain")
        XCTAssertNil(browsed)
        XCTAssertEqual(shell.url, "/pages/screen")
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

        editor.setMarkedText(
            "にほんご", selectedRange: NSRange(location: 4, length: 0),
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

    // Enterのすぐあと、項目が分かれる前に打った文字は、新しい項目に入る。
    func testWhatIsTypedRightAfterEnterGoesIntoTheNewItem() async throws {
        let editor = editors()[0]
        type("first", into: editor)

        editor.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        // The split has not been answered yet: the old editor still has the cursor
        XCTAssertTrue(focused === editor)
        editor.insertText("x", replacementRange: editor.selectedRange())

        try await waitUntil("the letter is in the new item") { self.bodies() == ["first", "x"] }
        try await waitUntil("the cursor is in the new item") { self.focused === self.editors()[1] }
        XCTAssertEqual(editors().map(\.string), ["first", "x"])
        XCTAssertEqual(focused?.selectedRange(), NSRange(location: 1, length: 0))
    }

    // 項目が分かれる前に打ったEnterも、新しい項目で押したことになる。その後の文字は、さらに次の項目に入る。
    func testAnEnterTypedBeforeTheSplitSplitsTheNewItem() async throws {
        let editor = editors()[0]
        type("first", into: editor)

        editor.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        editor.insertText("x", replacementRange: editor.selectedRange())
        press(36, "\r", [], in: editor)
        editor.insertText("y", replacementRange: editor.selectedRange())

        try await waitUntil("the letters are in two new items") { self.bodies() == ["first", "x", "y"] }
        try await waitUntil("the cursor is in the last item") { self.focused === self.editors().last }
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

    // Command+Enterは、項目をタスクにし、押すたびに状態を進める。Control+Enterも同じである。
    func testCommandEnterMakesTheItemATaskAndStepsItsState() async throws {
        let editor = editors()[0]
        type("to do", into: editor)

        press(36, "\r", [.command], in: editor)
        try await waitUntil("the item is a task") { self.shell.view.outline.rows[0].task == .notStarted }
        press(36, "\r", [.command], in: editor)
        try await waitUntil("the task is running") { self.shell.view.outline.rows[0].task == .running }
        press(36, "\r", [.control], in: editor)
        try await waitUntil("the task is done") { self.shell.view.outline.rows[0].task == .done }
        XCTAssertEqual(bodies(), ["to do"])
    }

    // Command+Oは、カーソルのあるウィキリンクのページを、付箋紙で開く。
    func testCommandOOpensThePageOfTheLinkAtTheCursor() async throws {
        let editor = editors()[0]
        type("see [[Idea]]", into: editor)
        editor.setSelectedRange(NSRange(location: 8, length: 0))

        press(Self.oKey, "o", .command, in: editor)

        try await waitUntil("the page of the link is open") { self.shell.url == "/pages/Idea" }
    }

    // 本文が見出しの1行である項目は、編集中も見出しの大きさの太字で出る。Markdownの記号はそのまま見える。見出しでなくなると、ふつうの文字に戻る。
    func testAnItemThatIsOneHeadingLineIsEditedAtTheSizeOfItsHeading() async throws {
        let editor = editors()[0]
        let usual = NSFont.systemFontSize

        type("## Title", into: editor)

        try await waitUntil("the editor has the size of a heading of the second level") {
            abs((editor.font?.pointSize ?? 0) - usual * 1.4) < 0.01
        }
        XCTAssertTrue(isBold(editor, at: 0))
        XCTAssertTrue(isBold(editor, at: 5))
        XCTAssertEqual(editor.string, "## Title")
        XCTAssertTrue(focused === editor)

        editor.insertText(" more", replacementRange: editor.selectedRange())
        XCTAssertTrue(isBold(editor, at: 10))

        editor.setSelectedRange(NSRange(location: 0, length: 3))
        editor.insertText("", replacementRange: editor.selectedRange())
        try await waitUntil("the editor has the usual size again") {
            abs((editor.font?.pointSize ?? 0) - usual) < 0.01
        }
        XCTAssertFalse(isBold(editor, at: 0))
        XCTAssertEqual(bodies(), ["Title more"])
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
    private func press(
        _ keyCode: UInt16, _ characters: String, _ modifiers: NSEvent.ModifierFlags, in editor: NSResponder
    ) {
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

    private static let (returnKey, downKey, upKey, zKey, oKey): (UInt16, UInt16, UInt16, UInt16, UInt16) =
        (36, 125, 126, 6, 31)

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

    /// The color of a letter, as numbers that can be compared.
    private func color(_ editor: NSTextView, at place: Int) -> [CGFloat]? {
        let color = editor.textStorage?.attribute(.foregroundColor, at: place, effectiveRange: nil) as? NSColor
        guard let seen = color?.usingColorSpace(.sRGB) else { return nil }
        return [seen.redComponent, seen.greenComponent, seen.blueComponent, seen.alphaComponent]
            .map { ($0 * 100).rounded() / 100 }
    }

    // 装飾付きの項目の文字と、編集中の項目の文字は、同じ色である。
    func testTheTextOfAShownItemHasTheColorOfTheTextThatIsEdited() async throws {
        try await boldItemAndTheCursorInTheNext()
        try await waitUntil("the first item is decorated") { self.editors()[0].string == "bold text" }
        let edited = editors()[1]
        edited.insertText("typed", replacementRange: edited.selectedRange())

        XCTAssertNotNil(color(edited, at: 0))
        XCTAssertEqual(color(editors()[0], at: 0), color(edited, at: 0))
        XCTAssertEqual(color(editors()[0], at: 6), color(edited, at: 0))
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

    private static let vKey: UInt16 = 9

    /// Two items, "first" and "second", with the cursor at the end of the second.
    private func cursorAtTheEndOfTheSecondItem() async throws -> NSTextView {
        try await twoItems()
        let second = editors()[1]
        second.setSelectedRange(NSRange(location: 6, length: 0))
        return second
    }

    // メニューの「ペースト」は、Markdownのリストを項目に分けて、カーソルの位置に貼る。
    func testPasteOfTheMenuCutsAMarkdownListIntoItems() async throws {
        let second = try await cursorAtTheEndOfTheSecondItem()
        pasteboard = "- apple\n- pear"

        XCTAssertTrue(second.tryToPerform(Selector(("paste:")), with: nil))

        try await waitUntil("the list is two items") { self.bodies() == ["first", "secondapple", "pear"] }
    }

    // メニューの「ペースト」（Command+V）は、クリップボードの画像をサーバーのassetsに保存し、カーソルの位置にそのリンクを入れる。
    func testPasteOfTheMenuPutsALinkToTheImageOnTheClipboardAtTheCursor() async throws {
        let second = try await cursorAtTheEndOfTheSecondItem()
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("peta-test-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        pastedImage = folder.appendingPathComponent("sample.png")
        try Data([1, 2, 3]).write(to: try XCTUnwrap(pastedImage))
        pasteboard = "not this"

        XCTAssertTrue(second.tryToPerform(Selector(("paste:")), with: nil))

        try await waitUntil("the second item links the image") {
            let bodies = self.bodies()
            return bodies.count == 2 && bodies[1].hasPrefix("second![sample.png](../assets/")
        }
    }

    // Shift+Command+Vは、クリップボードの文字をそのまま項目に貼る。
    func testShiftCommandVPutsTheTextIntoTheItemAsItIs() async throws {
        let second = try await cursorAtTheEndOfTheSecondItem()
        pasteboard = "- apple\n- pear"

        press(Self.vKey, "v", [.shift, .command], in: second)

        try await waitUntil("the text is in the item") { self.bodies() == ["first", "second- apple\n- pear"] }
    }

    // メニューの「ペーストしてスタイルを合わせる」も、文字をそのまま項目に貼る。
    func testPasteAndMatchStyleOfTheMenuPutsTheTextIntoTheItemAsItIs() async throws {
        let second = try await cursorAtTheEndOfTheSecondItem()
        pasteboard = "- apple\n- pear"

        XCTAssertTrue(second.tryToPerform(Selector(("pasteAsPlainText:")), with: nil))

        try await waitUntil("the text is in the item") { self.bodies() == ["first", "second- apple\n- pear"] }
    }

    // 貼る位置は、絵文字のような2つのUTF-16単位の文字の後でも、カーソルの位置である。
    func testAPasteAfterAnEmojiGoesToTheCursor() async throws {
        let editor = editors()[0]
        type("😀ab", into: editor)
        editor.setSelectedRange(NSRange(location: 3, length: 0))
        pasteboard = "x"

        XCTAssertTrue(editor.tryToPerform(Selector(("paste:")), with: nil))

        try await waitUntil("the text is after the a") { self.bodies() == ["😀axb"] }
    }

    /// The level and the body of each item, as "level body".
    private func levelsAndBodies() -> [String] {
        shell.view.outline.rows.map { "\($0.item.level) \($0.item.body)" }
    }

    /// Three items: "parent", its child "child", and "third" after them, with the cursor at
    /// the end of "third".
    private func parentChildAndThird() async throws {
        type("parent", into: editors()[0])
        editors()[0].doCommand(by: #selector(NSResponder.insertNewline(_:)))
        try await waitUntil("the cursor is in the second item") {
            self.editors().count == 2 && self.focused === self.editors()[1]
        }
        let child = editors()[1]
        type("child", into: child)
        child.doCommand(by: #selector(NSResponder.insertTab(_:)))
        try await waitUntil("the second item is a child") { self.levelsAndBodies() == ["1 parent", "2 child"] }
        child.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        try await waitUntil("the cursor is in the third item") {
            self.editors().count == 3 && self.focused === self.editors()[2]
        }
        editors()[2].doCommand(by: #selector(NSResponder.insertBacktab(_:)))
        try await waitUntil("the third item is not a child") {
            self.levelsAndBodies() == ["1 parent", "2 child", "1 "]
        }
        type("third", into: editors()[2])
    }

    // 親と子を選んでコピーし、別の項目の終わりに貼り付けると、親と子の形のまま戻る。
    func testAParentAndItsChildCopiedAndPastedAtTheEndOfAnotherItemComeBackInTheirShape() async throws {
        try await parentChildAndThird()
        let parent = editors()[0]
        window.makeFirstResponder(parent)
        parent.setSelectedRange(NSRange(location: 0, length: 0))
        parent.doCommand(by: #selector(NSResponder.moveUpAndModifySelection(_:)))
        try await waitUntil("the parent and its child are selected and no editor has the cursor") {
            self.selected() == ["parent", "child"] && self.focused == nil && self.window.firstResponder !== self.window
        }
        press(Self.cKey, "c", .command, in: try XCTUnwrap(window.firstResponder))
        try await waitUntil("the clipboard has the items") { self.copied != nil }
        // The system clipboard gives a paste what the copy put on it
        pasteboard = copied
        let third = editors()[2]
        window.makeFirstResponder(third)
        third.setSelectedRange(NSRange(location: 5, length: 0))

        XCTAssertTrue(third.tryToPerform(Selector(("paste:")), with: nil))

        try await waitUntil("the parent and its child are pasted") {
            self.levelsAndBodies() == ["1 parent", "2 child", "1 thirdparent", "2 child"]
        }
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

    /// Three items, "first", "second" and "third", with the cursor at the end of the third.
    private func threeItems() async throws {
        try await twoItems()
        let second = editors()[1]
        second.setSelectedRange(NSRange(location: (second.string as NSString).length, length: 0))
        second.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        try await waitUntil("the cursor is in the third item") {
            self.editors().count == 3 && self.focused === self.editors()[2]
        }
        type("third", into: editors()[2])
    }

    /// The middle of the view, in the window.
    private func middle(of view: NSView) -> NSPoint {
        view.convert(NSPoint(x: view.bounds.midX, y: view.bounds.midY), to: nil)
    }

    private func mouse(_ type: NSEvent.EventType, at place: NSPoint) throws -> NSEvent {
        try XCTUnwrap(
            NSEvent.mouseEvent(
                with: type, location: place, modifierFlags: [], timestamp: 0,
                windowNumber: window.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: 1))
    }

    /// A drag that starts in the middle of the editor, goes through the places of the window
    /// and lets the button go at the last. The moves are handed over as the window does with
    /// a click: they wait in the queue of the window for the editor that tracks the mouse.
    /// The button comes up after `holding` seconds at the last place.
    private func drag(from editor: NSTextView, through places: [NSPoint], holding: TimeInterval = 0) throws {
        let start = middle(of: editor)
        for place in places {
            NSApp.postEvent(try mouse(.leftMouseDragged, at: place), atStart: false)
        }
        let up = try mouse(.leftMouseUp, at: places.last ?? start)
        if holding > 0 {
            // A timer and not the main queue: the test runs on the main queue, which waits for
            // the editor to let the mouse go. The run loop fires timers while the editor waits
            let timer = Timer(timeInterval: holding, repeats: false) { _ in NSApp.postEvent(up, atStart: false) }
            RunLoop.main.add(timer, forMode: .common)
        } else {
            NSApp.postEvent(up, atStart: false)
        }
        if window.firstResponder !== editor { window.makeFirstResponder(editor) }
        editor.mouseDown(with: try mouse(.leftMouseDown, at: start))
    }

    // 項目のテキストから上へ出たドラッグは、その項目を選び、マウスの下の項目まで選択を広げる。打っていた文字は保存され、選んだ項目を動かしても消えない。
    func testADragOutOfTheTextOfAnItemSelectsTheItemsItGoesOver() async throws {
        try await threeItems()
        let (second, third) = (editors()[1], editors()[2])

        try drag(from: third, through: [middle(of: second)])

        try await waitUntil("the second and the third item are selected") {
            self.selected() == ["second", "third"] && self.window.firstResponder !== third
        }
        let keys = try XCTUnwrap(window.firstResponder)
        press(Self.upKey, "\u{F700}", [.shift, .option], in: keys)
        try await waitUntil("the selected items are on top") { self.bodies() == ["second", "third", "first"] }
    }

    // 項目のテキストの中のドラッグは、押した位置からマウスまでの文字を選び、項目は選ばない。
    func testADragInsideTheTextOfAnItemSelectsItsLetters() async throws {
        try await threeItems()
        let third = editors()[2]
        let start = third.convert(NSPoint(x: third.bounds.minX + 1, y: third.bounds.midY), to: nil)

        // From the middle of the editor, which is after the end of "third", to its start
        try drag(from: third, through: [start])

        XCTAssertEqual(third.selectedRange(), NSRange(location: 0, length: 5))
        XCTAssertFalse(shell.view.outline.selectionMode)
        XCTAssertTrue(focused === third)
    }

    /// The page gets `count` items more than the window can show, "item 1" and on, and is
    /// opened again in the window.
    private func longPage(_ count: Int = 40) async throws {
        guard case .page(let page?) = shell.view.screen else { throw CancellationError() }
        for n in 1...count {
            try await server.post(
                "/api/items", ["pageId": page.page.id, "position": "last", "level": 1, "body": "item \(n)"])
        }
        shell = Shell(server: server.address, url: "/pages/screen", clipboard: { nil })
        window.contentView = NSHostingView(rootView: OutlineScreen(shell: shell))
        try await waitUntil("the long page is shown") {
            self.shell.view.outline.rows.count == count + 1 && self.editors().count > 2
        }
    }

    /// The table of the list of the items, and the scroll view that holds it.
    private func list() throws -> (NSScrollView, NSTableView) {
        var found: NSTableView?
        func look(_ view: NSView) {
            if found == nil, let table = view as? NSTableView { found = table }
            view.subviews.forEach(look)
        }
        if let content = window.contentView { look(content) }
        let table = try XCTUnwrap(found)
        return (try XCTUnwrap(table.enclosingScrollView), table)
    }

    // ドラッグ選択のマウスを付箋の下端で止めると、付箋が下へスクロールし、下から出てきた項目まで選ばれる。
    func testADragSelectionHeldAtTheBottomScrollsDownAndSelectsWhatComesUp() async throws {
        try await longPage()
        let (scroll, table) = try list()
        let lastSeen = table.rows(in: table.visibleRect).upperBound - 1
        let first = editors()[1]
        // In the window, where up is up
        let (text, shown) = (first.convert(first.bounds, to: nil), scroll.convert(scroll.bounds, to: nil))
        let below = NSPoint(x: text.midX, y: text.minY - 15)
        let bottom = NSPoint(x: shown.midX, y: shown.minY + 5)

        try drag(from: first, through: [below, bottom], holding: 0.5)

        XCTAssertGreaterThan(scroll.contentView.bounds.minY, 0)
        let lastSelected = try XCTUnwrap(shell.view.outline.rows.lastIndex(where: \.isSelected))
        XCTAssertGreaterThan(lastSelected, lastSeen)
        XCTAssertFalse(shell.view.outline.rows[0].isSelected)
    }

    /// A long page, scrolled to its end. The scroll view of the list, how far it is scrolled,
    /// the first row that can be seen, and the editor of the last item.
    private func longPageAtItsEnd() async throws -> (NSScrollView, CGFloat, Int, NSTextView) {
        try await longPage()
        let (scroll, table) = try list()
        let clip = scroll.contentView
        let end = (scroll.documentView?.frame.height ?? 0) - clip.bounds.height
        clip.scroll(to: NSPoint(x: 0, y: end))
        scroll.reflectScrolledClipView(clip)
        try await waitUntil("the last item can be seen") {
            self.editors().contains { $0.string == "item 40" && !$0.visibleRect.isEmpty }
        }
        let firstSeen = table.rows(in: table.visibleRect).lowerBound
        return (scroll, end, firstSeen, try XCTUnwrap(editors().first { $0.string == "item 40" }))
    }

    // 付箋の下までスクロールしてから、ドラッグ選択のマウスを上端で止めると、付箋が上へスクロールし、上から出てきた項目まで選ばれる。
    func testADragSelectionHeldAtTheTopScrollsUpAndSelectsWhatComesDown() async throws {
        let (scroll, end, firstSeen, last) = try await longPageAtItsEnd()
        let clip = scroll.contentView
        let (text, shown) = (last.convert(last.bounds, to: nil), scroll.convert(scroll.bounds, to: nil))
        let above = NSPoint(x: text.midX, y: text.maxY + 15)
        let top = NSPoint(x: shown.midX, y: shown.maxY - 5)

        try drag(from: last, through: [above, top], holding: 0.5)

        XCTAssertLessThan(clip.bounds.minY, end)
        let firstSelected = try XCTUnwrap(shell.view.outline.rows.firstIndex(where: \.isSelected))
        XCTAssertLessThan(firstSelected, firstSeen)
        XCTAssertTrue(shell.view.outline.rows[40].isSelected)
    }

    // ドラッグ選択のマウスが付箋の上の外へ出ると、見えているいちばん上の項目まで選ばれる。まだ見えていない項目は選ばれない。
    func testADragSelectionAboveTheStickyReachesTheTopItemThatCanBeSeen() async throws {
        let (scroll, _, firstSeen, last) = try await longPageAtItsEnd()
        let (text, shown) = (last.convert(last.bounds, to: nil), scroll.convert(scroll.bounds, to: nil))
        let above = NSPoint(x: text.midX, y: text.maxY + 15)
        let beyond = NSPoint(x: shown.midX, y: shown.maxY + 200)

        // The button comes up at once: no step of the scroll comes between
        try drag(from: last, through: [above, beyond])

        XCTAssertEqual(shell.view.outline.rows.firstIndex(where: \.isSelected), firstSeen)
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
