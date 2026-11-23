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
        XCTAssertTrue(
            element.waitForExistence(timeout: JournalScreenTests.wait), "\(element) is not shown", file: file,
            line: line)
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

    /// Ends the edit, so that every item shows its text. The item stays selected.
    private func done() {
        tool("Hide keyboard")
        waitUntil("the edit has ended") { !editor.exists }
    }

    /// The items that show one of the texts, from top to bottom.
    private func order(_ texts: [String]) -> [String] {
        texts.filter { app.staticTexts[$0].exists }.sorted {
            app.staticTexts[$0].frame.minY < app.staticTexts[$1].frame.minY
        }
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

    /// Where on a pad its arrow lies: near the edge of its side.
    private func near(_ arrow: String, of pad: XCUIElement) -> XCUICoordinate {
        let offsets = [
            "left": CGVector(dx: 0.1, dy: 0.5), "right": CGVector(dx: 0.9, dy: 0.5),
            "up": CGVector(dx: 0.5, dy: 0.1), "down": CGVector(dx: 0.5, dy: 0.9),
        ]
        return pad.coordinate(withNormalizedOffset: offsets[arrow]!)
    }

    /// Touches the arrow of a pad once: `left`, `right`, `up` or `down`.
    private func pad(_ name: String, _ arrow: String) {
        let pad = app.buttons[name]
        waitFor(pad)
        near(arrow, of: pad).tap()
    }

    /// Touches the arrow of the pad that moves the cursor.
    private func cursor(_ arrow: String, times: Int = 1) {
        for _ in 0..<times { pad("Move cursor", arrow) }
    }

    /// The Markdown in the field of the item that is edited.
    private var editedText: String { editor.value as? String ?? "" }

    /// Types where the cursor is, and waits until the field edits `text`: so the cursor was
    /// where `text` shows the letters.
    private func typeAndSee(_ typed: String, _ text: String, file: StaticString = #filePath, line: UInt = #line) {
        editor.typeText(typed)
        waitUntil("the field edits \(text), not \(editedText)", file: file, line: line) { editedText == text }
    }

    /// The item that is selected and coloured shows the text.
    private func selectedShows(_ text: String) -> Bool {
        app.descendants(matching: .any).matching(identifier: "selected").staticTexts[text].exists
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

    // 作れない名前のページの検索結果は、ページを作らず、作れない訳を知らせる。知らせはOKだけで、閉じると消える（#754）。
    func testTheResultForAPageOfANameThatCannotBeMadeTellsWhy() throws {
        start()
        waitFor(search)
        search.tap()
        search.typeText("a:b")
        let result = app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'Create page \"a:b\"'")).firstMatch
        waitFor(result)
        result.tap()

        let notice = app.alerts.firstMatch
        waitFor(notice)
        XCTAssertEqual(notice.buttons.count, 1)
        notice.buttons["OK"].tap()
        waitUntil("the notice has gone") { !app.alerts.firstMatch.exists }
    }

    // 今日のボタンは、今日のジャーナルを開く。
    func testTheTodayButtonOpensTheJournalOfToday() throws {
        try download("notes.md", "the first item\n")
        openNotes()
        app.buttons["Today"].tap()
        waitUntil("the first item is gone") { !app.staticTexts["the first item"].exists }
        XCTAssertFalse(app.staticTexts["notes"].exists)
    }

    // 今日のページで下に続く過去の日付は、今日の日付と同じ大きさで見える（#885）。
    func testAPastDateOnThePageOfTodayIsAsLargeAsTheDateOfToday() throws {
        try Data("an old note\n".utf8).write(to: root.appendingPathComponent("markdown/journals/2020-01-01.md"))
        try download("notes.md", "the first item\n")
        openNotes()
        app.buttons["Today"].tap()
        let past = shown("2020年1月1日")
        waitFor(past)
        let day = Calendar.current.dateComponents([.year, .month, .day], from: Date())
        let today = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "\(day.year!)年\(day.month!)月\(day.day!)日")
        ).firstMatch
        waitFor(today)

        XCTAssertEqual(past.frame.height, today.frame.height)
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
        try download(
            "notes.md",
            above("A") + "```\nlet x = 1\n  let y = 2\n```\n\n" + above("B")
                + "| left | right |\n|---|---|\n| one | two |\n")
        openNotes()
        waitFor(app.staticTexts["let x = 1\n  let y = 2"])
        for cell in ["left", "right", "one", "two"] { waitFor(app.staticTexts[cell]) }
    }

    // assetsフォルダーの画像は絵として表示され、タッチで大きく開き、もう一度のタッチで閉じる。
    func testAnImageShowsAsAPictureAndATouchOpensItLargeAndAnotherClosesIt() throws {
        try download("notes.md", "![a picture](../assets/pic.png)\n")
        try picture("pic.png")
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

    /// Puts a picture into the assets folder, as a download does.
    private func picture(_ name: String) throws {
        let drawn = UIGraphicsImageRenderer(size: CGSize(width: 80, height: 40)).pngData { paper in
            UIColor.orange.setFill()
            paper.fill(CGRect(x: 0, y: 0, width: 80, height: 40))
        }
        try drawn.write(to: root.appendingPathComponent("markdown/assets/\(name)"))
    }

    // 画像は、1行目に代わりの文字（alt）を表示し、その下に絵を表示する（#886）。
    func testAnImageShowsItsAltOnALineAboveThePicture() throws {
        try download("notes.md", "![a picture](../assets/pic.png)\n")
        try picture("pic.png")
        openNotes()
        let picture = app.images["picture"]
        waitFor(picture)
        let line = app.staticTexts["a picture"]
        waitFor(line)
        XCTAssertLessThanOrEqual(line.frame.maxY, picture.frame.minY)
    }

    // 代わりの文字（alt）のない画像は、1行目にファイルのパスを表示する（#886）。
    func testAnImageWithoutAnAltShowsItsPathOnTheLineAboveThePicture() throws {
        try download("notes.md", "![](../assets/pic.png)\n")
        try picture("pic.png")
        openNotes()
        waitFor(app.staticTexts["assets/pic.png"])
    }

    // 画像の1行目に触れると、その項目がMarkdownの入力欄になる（#886）。
    func testATouchOnTheLineAboveAnImageEditsTheItem() throws {
        try download("notes.md", "![a picture](../assets/pic.png)\n")
        try picture("pic.png")
        openNotes()
        waitFor(app.images["picture"])
        let line = app.staticTexts["a picture"]
        waitFor(line)

        line.tap()

        waitFor(editor)
        waitUntil("the field edits the image, not \(editedText)") { editedText == "![a picture](../assets/pic.png)" }
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
        try download(
            "notes.md",
            above("A", " collapsed=\"true\"") + "parent\n\n" + child + "\n" + above("B") + "open\n\n"
                + child.replacingOccurrences(of: "C", with: "D") + "\n")
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
        // The field shows the letters a moment after they are typed: on a busy Mac, it held
        // only "m" when it was read at once
        waitUntil("the field has the letters") { editor.value as? String == "\(text)more" }
        XCTAssertTrue(editor.isHittable, "the field is under the keyboard")
    }

    // Enterで長くしたページで、編集を終えてから、画面の下端にある文字のあるアイテムにタッチすると、キーボードが出て編集が続く。
    func testATouchOnAnItemAtTheBottomOfALongPageEditsIt() throws {
        try longPageMadeWithEnter { "line \($0)" }
        let bottom = app.windows.firstMatch.frame.maxY
        let seen = (1...30).map { "line \($0)" }.filter {
            app.staticTexts[$0].exists && app.staticTexts[$0].frame.maxY <= bottom
        }
        let text = try XCTUnwrap(seen.max { app.staticTexts[$0].frame.minY < app.staticTexts[$1].frame.minY })
        edit(text)
        checkTheEditGoesOn(of: text)
    }

    // Enterで長くしたページで、編集を終えてから、画面の下端にある空のアイテムにタッチすると、キーボードが出て編集が続く。
    func testATouchOnAnEmptyItemAtTheBottomOfALongPageEditsIt() throws {
        try longPageMadeWithEnter { _ in "" }
        let bottom = app.windows.firstMatch.frame.maxY
        let seen = app.buttons.matching(identifier: "Empty item").allElementsBoundByIndex.filter {
            $0.frame.maxY <= bottom
        }
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
        waitUntil("the new item is edited, and has \(String(describing: editor.value))") {
            editor.value as? String == "second"
        }
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
        waitUntil("the new item is edited, and has \(String(describing: editor.value))") {
            editor.value as? String == ""
        }
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

    // 道具は、編集しているときに出る。キーボードを閉じる道具は編集を終え、項目は選ばれて色が付く。キーボードを出す道具で、編集に戻る。
    func testTheToolsShowWhileAnItemIsEditedAndHideKeyboardLeavesItSelected() throws {
        try download("notes.md", "only\n")
        openNotes()
        waitFor(app.staticTexts["only"])
        XCTAssertFalse(app.buttons["Move block"].exists)
        XCTAssertFalse(app.buttons["Move cursor"].exists)
        edit("only")
        waitFor(app.buttons["Move cursor"])

        tool("Hide keyboard")
        waitUntil("the edit has ended") { !editor.exists }
        waitUntil("the item is selected") { selectedShows("only") }
        XCTAssertTrue(app.buttons["Move selection"].exists)
        tool("Show keyboard")

        waitFor(editor)
        XCTAssertEqual(editedText, "only")
        XCTAssertFalse(selectedShows("only"))
    }

    private func three() throws {
        try download("notes.md", above("A") + "one\n\n" + above("B") + "two\n\n" + above("C") + "three\n")
        openNotes()
        edit("two")
    }

    // 項目を動かすパッドの右は字下げし、左は字下げを戻す。
    func testTheRightAndLeftOfTheBlockPadIndentAndOutdentTheItem() throws {
        try three()
        let before = editor.frame.minX
        pad("Move block", "right")
        waitUntil("the item is a child") { editor.frame.minX > before }
        pad("Move block", "left")
        waitUntil("the item is back") { editor.frame.minX == before }
    }

    // 項目を動かすパッドの上下は、項目を隣の項目と入れ替える。
    func testTheUpAndDownOfTheBlockPadSwapTheItemWithItsNeighbour() throws {
        try three()
        pad("Move block", "up")
        waitUntil("two is first") { editor.frame.minY < app.staticTexts["one"].frame.minY }
        pad("Move block", "down")
        pad("Move block", "down")
        waitUntil("two is last") { editor.frame.minY > app.staticTexts["three"].frame.minY }
        done()
        XCTAssertEqual(order(["one", "two", "three"]), ["one", "three", "two"])
    }

    // 項目を動かすパッドを押し続けると、項目は動き続ける。
    func testHoldingTheBlockPadKeepsMovingTheItem() throws {
        try download(
            "notes.md",
            above("A") + "one\n\n" + above("B") + "two\n\n" + above("C") + "three\n\n" + above("D") + "four\n")
        openNotes()
        edit("four")

        near("up", of: app.buttons["Move block"]).press(forDuration: 1.5)

        done()
        XCTAssertEqual(order(["one", "two", "three", "four"]), ["four", "one", "two", "three"])
    }

    // タスクの道具は、アイテムをタスクにする。
    func testTheTaskToolMakesTheItemATask() throws {
        try three()
        tool("Task")
        waitFor(app.buttons["Task not started"])
    }

    // 道具の列に、削除の道具はない。
    func testTheToolsHaveNoDeleteTool() throws {
        try three()
        waitFor(app.buttons["Move block"])
        XCTAssertFalse(app.buttons["Delete item"].exists)
    }

    // CopyとCutの道具は、項目を選んでいるときだけ出る。編集しているときは出ない。
    func testTheCopyAndCutToolsShowOnlyWhileAnItemIsSelected() throws {
        try three()
        waitFor(app.buttons["Move cursor"])
        XCTAssertFalse(app.buttons["Copy"].exists)
        XCTAssertFalse(app.buttons["Cut"].exists)

        done()

        waitFor(app.buttons["Copy"])
        XCTAssertTrue(app.buttons["Cut"].exists)
    }

    // 選んだ項目を切り取り、別の項目の終わりに貼り付けると、その項目に続く。
    func testAnItemThatIsCutAndPastedAtTheEndOfAnotherGoesOnInIt() throws {
        try three()
        done()

        tool("Cut")
        waitUntil("the item is gone") { !app.staticTexts["two"].exists }
        edit("one")
        pasteWhereTheCursorIs()

        waitUntil("the field edits onetwo, not \(editedText)") { editedText == "onetwo" }
    }

    // メニューの「Paste」は、クリップボードの画像を本文フォルダのassetsに保存し、カーソルの位置にそのリンクを入れる（ADR 071）。
    func testPasteOfTheMenuPutsALinkToTheImageOnTheClipboardAtTheCursor() throws {
        try three()
        UIPasteboard.general.image = Self.drawing()

        pasteWhereTheCursorIs()

        waitUntil("the field edits a link to the image, not \(editedText)") {
            editedText.hasPrefix("two![") && editedText.contains("](../assets/")
        }
        let assets = try FileManager.default.contentsOfDirectory(
            atPath: root.appendingPathComponent("markdown/assets").path)
        XCTAssertEqual(assets.count, 1)
    }

    /// A picture of one red pixel, drawn by the test.
    private static func drawing() -> UIImage {
        UIGraphicsImageRenderer(size: CGSize(width: 1, height: 1)).image { context in
            UIColor.red.setFill()
            context.fill(CGRect(x: 0, y: 0, width: 1, height: 1))
        }
    }

    /// Pastes with the menu of the field, at the cursor. A touch on the cursor of the field
    /// that has the keyboard shows the menu.
    private func pasteWhereTheCursorIs() {
        editor.coordinate(withNormalizedOffset: CGVector(dx: 0.95, dy: 0.5)).tap()
        let paste = app.menuItems["Paste"]
        waitFor(paste)
        paste.tap()
    }

    /// Opens a page of two items, makes the second a child of the first, and edits the first.
    private func editAParent() throws {
        try download("notes.md", above("A") + "parent\n\n" + above("B") + "child\n")
        openNotes()
        edit("child")
        pad("Move block", "right")
        done()
        edit("parent")
    }

    // 折りたたみの道具は切り替える。子を隠すと「Expand」になり、もう一度押すと子が戻る。
    func testTheCollapseToolHidesTheChildrenAndShowsThemAgain() throws {
        try editAParent()

        tool("Collapse")
        waitUntil("the child is hidden") { !app.staticTexts["child"].exists }
        tool("Expand")

        waitFor(app.staticTexts["child"])
        waitFor(app.buttons["Collapse"])
    }

    // 子のない項目では、折りたたみの道具は押せない。
    func testTheCollapseToolCannotBeTouchedOnAnItemWithoutChildren() throws {
        try editAParent()
        XCTAssertTrue(app.buttons["Collapse"].isEnabled)

        done()
        edit("child")

        waitUntil("the tool cannot be touched") { !app.buttons["Collapse"].isEnabled }
    }

    // 元に戻すと、最後の変更が取り消される。やり直すと、また変わる。
    func testUndoTakesBackTheLastChangeAndRedoDoesItAgain() throws {
        try three()
        // Undo may give the edit back to the item that it moves: the item is a field then
        func left() -> CGFloat { editor.exists ? editor.frame.minX : app.staticTexts["two"].frame.minX }
        let before = left()
        pad("Move block", "right")
        waitUntil("the item is a child") { left() > before }

        tool("Undo")
        waitUntil("the item is back") { left() == before }
        tool("Redo")

        waitUntil("the item is a child again") { left() > before }
    }

    /// Opens a page of three items and selects the middle one, as Hide keyboard leaves it.
    private func selectTwo() throws {
        try three()
        done()
        waitUntil("two is selected") { selectedShows("two") }
    }

    // 項目を選んでいる間、2つ目のパッドは、上下で選ぶ項目を変える。色も移る。
    func testThePadOfTheSelectionSelectsTheItemAboveOrBelow() throws {
        try selectTwo()

        pad("Move selection", "down")
        waitUntil("three is selected") { selectedShows("three") && !selectedShows("two") }
        pad("Move selection", "up")
        pad("Move selection", "up")

        waitUntil("one is selected") { selectedShows("one") && !selectedShows("three") }
    }

    // 項目を選んでいる間、項目を動かすパッドは、選んだ項目を動かす。
    func testTheBlockPadMovesTheSelectedItem() throws {
        try selectTwo()

        pad("Move block", "up")

        waitUntil("two is first") { order(["one", "two", "three"]) == ["two", "one", "three"] }
        XCTAssertTrue(selectedShows("two"))
    }

    // 道具を縮めると、丸いボタンだけになる。押すと、道具が戻る。
    func testTheToolsShrinkToARoundButtonAndComeBack() throws {
        try selectTwo()

        tool("Shrink tools")
        waitUntil("the tools are shrunk") { !app.buttons["Move block"].exists }
        tool("Expand tools")

        waitFor(app.buttons["Move block"])
    }

    // 縮めた道具は、編集と選択を行き来しても縮めたままである。
    func testTheToolsStayShrunkWhileTheSelectionChangesToAnEdit() throws {
        try selectTwo()
        tool("Shrink tools")

        edit("two")

        XCTAssertTrue(app.buttons["Expand tools"].exists)
        XCTAssertFalse(app.buttons["Move cursor"].exists)
    }

    // 項目に触れると、選択は終わり、その項目の編集になる。
    func testATouchOnAnItemEndsTheSelectionAndEditsIt() throws {
        try selectTwo()

        edit("three")

        XCTAssertEqual(editedText, "three")
        waitUntil("no item is selected") { !app.descendants(matching: .any)["selected"].exists }
    }

    // MARK: - The pad of the cursor

    // パッドの真ん中に触れても、カーソルは動かない。
    func testATouchOnTheMiddleOfThePadDoesNotMoveTheCursor() throws {
        try download("notes.md", "abc\n")
        openNotes()
        edit("abc")

        app.buttons["Move cursor"].tap()

        typeAndSee("X", "abcX")
    }

    // ←と→は、カーソルを1文字ずつ動かす。絵文字は、1文字として数える。
    func testLeftAndRightMoveTheCursorByOneCharacterAndCountAnEmojiAsOne() throws {
        try download("notes.md", "😀ab\n")
        openNotes()
        edit("😀ab")

        cursor("left", times: 3)
        cursor("right")

        typeAndSee("X", "😀Xab")
    }

    // パッドを押し続けると、カーソルは動き続ける。
    func testHoldingThePadKeepsMovingTheCursor() throws {
        try download("notes.md", "abcdefgh\n")
        openNotes()
        edit("abcdefgh")

        // Longer than eight moves take: the cursor stays at the start of the only item after
        near("left", of: app.buttons["Move cursor"]).press(forDuration: 3)

        typeAndSee("X", "Xabcdefgh")
    }

    /// Opens a page of two items, and edits the one that shows `edited`, with the cursor at
    /// its end.
    private func editOneOfTwo(_ first: String, _ second: String, edited: String) throws {
        try download("notes.md", above("A") + first + "\n\n" + above("B") + second + "\n")
        openNotes()
        edit(edited)
        XCTAssertEqual(editedText, edited)
    }

    // 項目の先頭での←は、前の項目の末尾へ移る。
    func testLeftAtTheStartMovesToTheEndOfTheItemBefore() throws {
        try editOneOfTwo("first", "second", edited: "second")
        cursor("left", times: 6)

        cursor("left")

        waitUntil("the item before is edited") { editedText == "first" }
        typeAndSee("X", "firstX")
    }

    // 項目の末尾での→は、次の項目の先頭へ移る。
    func testRightAtTheEndMovesToTheStartOfTheItemAfter() throws {
        try editOneOfTwo("first", "second", edited: "first")

        cursor("right")

        waitUntil("the item after is edited") { editedText == "second" }
        typeAndSee("X", "Xsecond")
    }

    // 最初の行での↑は、前の項目へ移る。行の中の位置は変わらない。
    func testUpOnTheFirstLineMovesToTheItemBeforeAtTheSamePlace() throws {
        try editOneOfTwo("first", "second", edited: "second")
        cursor("left", times: 3)

        cursor("up")

        waitUntil("the item before is edited") { editedText == "first" }
        typeAndSee("X", "firXst")
    }

    // 最後の行での↓は、次の項目へ移る。行の中の位置は変わらない。
    func testDownOnTheLastLineMovesToTheItemAfterAtTheSamePlace() throws {
        try editOneOfTwo("first", "second", edited: "first")
        cursor("left", times: 3)

        cursor("down")

        waitUntil("the item after is edited") { editedText == "second" }
        typeAndSee("X", "seXcond")
    }

    // 何行もある項目では、↑と↓は、同じ項目の上と下の行へ動かす。
    func testUpAndDownMoveBetweenTheLinesOfAnItem() throws {
        try download("notes.md", "one\ntwo\n")
        openNotes()
        let item = shown("two")
        waitFor(item)
        item.coordinate(withNormalizedOffset: CGVector(dx: 1, dy: 0.9)).withOffset(CGVector(dx: -4, dy: 0)).tap()
        waitFor(editor)
        let text = editedText
        let after = text.replacingOccurrences(of: "one", with: "oneX")

        cursor("up")
        typeAndSee("X", after)
        cursor("down")

        typeAndSee("Y", after + "Y")
    }
}
