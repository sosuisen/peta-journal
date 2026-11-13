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

        shell.follow("/pages/other")

        waitUntil("the other page shows") { self.bodies() == ["what the other page says"] }
        XCTAssertNil(shell.missing)
    }

    // まだないページへのリンクに触れても、ページは作られない。ないことが見え、元のページが開いたままになる。
    func testALinkToAPageThatIsNotThereSaysSoAndMakesNoPage() throws {
        try download("notes.md", "[[nowhere]]\n")
        try openNotes()

        shell.follow("/pages/nowhere")

        XCTAssertEqual(shell.missing, "nowhere")
        XCTAssertEqual(bodies(), ["[[nowhere]]"])
        XCTAssertNil(written("docs/nowhere.md"))
        XCTAssertEqual(edits, 0)
    }

    // まだないページへのリンクに触れたあとに作ると、そのページができて開く。アプリに伝わり、ファイルが書かれる。
    func testThePageThatALinkNamedIsMadeWhenTheUserAsksForIt() throws {
        try download("notes.md", "[[nowhere]]\n")
        try openNotes()
        shell.follow("/pages/nowhere")

        shell.make()

        waitUntil("the new page is open") { self.title() == "nowhere" }
        XCTAssertNil(shell.missing)
        XCTAssertEqual(edits, 1)
        XCTAssertNotNil(written("docs/nowhere.md"))
    }

    private func title() -> String? {
        if case .page(let page) = shell.view.screen { return page?.title }
        return nil
    }

    // 入力した名前のページがなければ、作る結果が出る。選ぶと、ページができて開く。アプリに伝わり、ファイルが書かれる。
    func testTheResultForANewPageMakesThePageAndOpensIt() throws {
        try start()

        pick(.create, searchingFor: "fresh")

        waitUntil("the new page is open") { self.title() == "fresh" }
        XCTAssertEqual(edits, 1)
        XCTAssertNotNil(written("docs/fresh.md"))
    }

    // 「today」と打つと、今日のジャーナルを開く結果が出る。選ぶと、今日のジャーナルができて開く。
    func testTheResultForTodayMakesTheJournalOfTodayAndOpensIt() throws {
        try start()

        pick(.today, searchingFor: "today")

        waitUntil("the journal is open") { self.isJournal() }
        XCTAssertEqual(edits, 1)
    }

    private func isJournal() -> Bool {
        if case .journal = shell.view.screen { return true }
        return false
    }

    // 今日のジャーナルを頼むと、今日のジャーナルができて開く。アプリに伝わる。
    func testTodayMakesTheJournalOfTodayAndOpensIt() throws {
        try start()

        shell.today()

        waitUntil("the journal is open") { self.isJournal() }
        XCTAssertEqual(edits, 1)
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

    // 前に見ていたページがなければ、今日のジャーナルが出る。
    func testTheAppOpensAtTodayWhenItShowedNoPageBefore() throws {
        try start()

        shell.restore()

        waitUntil("the journal is open") { self.isJournal() }
    }

    // 前に見ていたページが消えていたら、今日のジャーナルが出る。
    func testTheAppOpensAtTodayWhenThePageItShowedHasGone() throws {
        try download("notes.md", "what the notes say\n")
        try openNotes()
        shell.close()
        try FileManager.default.removeItem(at: markdown.appendingPathComponent("docs/notes.md"))
        waitUntil("the journal can be opened again") {
            guard let next = try? Shell(dataDir: self.root.appendingPathComponent("data"), markdownDir: self.markdown)
            else { return false }
            self.shell = next
            return true
        }
        takeIn()

        shell.restore()

        waitUntil("the journal is open") { self.isJournal() }
        XCTAssertNil(shell.missing)
    }

    // 入れたばかりのアプリは、作らないと言われたら、今日のジャーナルを作らず、何も開かない。アプリには何も伝わらない。
    func testANewAppThatIsNotToMakeTodayOpensNoScreen() throws {
        try start()

        shell.restore(make: false)

        XCTAssertTrue(shell.unopened)
        XCTAssertFalse(isJournal())
        XCTAssertEqual(edits, 0)
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

    // 前に見ていたページがあるアプリは、作らないと言われても、そのページを開く。
    func testAnAppThatShowedAPageOpensItAlsoWhenItIsNotToMakeToday() throws {
        try download("notes.md", "what the notes say\n")
        try openNotes()
        shell.close()
        waitUntil("the journal can be opened again") {
            guard let next = try? Shell(dataDir: self.root.appendingPathComponent("data"), markdownDir: self.markdown)
            else { return false }
            self.shell = next
            return true
        }

        shell.restore(make: false)

        waitUntil("the page shows again") { self.bodies() == ["what the notes say"] }
        XCTAssertFalse(shell.unopened)
    }

    // 項目に触れると、その項目が編集に入る。カーソルの依頼は、触れたことによるものと分かる。
    func testATouchedItemIsEditedAndItsFocusRequestIsKnownAsTheTouch() throws {
        try download("notes.md", "a **bold** word\n")
        try openNotes()

        shell.edit(id(of: "a **bold** word"))

        XCTAssertEqual(edited(), "a **bold** word")
        XCTAssertNotNil(shell.touched)
        XCTAssertEqual(shell.touched, shell.view.outline.focusRequest?.seq)
    }

    // 打った文字は、ページのファイルに書かれる。
    func testWhatIsTypedGoesIntoTheFileOfThePage() throws {
        try download("notes.md", "first\n")
        try openNotes()
        shell.edit(id(of: "first"))

        shell.type(id(of: "first"), "first and more")

        waitUntil("the file holds what was typed") { self.written()?.contains("first and more") == true }
    }

    // 改行は、カーソルの位置で項目を分ける。編集は、新しい項目に移る。
    func testEnterSplitsTheItemAtTheCursorAndTheEditGoesOnInTheNewItem() throws {
        try download("notes.md", "first\n")
        try openNotes()
        shell.edit(id(of: "first"))

        shell.enter(id(of: "first"), cursor: 5)

        waitUntil("there are two items") { self.bodies() == ["first", ""] }
        waitUntil("the new item is edited") { self.edited() == "" }
    }

    // 絵文字のあとの改行も、文字の数で数えた位置で分ける。
    func testEnterCountsTheCursorInCharacters() throws {
        try download("notes.md", "😀ab\n")
        try openNotes()
        shell.edit(id(of: "😀ab"))

        shell.enter(id(of: "😀ab"), cursor: 2)

        waitUntil("the item is split after the second character") { self.bodies() == ["😀a", "b"] }
    }

    // ほかの項目に触れると、編集がそこへ移る。前の項目の編集は終わる。
    func testATouchOnAnotherItemMovesTheEditThere() throws {
        try editSecondOfThree()

        shell.edit(id(of: "third"))

        XCTAssertEqual(shell.view.outline.rows.filter(\.isEditing).map(\.item.body), ["third"])
    }

    // 編集を終えると、どの項目も編集中でなくなる。
    func testFinishEndsTheEdit() throws {
        try editSecondOfThree()

        shell.finish(id(of: "second"))

        XCTAssertNil(edited())
    }

    // ページを変える操作は、そのたびにアプリに伝わる。開くだけ、触れるだけでは伝わらない。
    func testEveryChangeOfAPageIsToldToTheApp() throws {
        try editSecondOfThree()
        XCTAssertEqual(edits, 0)

        shell.type(id(of: "second"), "second!")
        XCTAssertEqual(edits, 1)
        shell.indent(id(of: "second!"))
        shell.cycleTask(id(of: "second!"))

        XCTAssertEqual(edits, 3)
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
        waitUntil("the item is first") { self.bodies() == ["second", "first", "third"] }
        shell.moveDown(id(of: "second"))
        shell.moveDown(id(of: "second"))

        waitUntil("the item is last") { self.bodies() == ["first", "third", "second"] }
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

    // 削除は、項目を消す。
    func testDeleteDeletesTheItem() throws {
        try editSecondOfThree()

        shell.delete(id(of: "second"))

        waitUntil("the item is gone") { self.items() == ["1 first", "1 third"] }
    }

    /// Opens a page whose third item shows the second one, and asks to delete the second.
    private func deleteAnItemThatAnotherShows() throws {
        let shows = " transclude=\"2020-01-01-00-00-00-AAAAAAB\""
        try download("notes.md", above("A") + "first\n\n" + above("B") + "second\n\n" + above("C", shows))
        try openNotes()
        shell.delete(shell.view.outline.rows[1].item.id)
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

    // ページの画像のファイルは、名前の大文字と小文字を問わずに見つかる。
    func testTheFileOfAnImageIsFoundWithoutRegardToCase() throws {
        try FileManager.default.createDirectory(at: markdown.appendingPathComponent("assets"), withIntermediateDirectories: true)
        try Data("png".utf8).write(to: markdown.appendingPathComponent("assets/Picture.PNG"))
        try start()

        XCTAssertEqual(shell.image("assets/picture.png")?.lastPathComponent, "Picture.PNG")
        XCTAssertNil(shell.image("assets/other.png"))
        XCTAssertNil(shell.image("assets"))
    }

    // assetsのファイルへのリンクは、そのファイルを答える。ファイルがなければ、ないことが残り、次のリンクで消える。
    func testALinkToAFileOfTheAssetsFolderGivesTheFileOrSaysThatItIsNotHere() throws {
        try FileManager.default.createDirectory(at: markdown.appendingPathComponent("assets"), withIntermediateDirectories: true)
        try Data("pdf".utf8).write(to: markdown.appendingPathComponent("assets/report.pdf"))
        try start()

        XCTAssertNil(shell.attached("assets/lost.pdf"))
        XCTAssertEqual(shell.missingFile, "assets/lost.pdf")
        XCTAssertEqual(shell.attached("assets/Report.pdf")?.lastPathComponent, "report.pdf")
        XCTAssertNil(shell.missingFile)
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
        waitUntil("the page is found") { self.shell.view.search.results.contains { $0.kind == .page && $0.title == "late" } }
    }
}
