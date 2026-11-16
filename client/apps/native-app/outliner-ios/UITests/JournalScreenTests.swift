import UIKit
import XCTest

/// The screen of the app in the simulator, with the journal inside the app. Each test gives
/// the app a folder of its own, so the pages of whoever runs the tests are never touched. The
/// app syncs with nothing here: the sync has tests of its own (`swift test`).
final class JournalScreenTests: XCTestCase {
    private var root: URL!
    private var app: XCUIApplication!

    private static let wait: TimeInterval = 15

    override func setUpWithError() throws {
        continueAfterFailure = false
        root = FileManager.default.temporaryDirectory.appendingPathComponent("peta-\(UUID().uuidString)")
        for name in ["docs", "journals", "assets"] {
            try FileManager.default.createDirectory(
                at: root.appendingPathComponent("markdown/\(name)"), withIntermediateDirectories: true)
        }
    }

    override func tearDownWithError() throws {
        app?.terminate()
        try? FileManager.default.removeItem(at: root)
    }

    // MARK: - Helpers

    /// Puts a page into the Markdown folder, as a download does.
    private func download(_ name: String, _ text: String) throws {
        try Data(text.utf8).write(to: root.appendingPathComponent("markdown/docs/\(name)"))
    }

    /// The line above an item that gives it an ID, so that the core does not write the file
    /// again to give it one.
    private func above(_ id: Character, _ properties: String = "") -> String {
        "<!-- @ 2020-01-01-00-00-00-\(String(repeating: String(id), count: 7)) idx=\"a\(id)\"\(properties) -->\n"
    }

    /// Starts the app on the folder of the test. `opening` is the URL of a page that the app
    /// opens at once.
    private func start(opening url: String? = nil) {
        app = XCUIApplication()
        app.launchEnvironment["PETA_ROOT"] = root.path
        if let url { app.launchEnvironment["PETA_OPEN"] = url }
        app.launch()
    }

    private var search: XCUIElement { app.textFields["search"] }
    private var editor: XCUIElement { app.textViews["editor"] }

    /// Something on the screen that shows the text, or a part of it.
    private func shown(_ text: String) -> XCUIElement {
        app.descendants(matching: .any).matching(NSPredicate(format: "label CONTAINS %@", text)).firstMatch
    }

    private func waitFor(_ element: XCUIElement, file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertTrue(element.waitForExistence(timeout: JournalScreenTests.wait), "\(element) is not shown", file: file, line: line)
    }

    private func waitUntil(_ what: String, file: StaticString = #filePath, line: UInt = #line, _ holds: () -> Bool) {
        let end = Date().addingTimeInterval(JournalScreenTests.wait)
        while !holds() {
            if Date() > end { return XCTFail("Not in time: \(what)", file: file, line: line) }
            usleep(100_000)
        }
    }

    /// Starts the app on the page `notes`. A test of something else than the search does not
    /// go through the search field: the keyboard is slow to come up in the simulator, and now
    /// and then the field does not get it.
    private func openNotes() {
        start(opening: "/pages/notes")
        waitFor(app.staticTexts["notes"])
    }

    /// Searches for the page `notes` and opens it.
    private func searchNotes() {
        start()
        waitFor(search)
        search.tap()
        search.typeText("notes")
        let result = app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'notes'")).firstMatch
        waitFor(result)
        result.tap()
        waitFor(app.staticTexts["notes"])
    }

    /// Ends the edit, so that every item shows its text.
    private func done() {
        tool("Done")
        waitUntil("the edit has ended") { !editor.exists }
    }

    /// The items that show one of the texts, from top to bottom.
    private func order(_ texts: [String]) -> [String] {
        texts.filter { app.staticTexts[$0].exists }.sorted { app.staticTexts[$0].frame.minY < app.staticTexts[$1].frame.minY }
    }

    /// Touches the item that shows the text right of its end, so that it is edited with the
    /// cursor at its end. The text is as wide as the item, and its letters are shorter: the
    /// right end of the text is right of them, and still on the screen.
    private func edit(_ text: String) {
        let item = app.staticTexts[text]
        waitFor(item)
        item.coordinate(withNormalizedOffset: CGVector(dx: 1, dy: 0.5)).withOffset(CGVector(dx: -4, dy: 0)).tap()
        waitFor(editor)
    }

    private func tool(_ name: String) {
        let tool = app.buttons[name]
        waitFor(tool)
        tool.tap()
    }

    // MARK: - Opening

    // 取り込まれたページは、検索で見つかり、開く。
    func testAPageThatWasTakenInIsFoundBySearchAndOpens() throws {
        try download("notes.md", "the first item\n")
        searchNotes()
        waitFor(app.staticTexts["the first item"])
    }

    // 新しいページの検索結果は、ページを作って開く。
    func testTheResultForANewPageMakesThePageAndOpensIt() throws {
        start()
        waitFor(search)
        search.tap()
        search.typeText("fresh")
        let result = app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'Create page \"fresh\"'")).firstMatch
        waitFor(result)
        result.tap()
        waitFor(app.staticTexts["fresh"])
    }

    // 今日のボタンは、今日のジャーナルを開く。
    func testTheTodayButtonOpensTheJournalOfToday() throws {
        try download("notes.md", "the first item\n")
        openNotes()
        app.buttons["Today"].tap()
        waitUntil("the first item is gone") { !app.staticTexts["the first item"].exists }
        XCTAssertFalse(app.staticTexts["notes"].exists)
    }

    // アプリは、前に表示していたページで開く。
    func testTheAppOpensAtThePageItShowedBefore() throws {
        try download("notes.md", "the first item\n")
        searchNotes()
        app.terminate()
        app.launch()
        waitFor(app.staticTexts["the first item"])
    }

    // MARK: - What an item shows

    // アイテムは装飾つきで表示され、Markdownの記号は表示されない。
    func testAnItemShowsItsDecorationAndNotTheMarksOfMarkdown() throws {
        try download("notes.md", "plain **bold** and `code`\n")
        openNotes()
        waitFor(app.staticTexts["plain bold and code"])
    }

    // コードブロックは書かれたとおりの行を表示し、表はセルごとに表示する。
    func testACodeBlockShowsItsLinesAndATableItsCells() throws {
        try download("notes.md", above("A") + "```\nlet x = 1\n  let y = 2\n```\n\n" + above("B") + "| left | right |\n|---|---|\n| one | two |\n")
        openNotes()
        waitFor(app.staticTexts["let x = 1\n  let y = 2"])
        for cell in ["left", "right", "one", "two"] { waitFor(app.staticTexts[cell]) }
    }

    // assetsフォルダーの画像は絵として表示され、タッチで大きく開き、もう一度のタッチで閉じる。
    func testAnImageShowsAsAPictureAndATouchOpensItLargeAndAnotherClosesIt() throws {
        try download("notes.md", "![a picture](../assets/pic.png)\n")
        let drawn = UIGraphicsImageRenderer(size: CGSize(width: 80, height: 40)).pngData { paper in
            UIColor.orange.setFill()
            paper.fill(CGRect(x: 0, y: 0, width: 80, height: 40))
        }
        try drawn.write(to: root.appendingPathComponent("markdown/assets/pic.png"))
        openNotes()
        let picture = app.images["picture"]
        waitFor(picture)
        XCTAssertEqual(picture.label, "a picture")
        picture.tap()
        let large = app.images["large picture"]
        waitFor(large)
        large.tap()
        waitUntil("the large picture is closed") { !large.exists }
    }

    // ファイルがまだない画像は、代わりの文字を表示する。
    func testAnImageWhoseFileIsNotHereShowsItsAlt() throws {
        try download("notes.md", "![a picture](../assets/none.png)\n")
        openNotes()
        waitFor(app.staticTexts["a picture"])
    }

    // タスクは状態の印を表示し、印へのタッチで次の状態へ進む。
    func testATaskShowsTheMarkOfItsStateAndATouchMovesItOn() throws {
        try download("notes.md", "- [ ] to do\n")
        openNotes()
        waitFor(app.buttons["Task not started"])
        app.buttons["Task not started"].tap()
        waitFor(app.buttons["Task running"])
        app.buttons["Task running"].tap()
        waitFor(app.buttons["Task done"])
    }

    // 子を隠した項目の点は、大きな薄い円の上に描かれる。子のある項目でも、隠していなければ円はない。
    func testTheBulletOfAFoldedItemStandsOnADisc() throws {
        let child = "<!-- @@ 2020-01-01-00-00-00-CCCCCCC idx=\"aC\" -->\nchild\n"
        try download("notes.md", above("A", " collapsed=\"true\"") + "parent\n\n" + child + "\n" + above("B") + "open\n\n" + child.replacingOccurrences(of: "C", with: "D") + "\n")
        openNotes()
        waitFor(app.staticTexts["open"])
        let folded = app.descendants(matching: .any).matching(identifier: "folded")
        XCTAssertEqual(folded.count, 1)
        // The disc is beside the folded item, whose child is hidden
        XCTAssertEqual(folded.firstMatch.frame.midY, app.staticTexts["parent"].frame.midY, accuracy: 12)
        XCTAssertEqual(app.staticTexts.matching(NSPredicate(format: "label == 'child'")).count, 1)
    }

    // MARK: - Links

    // ページへのリンクは、そのページを開く。
    func testALinkToAPageOpensThePage() throws {
        try download("notes.md", "[[other]]\n")
        try download("other.md", "the other page\n")
        openNotes()
        let link = app.links["other"]
        waitFor(link)
        link.tap()
        waitFor(app.staticTexts["the other page"])
    }

    // 上端のバーの戻るのボタンで前のページが開き、進むのボタンでそのあとのページが開く（ADR 035）。
    func testBackOpensThePageBeforeAndForwardThePageAfter() throws {
        try download("notes.md", "[[other]]\n")
        try download("other.md", "the other page\n")
        openNotes()
        let link = app.links["other"]
        waitFor(link)
        link.tap()
        waitFor(app.staticTexts["the other page"])

        app.buttons["Back"].tap()

        waitFor(app.links["other"])
        app.buttons["Forward"].tap()
        waitFor(app.staticTexts["the other page"])
    }

    // 起動して今日のジャーナルだけが開いているときは、戻るのボタンも進むのボタンも、出たまま押せない。
    func testTheBackAndForwardButtonsCannotBeTouchedWithoutAPageInTheirDirection() throws {
        start()
        let back = app.buttons["Back"]
        waitFor(back)

        XCTAssertFalse(back.isEnabled)
        XCTAssertFalse(app.buttons["Forward"].isEnabled)
    }

    // まだないページへのリンクは、そう知らせるだけでページを作らず、ボタンで作る。
    func testALinkToAPageThatIsNotThereSaysSoAndTheButtonMakesIt() throws {
        try download("notes.md", "[[nowhere]]\n")
        openNotes()
        let link = app.links["nowhere"]
        waitFor(link)
        link.tap()
        waitFor(app.staticTexts["No page yet: nowhere"])
        // The page that holds the link is still open
        XCTAssertTrue(app.staticTexts["notes"].exists)
        app.buttons["Make it"].tap()
        waitUntil("the new page is open") { !app.staticTexts["notes"].exists && !app.buttons["Make it"].exists }
        waitFor(app.staticTexts["nowhere"])
    }

    // MARK: - Editing

    // 新しいジャーナルの空のアイテムは、タッチして書ける。
    func testTheEmptyItemOfANewJournalIsTouchedAndWrittenInto() throws {
        start()
        let empty = app.buttons["Empty item"]
        waitFor(empty)
        empty.tap()
        waitFor(editor)
        editor.typeText("first words")
        done()
        waitFor(app.staticTexts["first words"])
    }

    // 開始記号を打ち終えると、入力欄に閉じる記号が補われ、カーソルは留まる（ADR 050）。
    func testTypingAnOpeningClosesItAndTheCursorStays() throws {
        start()
        let empty = app.buttons["Empty item"]
        waitFor(empty)
        empty.tap()
        waitFor(editor)
        editor.typeText(">>>")
        waitUntil("the opening is closed") { editor.value as? String == ">>>\n>>>" }
        editor.typeText("x")
        XCTAssertEqual(editor.value as? String, ">>>x\n>>>")
    }

    // タッチしたアイテムは、Markdownの入力欄になる。
    func testATouchTurnsAnItemIntoAFieldWithItsMarkdown() throws {
        try download("notes.md", "plain **bold**\n")
        openNotes()
        edit("plain bold")
        XCTAssertEqual(editor.value as? String, "plain **bold**")
    }

    // アイテムの文字の先頭に触れると、Markdownの先頭から書ける。
    func testATouchAtTheStartOfTheTextOfAnItemWritesAtTheStartOfItsMarkdown() throws {
        try download("notes.md", "**bold** plain\n")
        openNotes()
        let item = app.staticTexts["bold plain"]
        waitFor(item)
        item.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0.5)).withOffset(CGVector(dx: 1, dy: 0)).tap()
        waitFor(editor)
        editor.typeText("X")
        XCTAssertEqual(editor.value as? String, "X**bold** plain")
    }

    // 入力した文字は、アイテムの末尾に入り、編集を終えたアイテムに表示される。
    func testWhatIsTypedGoesToTheEndOfTheItemAndShowsAfterTheEdit() throws {
        try download("notes.md", "start\n")
        openNotes()
        edit("start")
        editor.typeText(" more")
        done()
        waitFor(app.staticTexts["start more"])
    }

    /// Makes the page longer than the screen with Enter, and ends the edit, as the user does.
    /// `words` is what is typed into each new item.
    private func longPageMadeWithEnter(words: (Int) -> String) throws {
        try download("notes.md", "first\n")
        openNotes()
        edit("first")
        for number in 1...30 {
            editor.typeText("\n" + words(number))
            waitUntil("item \(number) is edited") { editor.value as? String == words(number) }
        }
        done()
        waitUntil("the keyboard has gone") { app.keyboards.count == 0 }
    }

    /// Checks that the edit goes on after a touch: the keyboard is there, and takes letters.
    private func checkTheEditGoesOn(of text: String) {
        waitFor(app.keyboards.firstMatch)
        // The edit that ends by mistake ends within a moment
        usleep(2_000_000)
        XCTAssertTrue(editor.exists, "the edit has ended")
        XCTAssertTrue(app.keyboards.firstMatch.exists, "the keyboard has gone")
        editor.typeText("more")
        XCTAssertEqual(editor.value as? String, "\(text)more")
        XCTAssertTrue(editor.isHittable, "the field is under the keyboard")
    }

    // Enterで長くしたページで、編集を終えてから、画面の下端にある文字のあるアイテムにタッチすると、キーボードが出て編集が続く。
    func testATouchOnAnItemAtTheBottomOfALongPageEditsIt() throws {
        try longPageMadeWithEnter { "line \($0)" }
        let bottom = app.windows.firstMatch.frame.maxY
        let seen = (1...30).map { "line \($0)" }.filter { app.staticTexts[$0].exists && app.staticTexts[$0].frame.maxY <= bottom }
        let text = try XCTUnwrap(seen.max { app.staticTexts[$0].frame.minY < app.staticTexts[$1].frame.minY })
        edit(text)
        checkTheEditGoesOn(of: text)
    }

    // Enterで長くしたページで、編集を終えてから、画面の下端にある空のアイテムにタッチすると、キーボードが出て編集が続く。
    func testATouchOnAnEmptyItemAtTheBottomOfALongPageEditsIt() throws {
        try longPageMadeWithEnter { _ in "" }
        let bottom = app.windows.firstMatch.frame.maxY
        let seen = app.buttons.matching(identifier: "Empty item").allElementsBoundByIndex.filter { $0.frame.maxY <= bottom }
        let empty = try XCTUnwrap(seen.max { $0.frame.minY < $1.frame.minY })
        empty.tap()
        waitFor(editor)
        checkTheEditGoesOn(of: "")
    }

    // 画面を横にして、編集中のアイテムが見えるところから外れても、編集は続き、打った文字が入る。
    func testTheEditGoesOnWhenTheScreenIsTurnedAndTheEditedItemLeavesWhatIsSeen() throws {
        let lines = (1...40).map { "line \($0)" }
        try download("notes.md", lines.joined(separator: "\n\n") + "\n")
        openNotes()
        waitUntil("the keyboard of the search has gone") { app.keyboards.count == 0 }
        let bottom = app.windows.firstMatch.frame.maxY
        let seen = lines.filter { app.staticTexts[$0].exists && app.staticTexts[$0].frame.maxY <= bottom }
        let text = try XCTUnwrap(seen.max { app.staticTexts[$0].frame.minY < app.staticTexts[$1].frame.minY })
        edit(text)
        waitFor(app.keyboards.firstMatch)
        XCUIDevice.shared.orientation = .landscapeLeft
        defer { XCUIDevice.shared.orientation = .portrait }
        checkTheEditGoesOn(of: text)
    }

    // Enterは下に新しいアイテムを作り、入力はそこで続く。コアがアイテムを分けている間に打った文字も、新しいアイテムに入る。
    func testEnterMakesANewItemBelowAndTheTypingGoesOnThere() throws {
        try download("notes.md", "first\n")
        openNotes()
        edit("first")
        editor.typeText("\nsecond")
        waitFor(app.staticTexts["first"])
        waitUntil("the new item is edited, and has \(String(describing: editor.value))") { editor.value as? String == "second" }
        done()
        XCTAssertEqual(order(["first", "second"]), ["first", "second"])
    }

    private static let backspace = XCUIKeyboardKey.delete.rawValue

    /// Opens a page of one item, touches it, and presses Enter at its end. The edit is then in
    /// a new, empty item, with the cursor at its start. A test cannot put the cursor at the
    /// start of a field with text: neither a touch nor a key moves it there every time.
    private func editANewEmptyItem(after first: String) throws {
        try download("notes.md", first + "\n")
        openNotes()
        edit(first)
        editor.typeText("\n")
        waitUntil("the new item is edited, and has \(String(describing: editor.value))") { editor.value as? String == "" }
    }

    // つなぎ目のカーソルは、絵文字を1文字として数えた位置に置かれる。
    func testTheCursorAfterAJoinCountsAnEmojiBeforeItAsOneCharacter() throws {
        try editANewEmptyItem(after: "a😀")

        editor.typeText(JournalScreenTests.backspace)

        waitUntil("the item before is edited, and has \(String(describing: editor.value))") {
            editor.value as? String == "a😀"
        }
        editor.typeText("X")
        waitUntil("the letter is after the emoji, and the field has \(String(describing: editor.value))") {
            editor.value as? String == "a😀X"
        }
    }

    // 空のアイテムの先頭でのBackspaceは、そのアイテムを消す。編集は、前のアイテムの終わりで続く。
    func testBackspaceInAnEmptyItemTakesItAwayAndTheEditGoesOnAtTheEndOfTheOneBefore() throws {
        try editANewEmptyItem(after: "first")

        editor.typeText(JournalScreenTests.backspace)

        waitUntil("the item before is edited, and has \(String(describing: editor.value))") {
            editor.value as? String == "first"
        }
        editor.typeText("X")
        done()
        waitFor(app.staticTexts["firstX"])
        XCTAssertEqual(app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'first'")).count, 1)
    }

    // 先頭より後ろでのBackspaceは、前の1文字を消すだけである。
    func testBackspaceAfterTheStartOnlyDeletesTheCharacterBeforeTheCursor() throws {
        try download("notes.md", above("A") + "first\n\n" + above("B") + "second\n")
        openNotes()
        edit("second")

        editor.typeText(JournalScreenTests.backspace)

        waitUntil("the last letter is gone, and the field has \(String(describing: editor.value))") {
            editor.value as? String == "secon"
        }
        done()
        XCTAssertEqual(order(["first", "secon"]), ["first", "secon"])
    }

    // 道具は、アイテムを編集している間だけ表示され、Doneで編集が終わる。
    func testTheToolsShowOnlyWhileAnItemIsEditedAndDoneEndsTheEdit() throws {
        try download("notes.md", "only\n")
        openNotes()
        waitFor(app.staticTexts["only"])
        XCTAssertFalse(app.buttons["Indent"].exists)
        edit("only")
        waitFor(app.buttons["Indent"])
        app.buttons["Done"].tap()
        waitUntil("the edit has ended") { !editor.exists && !app.buttons["Indent"].exists }
        waitFor(app.staticTexts["only"])
    }

    private func three() throws {
        try download("notes.md", above("A") + "one\n\n" + above("B") + "two\n\n" + above("C") + "three\n")
        openNotes()
        edit("two")
    }

    // 字下げの道具は、アイテムを上のアイテムの子にし、字上げの道具は元に戻す。
    func testTheIndentToolsMoveTheItemUnderTheOneAboveAndBack() throws {
        try three()
        let before = editor.frame.minX
        tool("Indent")
        waitUntil("the item is a child") { editor.frame.minX > before }
        tool("Outdent")
        waitUntil("the item is back") { editor.frame.minX == before }
    }

    // 移動の道具は、アイテムを隣のアイテムと入れ替える。
    func testTheMoveToolsSwapTheItemWithItsNeighbour() throws {
        try three()
        tool("Move up")
        waitUntil("two is first") { editor.frame.minY < app.staticTexts["one"].frame.minY }
        tool("Move down")
        tool("Move down")
        waitUntil("two is last") { editor.frame.minY > app.staticTexts["three"].frame.minY }
        done()
        XCTAssertEqual(order(["one", "two", "three"]), ["one", "three", "two"])
    }

    // タスクの道具は、アイテムをタスクにする。
    func testTheTaskToolMakesTheItemATask() throws {
        try three()
        tool("Task")
        waitFor(app.buttons["Task not started"])
    }

    // 削除の道具は、アイテムを削除する。
    func testTheDeleteToolDeletesTheItem() throws {
        try three()
        tool("Delete item")
        waitUntil("two is gone") { !editor.exists || editor.value as? String != "two" }
        if editor.exists { done() }
        XCTAssertEqual(order(["one", "two", "three"]), ["one", "three"])
    }
}
