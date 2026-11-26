import Foundation
import PetaBorder
import PetaTypes
import XCTest

/// The shell with a real core under it, on the folders of a test.
@MainActor
final class ShellTests: JournalTestCase {
    private func bodies() -> [String] {
        shell.view.outline.rows.map(\.item.body)
    }

    /// The items of the page, each as its level and its body.
    private func items() -> [String] {
        shell.view.outline.rows.map { "\($0.item.level) \($0.item.body)" }
    }

    private func id(of body: String) -> String {
        shell.view.outline.rows.first { $0.item.body == body }!.item.id
    }

    /// A field with the cursor `at` UTF-16 units into `text`, at the edge of its lines. Which
    /// keys mean something where is tested in the core (ADR 052).
    private func cursor(_ at: UInt64, in text: String) -> EditorState {
        EditorState(text: text, selectionStart: at, selectionEnd: at, onFirstLine: true, onLastLine: true)
    }

    private func edited() -> String? {
        shell.view.outline.rows.first { $0.isEditing }?.item.body
    }

    /// The text of the file of a page, after the core wrote what changed in the app.
    private func written(_ path: String = "docs/notes.md") -> String? {
        takeIn()
        return try? String(contentsOf: markdown.appendingPathComponent(path), encoding: .utf8)
    }

    /// Opens a page of three items, and touches the second to edit it.
    private func editSecondOfThree() throws {
        try download("notes.md", above("A") + "first\n\n" + above("B") + "second\n\n" + above("C") + "third\n")
        try openNotes()
        shell.edit(id(of: "second"))
        XCTAssertEqual(edited(), "second")
    }

    // 検索すると、取り込んだページが結果に出る。結果を選ぶと、そのページの項目が見える。
    func testAPageThatWasTakenInIsFoundByTheSearchAndOpens() throws {
        try download("notes.md", "今日は東京都で会議がありました。\n")
        try start()

        // A word of the body finds the item, under the name of its page
        pick(searchingFor: "会議") { $0.title == "notes" }

        waitUntil("the page shows its item") { self.bodies() == ["今日は東京都で会議がありました。"] }
        XCTAssertFalse(shell.view.search.showDropdown)
    }

    // ページへのリンクに触れると、そのページが開く。
    func testALinkToAPageOpensThePage() throws {
        try download("notes.md", "[[other]]\n")
        try download("other.md", "what the other page says\n")
        try openNotes()

        shell.follow(.page(url: "/pages/other"))

        waitUntil("the other page shows") { self.bodies() == ["what the other page says"] }
        XCTAssertEqual(browsed, [])
    }

    // 戻ると前のページが開き、進むと戻る前のページが開く。開いたページの履歴は、コアが持つ（ADR 035）。
    func testBackOpensThePageBeforeAndForwardThePageAfter() throws {
        try download("notes.md", "[[other]]\n")
        try download("other.md", "what the other page says\n")
        try openNotes()
        XCTAssertFalse(shell.view.history.canGoBack)
        shell.follow(.page(url: "/pages/other"))
        waitUntil("the other page shows") { self.bodies() == ["what the other page says"] }

        shell.back()

        waitUntil("the notes show again") { self.bodies() == ["[[other]]"] }
        XCTAssertTrue(shell.view.history.canGoForward)

        shell.forward()

        waitUntil("the other page shows again") { self.bodies() == ["what the other page says"] }
    }

    // まだないページへのリンクに触れると、ほかのシェルと同じく、そのページができて開き、ファイルが書かれる（ADR 083）。
    func testALinkToAPageThatIsNotThereMakesThePageAndOpensIt() throws {
        try download("notes.md", "[[nowhere]]\n")
        try openNotes()

        shell.follow(.page(url: "/pages/nowhere"))

        waitUntil("the new page is open") { self.title() == "nowhere" }
        XCTAssertNotNil(written("docs/nowhere.md"))
    }

    // Webへのリンクに触れると、そのアドレスがブラウザに渡る。開いているページは変わらない。
    func testALinkOutOfTheJournalOpensInTheBrowser() throws {
        try download("notes.md", "[a](https://example.com/a)\n")
        try openNotes()

        shell.follow(.web(url: "https://example.com/a"))

        waitUntil("the address goes to the browser") { self.browsed.map(\.absoluteString) == ["https://example.com/a"] }
        XCTAssertEqual(title(), "notes")
    }

    private func title() -> String? {
        if case .page(let page) = shell.view.screen { return page?.title }
        return nil
    }

    // 入力した名前のページがなければ、作る結果が出る。選ぶと、ページができて開き、ファイルが書かれる。
    func testTheResultForANewPageMakesThePageAndOpensIt() throws {
        try start()

        pick(.create, searchingFor: "fresh")

        waitUntil("the new page is open") { self.title() == "fresh" }
        XCTAssertNotNil(written("docs/fresh.md"))
    }

    // 作れない名前のページを作ろうとすると、コアの知らせが出る。知らせは答えを待たないので、閉じても何も送らない（#754）。
    func testANoticeOfTheCoreShowsWhenAPageOfANameThatCannotBeMadeIsMade() throws {
        try start()

        pick(.create, searchingFor: "a:b")

        waitUntil("the core tells") { self.shell.question != nil }
        XCTAssertEqual(shell.question?.waits, false)
        XCTAssertTrue(shell.question?.text.contains(":") == true, "\(shell.question?.text ?? "")")
        shell.answer(yes: true)
        XCTAssertNil(shell.question)
        XCTAssertNil(written("docs/a:b.md"))
    }

    // 「today」と打つと、今日のジャーナルを開く結果が出る。選ぶと、今日のジャーナルができて開く。
    func testTheResultForTodayMakesTheJournalOfTodayAndOpensIt() throws {
        try start()

        pick(.today, searchingFor: "today")

        waitUntil("the journal is open") { self.isJournal() }
    }

    private func isJournal() -> Bool {
        if case .journal = shell.view.screen { return true }
        return false
    }

    // 今日のジャーナルを頼むと、今日のジャーナルができて開く。
    func testTodayMakesTheJournalOfTodayAndOpensIt() throws {
        try start()

        shell.today()

        waitUntil("the journal is open") { self.isJournal() }
    }

    // ジャーナルを、読んだ日の終わり近くまでスクロールすると、その前の日を読み足す。
    func testScrollingTheJournalNearTheEndOfTheDaysItHasLoadsTheDaysBefore() throws {
        // More days than the journal loads at first: each day has one item
        let days = 130
        for ago in 1...days {
            try journal(daysAgo: ago, "day \(ago)")
        }
        try start()
        shell.today()
        waitUntil("the first days are loaded") { self.shell.view.outline.rows.count >= 100 }
        let loaded = shell.view.outline.rows.count
        XCTAssertFalse(bodies().contains("day \(days)"))

        shell.scrolled(top: loaded - 5, shown: (loaded - 10)...(loaded - 1))

        waitUntil("the days before are loaded") { self.bodies().contains("day \(days)") }
    }

    /// Puts the journal of the day `ago` days before today into the Markdown folder, with one
    /// item of `text`.
    private func journal(daysAgo ago: Int, _ text: String) throws {
        let day = Calendar.current.date(byAdding: .day, value: -ago, to: Date())!
        let format = DateFormatter()
        format.locale = Locale(identifier: "en_US_POSIX")
        format.dateFormat = "yyyy-MM-dd"
        let name = format.string(from: day)
        let folder = markdown.appendingPathComponent("journals")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        try Data("\(text)\n".utf8).write(to: folder.appendingPathComponent("\(name).md"))
    }

    /// Ends the app, and opens it again on the same folders, at the screen it had.
    private func openAgain() {
        shell.close()
        // The journal is let go of on another thread, and cannot be opened before that
        waitUntil("the journal can be opened again") {
            guard let next = try? Shell(dataDir: self.root.appendingPathComponent("data"), markdownDir: self.markdown)
            else { return false }
            self.shell = next
            return true
        }
        shell.restore()
    }

    // アプリを開くと、前に見ていたページが出る。
    func testTheAppOpensAtThePageItShowedBefore() throws {
        try download("notes.md", "what the notes say\n")
        try openNotes()

        openAgain()

        waitUntil("the page shows again") { self.bodies() == ["what the notes say"] }
    }

    // 入れたばかりのアプリは、作らないと言われたら、今日のジャーナルを作らず、何も開かない。
    func testANewAppThatIsNotToMakeTodayOpensNoScreen() throws {
        try start()

        shell.restore(make: false)

        XCTAssertTrue(shell.unopened)
        XCTAssertFalse(isJournal())
    }

    // 何も開かなかったアプリは、クラウドとの同期の取り込みのあとに、今日のジャーナルを開く。
    func testAnAppThatOpenedNoScreenOpensTodayAfterTheTakingInOfASync() throws {
        try start()
        shell.restore(make: false)

        let shell = shell!
        Thread.detachNewThread { shell.syncAndWait() }

        waitUntil("the journal is open") { self.isJournal() }
        XCTAssertFalse(shell.unopened)
    }

    // 文字の位置なしで項目に触れると、その項目が編集に入り、カーソルは末尾に置かれる（ADR 057）。
    func testATouchWithoutALetterEditsTheItemWithTheCursorAtTheEnd() throws {
        try download("notes.md", "a **bold** word\n")
        try openNotes()

        shell.edit(id(of: "a **bold** word"))

        XCTAssertEqual(edited(), "a **bold** word")
        XCTAssertEqual(shell.view.outline.focusRequest?.cursor, 15)
    }

    // 表示した文字に触れると、その項目が編集に入り、カーソルは触れた文字のMarkdownでの位置に置かれる。
    func testATouchOnTheShownTextPutsTheCursorAtItsPlaceInTheMarkdown() throws {
        try download("notes.md", "a **bold** word\n")
        try openNotes()

        // The shown text is "a bold word": the touch is after "a bo"
        shell.edit(id(of: "a **bold** word"), letter: 4)

        XCTAssertEqual(edited(), "a **bold** word")
        XCTAssertEqual(shell.view.outline.focusRequest?.cursor, 6)
    }

    // 打った文字は、ページのファイルに書かれる。
    func testWhatIsTypedGoesIntoTheFileOfThePage() throws {
        try download("notes.md", "first\n")
        try openNotes()
        shell.edit(id(of: "first"))

        shell.type(id(of: "first"), "first and more")

        waitUntil("the file holds what was typed") { self.written()?.contains("first and more") == true }
    }

    // アプリが裏に回ると、保存を待っていた本文が、待たずに保存され、ページのファイルに書かれる（#766）。
    func testWhatWaitsToBeSavedIsSavedWhenTheAppGoesToTheBack() throws {
        try download("notes.md", "first\n")
        try openNotes()
        shell.edit(id(of: "first"))
        shell.type(id(of: "first"), "first before the back")

        shell.wentToTheBack()

        XCTAssertEqual(written()?.contains("first before the back"), true)
    }

    // 開始記号を打ち終えると、コアが閉じる記号を補う（ADR 050、ADR 057）。
    func testTypingAnOpeningClosesIt() throws {
        try download("notes.md", "first\n")
        try openNotes()
        let item = id(of: "first")
        shell.edit(item)

        shell.type(item, ">>", cursor: 2)
        shell.type(item, ">>>", cursor: 3)

        XCTAssertEqual(edited(), ">>>\n>>>")
    }

    // 改行は、カーソルの位置で項目を分ける。編集は、新しい項目に移る。
    func testEnterSplitsTheItemAtTheCursorAndTheEditGoesOnInTheNewItem() throws {
        try download("notes.md", "first\n")
        try openNotes()
        shell.edit(id(of: "first"))

        shell.press(.enter, in: id(of: "first"), cursor(5, in: "first"))

        waitUntil("there are two items") { self.bodies() == ["first", ""] }
        waitUntil("the new item is edited") { self.edited() == "" }
    }

    /// Presses Enter at the end of the item `first`, the only item of the page. The core holds
    /// the keys from then on, until the field of the new item takes the keyboard (ADR 087).
    private func pressEnterAtTheEndOfFirst() throws {
        try download("notes.md", "first\n")
        try openNotes()
        shell.edit(id(of: "first"))
        shell.press(.enter, in: id(of: "first"), cursor(5, in: "first"))
    }

    /// Waits until the core orders the field of the new item to take the keyboard.
    private func waitForTheOrderOfTheNewItem() {
        waitUntil("the new item is edited") { self.edited() == "" }
        waitUntil("the core orders a field to take the keyboard") { self.shell.waitsForTheKeyboard }
    }

    private func names(_ keys: [DeferredKey]) -> [String] {
        keys.map(\.key)
    }

    // 項目を分ける間に打った文字は、コアが預かり、新しい項目の欄がキーボードを取ったときに、その欄へ渡される。
    func testWhatIsTypedWhileTheItemIsSplitIsGivenToTheFieldThatTakesTheKeyboard() throws {
        try pressEnterAtTheEndOfFirst()
        XCTAssertTrue(shell.holdsKeys)

        shell.hold("ab")
        waitForTheOrderOfTheNewItem()

        XCTAssertEqual(names(shell.keyboardTaken()), ["a", "b"])
    }

    // コアは、注文を出したあとも、欄がキーボードを取るまで預かり続ける。その間に打った文字は次の注文で渡され、その注文に答えると預かりは終わる。
    func testTheCoreHoldsTheKeysUntilTheFieldTakesTheKeyboard() throws {
        try pressEnterAtTheEndOfFirst()
        waitForTheOrderOfTheNewItem()
        XCTAssertTrue(shell.holdsKeys)

        shell.hold("c")
        XCTAssertEqual(shell.keyboardTaken(), [])

        waitUntil("the core orders the field again") { self.shell.waitsForTheKeyboard }
        XCTAssertEqual(names(shell.keyboardTaken()), ["c"])
        waitUntil("the core no longer holds") { !self.shell.holdsKeys }
    }

    // 欄が最後の注文に答えると、預かりはすぐに終わる。そのあとすぐに打った文字は、捨てられずに欄へ入る。
    func testTheHoldingEndsAsSoonAsTheFieldAnswersTheLastOrder() throws {
        try pressEnterAtTheEndOfFirst()
        waitForTheOrderOfTheNewItem()

        XCTAssertEqual(shell.keyboardTaken(), [])

        XCTAssertFalse(shell.holdsKeys)
    }

    // 預ける改行はEnterのキー、預けるBackspaceはBackspaceのキーになる。打った順に渡される。
    func testALineBreakIsHeldAsEnterAndABackspaceAsBackspace() throws {
        try pressEnterAtTheEndOfFirst()

        shell.hold("a\nb")
        shell.holdBackspace()
        waitForTheOrderOfTheNewItem()

        XCTAssertEqual(names(shell.keyboardTaken()), ["a", "Enter", "b", "Backspace"])
    }

    // キーボードを取る欄がないまま編集が終わると、コアは預かるのをやめ、預けた文字はどこにも入らない。
    func testTheEndOfTheEditWithoutAFieldEndsTheHoldingAndTheKeysGo() throws {
        try pressEnterAtTheEndOfFirst()
        shell.hold("ab")
        waitForTheOrderOfTheNewItem()

        shell.finish(id(of: ""))

        XCTAssertFalse(shell.waitsForTheKeyboard)
        waitUntil("the core no longer holds") { !self.shell.holdsKeys }
        XCTAssertEqual(shell.keyboardTaken(), [])
        XCTAssertEqual(bodies(), ["first", ""])
    }

    // 行の端にないカーソルの矢印は、アウトラインには意味がないので、欄に任される。編集は、同じ項目で続く。
    func testAnArrowAwayFromTheEdgeOfTheLinesIsLeftToTheField() throws {
        try editSecondOfThree()
        let middle = EditorState(
            text: "second", selectionStart: 3, selectionEnd: 3, onFirstLine: false, onLastLine: false)

        XCTAssertFalse(shell.press(.up, in: id(of: "second"), middle))
        XCTAssertEqual(edited(), "second")
    }

    // アプリを閉じたあとのキーは、何もしない。画面を下ろす途中でも、欄はキーを伝えてくるためである。
    func testAKeyAfterTheAppClosedDoesNothing() throws {
        try download("notes.md", "first\n")
        try openNotes()
        let first = id(of: "first")
        shell.edit(first)

        shell.close()

        XCTAssertFalse(shell.press(.enter, in: first, cursor(5, in: "first")))
        XCTAssertEqual(bodies(), ["first"])
    }

    // 先頭でのBackspaceは、項目を前の項目につなぐ。編集は、つないだ項目で続く。カーソルは、つなぎ目に置かれる。
    func testBackspaceAtTheStartJoinsTheItemToTheOneBeforeAndTheEditGoesOnWhereTheyMeet() throws {
        try editSecondOfThree()

        shell.press(.backspace, in: id(of: "second"), cursor(0, in: "second"))

        waitUntil("the two items are one") { self.bodies() == ["firstsecond", "third"] }
        waitUntil("the joined item is edited") { self.edited() == "firstsecond" }
        XCTAssertEqual(shell.view.outline.focusRequest?.cursor, 5)
    }

    // 編集を終えると、どの項目も編集中でなくなる。
    func testFinishEndsTheEdit() throws {
        try editSecondOfThree()

        shell.finish(id(of: "second"))

        XCTAssertNil(edited())
    }

    // 字下げは、項目を上の項目の子にする。戻すと、元に戻る。編集は続く。
    func testIndentMovesTheItemUnderTheOneAboveAndOutdentMovesItBack() throws {
        try editSecondOfThree()

        shell.indent(id(of: "second"))
        waitUntil("the item is a child") { self.items() == ["1 first", "2 second", "1 third"] }
        XCTAssertEqual(edited(), "second")
        shell.outdent(id(of: "second"))

        waitUntil("the item is back") { self.items() == ["1 first", "1 second", "1 third"] }
    }

    // 上へと下へは、項目を隣の項目と入れ替える。
    func testMoveUpAndMoveDownSwapTheItemWithItsNeighbour() throws {
        try editSecondOfThree()

        shell.moveUp(id(of: "second"))
        waitWhileTheFieldAnswers("the item is first") { self.bodies() == ["second", "first", "third"] }
        shell.moveDown(id(of: "second"))
        shell.moveDown(id(of: "second"))

        waitWhileTheFieldAnswers("the item is last") { self.bodies() == ["first", "third", "second"] }
    }

    /// Waits until the condition holds, while the field of the edited item keeps the keyboard:
    /// it answers each order of the core to take the keyboard, as the field on the screen does.
    /// The core ends a move only after the answer, and starts the next one then (ADR 087).
    private func waitWhileTheFieldAnswers(
        _ what: String, _ holds: @escaping () -> Bool, file: StaticString = #filePath, line: UInt = #line
    ) {
        let answering = {
            _ = self.shell.keyboardTaken()
            return holds()
        }
        waitUntil(what, answering, file: file, line: line)
    }

    // タスクの切り替えは、項目をタスクにし、そのたびに状態を進める。最後はタスクでなくなる。
    func testCycleTaskMakesTheItemATaskAndMovesItOn() throws {
        try editSecondOfThree()
        let second = id(of: "second")
        func task() -> TaskState? { shell.view.outline.rows.first { $0.item.id == second }?.task }

        shell.cycleTask(second)
        waitUntil("the task is not started") { task() == .notStarted }
        shell.cycleTask(second)
        waitUntil("the task runs") { task() == .running }
        shell.cycleTask(second)
        waitUntil("the task is done") { task() == .done }
        shell.cycleTask(second)

        waitUntil("the item is no task") { task() == nil }
    }

    // 折りたたみは、項目の子を隠す。もう一度で、子が戻る。
    func testToggleCollapsedHidesTheChildrenAndShowsThemAgain() throws {
        try editSecondOfThree()
        shell.indent(id(of: "second"))
        waitUntil("the item is a child") { self.items() == ["1 first", "2 second", "1 third"] }
        let first = id(of: "first")

        shell.toggleCollapsed(first)
        waitUntil("the child is hidden") { self.items() == ["1 first", "1 third"] }
        XCTAssertEqual(shell.view.outline.rows.first?.isCollapsed, true)
        shell.toggleCollapsed(first)

        waitUntil("the child is back") { self.items() == ["1 first", "2 second", "1 third"] }
    }

    private func selected() -> [String] {
        shell.view.outline.rows.filter(\.isSelected).map(\.item.body)
    }

    /// Opens a page of three items, and selects the second, as the end of its edit leaves it.
    private func selectSecondOfThree() throws {
        try editSecondOfThree()
        shell.finish(id(of: "second"))
        shell.select(id(of: "second"))
        XCTAssertEqual(selected(), ["second"])
    }

    // 選択を終えると、どの項目も選ばれていない。
    func testUnselectEndsTheSelection() throws {
        try selectSecondOfThree()

        shell.unselect()

        XCTAssertEqual(selected(), [])
    }

    // 選んだ項目は、上へ動かせる。選んだままである。
    func testMoveSelectedMovesTheSelectedItem() throws {
        try selectSecondOfThree()

        shell.moveSelected(.moveUp)

        waitUntil("the item is first") { self.bodies() == ["second", "first", "third"] }
        XCTAssertEqual(selected(), ["second"])
    }

    // 選んだ項目をコピーすると、その項目がMarkdownでクリップボードに載る。項目はページに残る。
    func testCopyingTheSelectedItemPutsItsMarkdownOnTheClipboard() throws {
        try selectSecondOfThree()

        shell.copySelected()

        waitUntil("the item is on the clipboard") { !self.copied.isEmpty }
        XCTAssertEqual(copied, ["- second"])
        XCTAssertEqual(bodies(), ["first", "second", "third"])
    }

    // 選んだ項目を切り取ると、その項目がMarkdownでクリップボードに載り、ページから消える。
    func testCuttingTheSelectedItemPutsItOnTheClipboardAndTakesItOffThePage() throws {
        try selectSecondOfThree()

        shell.cutSelected()

        waitUntil("the item is gone") { self.bodies() == ["first", "third"] }
        XCTAssertEqual(copied, ["- second"])
    }

    // 2つの項目のリストを項目の終わりに貼り付けると、1つ目はその項目に続き、2つ目は下の新しい項目になる。
    func testAListOfTwoItemsPastedAtTheEndOfAnItemGoesOnInItAndMakesANewItemBelow() throws {
        try editSecondOfThree()

        shell.paste("- apple\n- pear", into: id(of: "second"), at: 6)

        waitUntil("the list is items") { self.bodies() == ["first", "secondapple", "pear", "third"] }
    }

    // 画像のファイルを貼ると、本文フォルダのassetsに保存され、カーソルの位置にそのリンクが入る。ファイルは保存のあとに消える（ADR 071）。
    func testAPastedImageFileIsSavedInTheAssetsAndLinkedAtTheCursor() throws {
        try editSecondOfThree()
        let image = try PastedImage.write(Data([1, 2, 3]), named: "sample.png", type: "public.png")

        shell.pasteImage(image, into: id(of: "second"), at: 3)

        waitUntil("the item links the image after sec") {
            self.bodies()[1].hasPrefix("sec![sample.png](../assets/") && self.bodies()[1].hasSuffix(")ond")
        }
        let name = bodies()[1].components(separatedBy: "../assets/")[1].components(separatedBy: ")")[0]
        XCTAssertEqual(try Data(contentsOf: markdown.appendingPathComponent("assets/\(name)")), Data([1, 2, 3]))
        waitUntil("the file is deleted") { !FileManager.default.fileExists(atPath: image.path) }
    }

    // 親と子を選んでコピーし、別の項目の終わりに貼り付けると、親と子の形のまま戻る。
    func testAParentAndItsChildCopiedAndPastedAtTheEndOfAnotherItemComeBackAsAParentAndItsChild() throws {
        let child = "<!-- @@ 2020-01-01-00-00-00-AAAAAAB idx=\"aB\" -->\n"
        try download("notes.md", above("A") + "parent\n\n" + child + "child\n\n" + above("C") + "third\n")
        try openNotes()
        shell.edit(id(of: "parent"))
        shell.finish(id(of: "parent"))
        shell.select(id(of: "parent"))
        shell.copySelected()
        waitUntil("the items are on the clipboard") { !self.copied.isEmpty }
        shell.edit(id(of: "third"))

        shell.paste(copied.last!, into: id(of: "third"), at: 5)

        waitUntil("the copied items are back as they were") {
            self.items() == ["1 parent", "2 child", "1 thirdparent", "2 child"]
        }
    }

    // 元に戻すは、最後の変更を取り消す。やり直すは、それをまた行う。
    func testUndoTakesBackTheLastChangeAndRedoDoesItAgain() throws {
        try editSecondOfThree()
        shell.indent(id(of: "second"))
        waitUntil("the item is a child") { self.items() == ["1 first", "2 second", "1 third"] }

        shell.undo()
        waitUntil("the item is back") { self.items() == ["1 first", "1 second", "1 third"] }
        shell.redo()

        waitUntil("the item is a child again") { self.items() == ["1 first", "2 second", "1 third"] }
    }

    // 項目のメニューで消すと、項目はページから消える。
    func testAnItemDeletedFromItsMenuLeavesThePage() throws {
        try download("notes.md", above("A") + "first\n\n" + above("B") + "second\n\n" + above("C") + "third\n")
        try openNotes()

        shell.delete(id(of: "second"))

        waitUntil("the item is gone") { self.bodies() == ["first", "third"] }
    }

    // 項目のメニューでstashへ送ると、項目はページの終わりの`# Stash`の見出しの下へ移る。
    func testAnItemSentToTheStashFromItsMenuGoesUnderTheStashHeadingAtTheEnd() throws {
        try download("notes.md", above("A") + "first\n\n" + above("B") + "second\n\n" + above("C") + "third\n")
        try openNotes()

        shell.sendToStash(id(of: "second"))

        waitUntil("the item is under the heading") { self.bodies() == ["first", "third", "# Stash", "second"] }
    }

    // 項目のメニューでtransclusionを外すと、その項目は空の項目になる。見せていた項目は残る。
    func testATransclusionRemovedFromItsMenuLeavesAnEmptyItemAndTheItemItShowedStays() throws {
        let shows = " transclude=\"2020-01-01-00-00-00-AAAAAAB\""
        try download("notes.md", above("A") + "first\n\n" + above("B") + "second\n\n" + above("C", shows))
        try openNotes()
        waitUntil("the transclusion is shown") { self.shell.view.outline.rows.count == 3 }
        let transclusion = shell.view.outline.rows[2]
        XCTAssertTrue(transclusion.isTransclusion)

        shell.removeTransclusion(transclusion.item.id)

        waitUntil("the transclusion is gone") { self.shell.view.outline.rows.last?.isTransclusion == false }
        XCTAssertEqual(bodies(), ["first", "second", ""])
    }

    // 最初の行での上の矢印で、カーソルは上の項目へ移る。行の中の位置は変わらない。
    func testNavigateUpMovesTheEditToTheItemAboveAtThePlace() throws {
        try editSecondOfThree()

        XCTAssertTrue(shell.press(.up, in: id(of: "second"), cursor(3, in: "second")))

        waitUntil("the item above is edited") { self.edited() == "first" }
        XCTAssertEqual(shell.view.outline.focusRequest?.cursor, 3)
    }

    // 最後の行での下の矢印で、カーソルは下の項目へ移る。
    func testNavigateDownMovesTheEditToTheItemBelowAtThePlace() throws {
        try editSecondOfThree()

        shell.press(.down, in: id(of: "second"), cursor(0, in: "second"))

        waitUntil("the item below is edited") { self.edited() == "third" }
        XCTAssertEqual(shell.view.outline.focusRequest?.cursor, 0)
    }

    /// Opens a page whose third item shows the second one, and joins the second to the first
    /// with Backspace at its start.
    private func deleteAnItemThatAnotherShows() throws {
        let shows = " transclude=\"2020-01-01-00-00-00-AAAAAAB\""
        try download("notes.md", above("A") + "first\n\n" + above("B") + "second\n\n" + above("C", shows))
        try openNotes()
        let second = shell.view.outline.rows[1].item.id
        shell.edit(second)
        shell.press(.backspace, in: second, cursor(0, in: "second"))
        waitUntil("the core asks") { self.shell.question != nil }
    }

    // ほかの項目が写している項目を消すときは、先に尋ねる。消すと失うものがある問いである。いいえなら、項目は残る。
    func testTheCoreAsksBeforeItDeletesAnItemThatAnotherShows() throws {
        try deleteAnItemThatAnotherShows()
        XCTAssertEqual(shell.question?.destructive, true)

        shell.answer(yes: false)

        XCTAssertNil(shell.question)
        XCTAssertEqual(shell.view.outline.rows.count, 3)
    }

    // 尋ねられて、はいと答えると、項目は消える。
    func testTheItemThatAnotherShowsIsDeletedWhenTheUserSaysSo() throws {
        try deleteAnItemThatAnotherShows()

        shell.answer(yes: true)

        waitUntil("the item is gone") { self.shell.view.outline.rows.count < 3 }
        XCTAssertNil(shell.question)
    }

    // assetsのファイルへのリンクは、コアがブラウザで開くように命じても、アプリの中でそのファイルを開く。アプリにはサーバーがないため。ファイルがなければ、ないことが残り、次のリンクで消える（ADR 083）。
    func testALinkToAFileOfTheAssetsFolderOpensTheFileOrSaysThatItIsNotHere() throws {
        try FileManager.default.createDirectory(
            at: markdown.appendingPathComponent("assets"), withIntermediateDirectories: true)
        try Data("pdf".utf8).write(to: markdown.appendingPathComponent("assets/report a.pdf"))
        try start()

        shell.follow(.file(path: "assets/lost.pdf"))
        waitUntil("the file is said to be missing") { self.shell.missingFile == "assets/lost.pdf" }
        XCTAssertNil(shell.file)

        shell.follow(.file(path: "assets/Report a.pdf"))
        waitUntil("the file opens") { self.shell.file?.lastPathComponent == "report a.pdf" }
        XCTAssertNil(shell.missingFile)
        XCTAssertEqual(browsed, [])
    }

    // クラウドとの同期のスレッドは、コアが取り込みと書き出しを終えるまで待つ。
    func testTheThreadOfACloudSyncWaitsUntilTheCoreIsDone() throws {
        try start()
        try download("late.md", "came with a download\n")
        var done = false

        Thread.detachNewThread { [shell] in
            shell!.syncAndWait()
            DispatchQueue.main.async { done = true }
        }
        waitUntil("the core is done") { done }

        shell.search("late")
        waitUntil("the page is found") {
            self.shell.view.search.results.contains { $0.kind == .page && $0.title == "late" }
        }
    }
}
