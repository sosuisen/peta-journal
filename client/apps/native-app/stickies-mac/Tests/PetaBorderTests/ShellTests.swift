import AppKit
import Foundation
import PetaCore
import PetaTypes
import XCTest

@testable import PetaBorder

/// The shell against a real journal server. These are skipped without `PETA_E2E_SERVER`.
@MainActor
final class ShellTests: XCTestCase {
    /// An extension of the server that asks the clients, as an extension of the user may. It
    /// asks when a page gets a tag that starts with "ask", and adds an item after a yes.
    private static let askingExtension = """
        globalThis.onPageTagAdd = (page, tags) => {
            if (!tags.some(tag => tag.startsWith('ask'))) return;
            if (peta.confirm('Insert into ' + page.fileName + '?')) {
                peta.createItem({ position: 'last', level: 1, body: 'confirmed' });
            }
        };
        """

    private var server: JournalServer!

    override func setUp() async throws {
        server = try JournalServer(port: 18097, functions: ["asking": Self.askingExtension])
    }

    override func tearDown() async throws {
        server?.stop()
    }

    /// Waits for what the core and the server do. A test that waits in vain fails here.
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

    /// The page on screen, once it is there.
    private func page(_ shell: Shell) -> PageView? {
        if case .page(let page?) = shell.view.screen { return page }
        return nil
    }

    // ページを開くと、その行が画面に出る。新しいページには、空の項目が1つある。
    func testAPageIsOpenedAndItsRowsAreShown() async throws {
        let shell = Shell(server: server.address, url: "/pages/swift-notes")

        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }

        XCTAssertEqual(page(shell)?.page.name, "swift-notes")
        XCTAssertEqual(shell.view.outline.rows.map(\.item.body), [""])
        XCTAssertEqual(shell.title, "swift-notes")
    }

    // 項目に入力すると、少しあとにサーバーに保存される。
    func testTypingIntoAnItemIsSavedOnTheServer() async throws {
        let shell = Shell(server: server.address, url: "/pages/swift-notes")
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }
        let item = shell.view.outline.rows[0].item
        let pageId = try XCTUnwrap(page(shell)).page.id

        shell.startEditing(item.id)
        XCTAssertEqual(shell.view.outline.focusedItemId, item.id)
        shell.typed("typed in SwiftUI 日本語", into: item.id)

        XCTAssertEqual(shell.view.outline.rows[0].item.body, "typed in SwiftUI 日本語")
        let server = try XCTUnwrap(self.server)
        try await waitUntil("the server has the text") {
            let saved = (try? server.get("/api/pages/\(pageId)")) ?? Data()
            return String(decoding: saved, as: UTF8.self).contains("typed in SwiftUI 日本語")
        }
    }

    // シェルを閉じると、保存を待っていた本文が、閉じる前にサーバーに保存される（#766）。
    func testClosingSavesTheTextStillWaitingToBeSaved() async throws {
        let shell = Shell(server: server.address, url: "/pages/swift-closing")
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }
        let item = shell.view.outline.rows[0].item
        let pageId = try XCTUnwrap(page(shell)).page.id
        shell.startEditing(item.id)
        shell.typed("typed before closing", into: item.id)

        shell.close()

        let saved = try server.get("/api/pages/\(pageId)")
        XCTAssertTrue(String(decoding: saved, as: UTF8.self).contains("typed before closing"))
    }

    // 戻ると、前に開いたページがまた開き、付箋紙が覚えるURLもそのページになる。進むと、戻る前のページに戻る（ADR 035）。
    func testBackAndForwardFollowThePagesOpenedAndTheUrlFollows() async throws {
        let shell = Shell(server: server.address, url: "/pages/first")
        try await waitUntil("the first page is shown") { self.page(shell)?.page.name == "first" }
        XCTAssertFalse(shell.view.history.canGoBack)
        shell.open("/pages/second")
        try await waitUntil("the second page is shown") { self.page(shell)?.page.name == "second" }

        shell.goBack()

        try await waitUntil("the first page is shown again") { self.page(shell)?.page.name == "first" }
        XCTAssertEqual(shell.url, "/pages/first")
        XCTAssertTrue(shell.view.history.canGoForward)

        shell.goForward()

        try await waitUntil("the second page is shown again") { self.page(shell)?.page.name == "second" }
        XCTAssertEqual(shell.url, "/pages/second")
    }

    // 開始記号を打ち終えると、コアが閉じる記号を補う（ADR 050、ADR 057）。
    func testTypingAnOpeningClosesIt() async throws {
        let shell = Shell(server: server.address, url: "/pages/swift-notes")
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }
        let item = shell.view.outline.rows[0].item.id

        shell.startEditing(item)
        shell.typed(">>", into: item, cursor: 2)
        shell.typed(">>>", into: item, cursor: 3)

        XCTAssertEqual(shell.view.outline.rows[0].item.body, ">>>\n>>>")
    }

    // 表示した文字に触れると、その項目の編集に入り、カーソルは触れた文字のMarkdownでの位置に置かれる（ADR 043）。
    func testATouchOnTheShownTextPutsTheCursorAtItsPlaceInTheMarkdown() async throws {
        let shell = try await shellWithPage()
        let first = shell.view.outline.rows[0].item.id
        shell.startEditing(first)
        shell.typed("**bold** end", into: first)
        shell.finishEditing(first)
        try await waitUntil("the edit is over") { !shell.view.outline.rows[0].isEditing }

        // The shown text is "bold end": the touch is after "bol"
        shell.touched(first, letter: 3)

        try await waitUntil("the item is edited") { shell.view.outline.rows[0].isEditing }
        XCTAssertEqual(shell.view.outline.focusRequest?.itemId, first)
        XCTAssertEqual(shell.view.outline.focusRequest?.cursor, 5)
    }

    /// A shell with a page of its own on screen.
    private func shellWithPage(clipboard: @escaping () -> String? = { nil }) async throws -> Shell {
        let shell = Shell(server: server.address, url: "/pages/swift-notes", clipboard: clipboard)
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }
        return shell
    }

    // ページへのリンクをCommandを押してたどると、そのページを開く新しい付箋紙を頼む。この付箋紙のページは変わらず、ブラウザにも渡らない。
    func testALinkToAPageFollowedWithCommandAsksForANewStickyWithThePage() async throws {
        var asked: String?
        var browsed: URL?
        let shell = Shell(
            server: server.address, url: "/pages/swift-notes", clipboard: { nil },
            browse: { browsed = $0 }, openSticky: { asked = $0 })
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }

        shell.follow(.page(url: "/pages/apple-pie"), holding: .command)

        XCTAssertEqual(asked, "/pages/apple-pie")
        XCTAssertEqual(shell.url, "/pages/swift-notes")
        XCTAssertNil(browsed)
    }

    /// Types into the first item and presses Enter at the end of it, which makes a second item.
    private func twoItems(_ shell: Shell) async throws {
        let first = shell.view.outline.rows[0].item.id
        shell.startEditing(first)
        shell.typed("first", into: first)
        XCTAssertTrue(press(shell, .enter, in: first, editor("first", cursor: 5)))
        try await waitUntil("there are two items") { shell.view.outline.rows.count == 2 }
    }

    /// Waits until the shell hears what the server tells its clients. A change of another
    /// client before that would not reach it.
    private func hearsTheServer(_ shell: Shell) async throws {
        try await waitUntil("the shell hears the server") { shell.view.connection == .connected }
    }

    /// Gives the page the tag, as another client does.
    private func tag(_ pageId: String, _ tag: String) async throws {
        try await server.send("PUT", "/api/pages/\(pageId)/properties/tags", ["valueType": "tags", "value": tag])
    }

    // 作れない名前のページを検索から作ろうとすると、コアの知らせが出る。知らせは答えを待たないので、閉じても何も送らない（#754）。
    func testANoticeOfTheCoreShowsWhenAPageOfANameThatCannotBeMadeIsMade() async throws {
        let shell = try await shellWithPage()

        shell.search("a:b")
        try await waitUntil("the search offers to make the page") {
            shell.view.search.results.contains { $0.kind == .create }
        }
        let create = try XCTUnwrap(shell.view.search.results.first { $0.kind == .create })
        shell.pick(create.position)

        try await waitUntil("the core tells") { shell.question != nil }
        XCTAssertEqual(shell.question?.kind, .notice)
        XCTAssertTrue(shell.question?.message.contains(":") == true, shell.question?.message ?? "")
        shell.answer(yes: true)
        XCTAssertNil(shell.question)
        XCTAssertEqual(shell.url, "/pages/swift-notes")
    }

    // サーバーの拡張が確認を求めると、質問が出る。「いいえ」では何も起きず、「はい」で拡張が先へ進む。
    func testAConfirmationOfAnExtensionIsAnsweredWithYesOrNo() async throws {
        let shell = try await shellWithPage()
        let pageId = try XCTUnwrap(page(shell)).page.id
        try await hearsTheServer(shell)

        try await tag(pageId, "ask-first")
        try await waitUntil("the shell is asked to confirm") { shell.question != nil }
        XCTAssertEqual(shell.question?.message, "Insert into swift-notes?")
        XCTAssertEqual(shell.question?.kind, .confirm)
        shell.answer(yes: false)
        XCTAssertNil(shell.question)
        try await tag(pageId, "ask-second")
        try await waitUntil("the shell is asked again") { shell.question != nil }
        shell.answer(yes: true)

        // Only the yes added an item: the no added none before it. The item of the extension
        // takes the place of the empty item of the new page, as the server does with an item
        // placed last on a page whose only item is empty
        try await waitUntil("the item of the extension is shown") { self.bodies(shell).contains("confirmed") }
        XCTAssertEqual(bodies(shell), ["confirmed"])
    }

    // Enterで項目を分けると、コアが新しい項目にカーソルを置くように頼む。
    func testEnterAsksTheCursorIntoTheNewItem() async throws {
        let shell = try await shellWithPage()

        try await twoItems(shell)

        let second = shell.view.outline.rows[1].item.id
        try await waitUntil("the cursor is asked into the new item") { shell.view.outline.focusOrder?.itemId == second }
    }

    // コアがクリップボードを求めると、その文字列で答える。項目のIDなら、トランスクルージョンになる。
    func testTheClipboardIsReadWhenTheCoreAsks() async throws {
        var copied: String?
        let shell = try await shellWithPage(clipboard: { copied })
        try await twoItems(shell)
        let (first, second) = (shell.view.outline.rows[0].item.id, shell.view.outline.rows[1].item.id)
        copied = first

        shell.send(.pasteAsTransclusion(itemId: second))

        try await waitUntil("the second item shows the first") {
            shell.view.outline.rows.contains { $0.item.id == second && $0.item.transcludeItemId == first }
        }
    }

    // 画像のファイルを貼ると、サーバーのassetsに保存され、カーソルの位置にそのリンクが入る。ファイルは保存のあとに消える（ADR 071）。
    func testAPastedImageFileIsSavedInTheAssetsAndLinkedAtTheCursor() async throws {
        let shell = try await shellWithPage()
        let item = shell.view.outline.rows[0].item.id
        shell.startEditing(item)
        shell.typed("ab", into: item, cursor: 1)
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("peta-test-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let image = folder.appendingPathComponent("sample.png")
        try Data([1, 2, 3]).write(to: image)

        shell.pasteImage(image, into: item, body: "ab", cursor: 1)

        try await waitUntil("the item links the image after the a") {
            let body = shell.view.outline.rows[0].item.body
            return body.hasPrefix("a![sample.png](../assets/") && body.hasSuffix(")b")
        }
        try await waitUntil("the file is deleted") { !FileManager.default.fileExists(atPath: image.path) }
    }

    // ほかで表示されている項目を選んで消すときは、元に戻しにくい操作として確認を求められる。
    func testDeletingATranscludedItemIsAskedAsHardToUndo() async throws {
        let shell = try await shellWithPage()
        try await hearsTheServer(shell)
        try await twoItems(shell)
        let (first, second) = (shell.view.outline.rows[0].item.id, shell.view.outline.rows[1].item.id)
        // Enter left the cursor in the second item. While any item is edited, the core does not
        // read again a page that another client changed. It reads it once the edit ends
        shell.finishEditing(second)
        // Another client, such as the web frontend, shows the first item in the second
        try await server.send("POST", "/api/items/\(second)/transclude", ["sourceItemId": first])
        try await waitUntil("the second item shows the first") {
            shell.view.outline.rows.contains { $0.item.id == second && $0.item.transcludeItemId == first }
        }

        XCTAssertTrue(press(shell, .up, in: first, editor("first", cursor: 0), holding: .shift))
        // The key that a Mac calls delete
        press(shell, .backspace)

        try await waitUntil("the shell is asked to confirm") { shell.question != nil }
        XCTAssertEqual(shell.question?.kind, .confirmDestructive)
    }

    /// An editor of one line with the cursor at `cursor`. Which keys mean something where is
    /// tested in the core (ADR 052).
    private func editor(_ text: String, cursor: UInt64) -> EditorState {
        EditorState(text: text, selectionStart: cursor, selectionEnd: cursor, onFirstLine: true, onLastLine: true)
    }

    /// Presses a key in the editor of the item, as the app does: the editor hands the chord to
    /// the core, which decides what it means (ADR 052).
    private func press(
        _ shell: Shell, _ key: Key, in itemId: String, _ editor: EditorState,
        holding held: NSEvent.ModifierFlags = []
    ) -> Bool {
        shell.pressed(.inEditor(itemId: itemId, chord: Keyboard.chord(key, holding: held), editor: editor))
    }

    /// Presses a key while items are selected, as the app does.
    private func press(_ shell: Shell, _ key: Key, holding held: NSEvent.ModifierFlags = []) {
        _ = shell.pressed(.inSelection(chord: Keyboard.chord(key, holding: held)))
    }

    /// Chooses "Undo" or "Redo" of the Edit menu while the item is edited. The menu takes
    /// Command+Z before the editor, and names the key itself.
    private func choose(_ shell: Shell, _ key: EditorKey, in itemId: String, _ editor: EditorState) -> Bool {
        shell.pressed(.named(itemId: itemId, key: key, editor: editor))
    }

    private func bodies(_ shell: Shell) -> [String] {
        shell.view.outline.rows.map(\.item.body)
    }

    // Enterは、カーソルの位置で項目を分ける。カーソルは、新しい項目の先頭に移る。
    func testEnterSplitsTheItemAtTheCursor() async throws {
        let shell = try await shellWithPage()
        let first = shell.view.outline.rows[0].item.id
        shell.startEditing(first)
        shell.typed("日本語second", into: first)

        XCTAssertTrue(press(shell, .enter, in: first, editor("日本語second", cursor: 3)))

        try await waitUntil("the item is split") { self.bodies(shell) == ["日本語", "second"] }
        let second = shell.view.outline.rows[1].item.id
        try await waitUntil("the cursor is asked into the new item") { shell.view.outline.focusOrder?.itemId == second }
    }

    // Tabは項目を字下げし、Shift+Tabは字上げする。
    func testTabIndentsAndShiftTabOutdents() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let second = shell.view.outline.rows[1].item
        XCTAssertEqual(second.level, shell.view.outline.rows[0].item.level)

        XCTAssertTrue(press(shell, .tab, in: second.id, editor("", cursor: 0)))
        XCTAssertEqual(shell.view.outline.rows[1].item.level, second.level + 1)
        XCTAssertTrue(press(shell, .tab, in: second.id, editor("", cursor: 0), holding: .shift))
        XCTAssertEqual(shell.view.outline.rows[1].item.level, second.level)
    }

    // 最初の行での上矢印は、前の項目へ移る。カーソルの位置は保たれる。
    func testUpOnTheFirstLineGoesToTheItemBefore() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let (first, second) = (shell.view.outline.rows[0].item.id, shell.view.outline.rows[1].item.id)
        shell.typed("second", into: second)

        XCTAssertTrue(press(shell, .up, in: second, editor("second", cursor: 2)))

        try await waitUntil("the cursor is asked into the first item") {
            shell.view.outline.focusRequest?.itemId == first
        }
        XCTAssertEqual(shell.view.outline.focusRequest?.cursor, 2)
    }

    // 最後の行での下矢印は、次の項目へ移る。
    func testDownOnTheLastLineGoesToTheItemAfter() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let (first, second) = (shell.view.outline.rows[0].item.id, shell.view.outline.rows[1].item.id)
        shell.startEditing(first)

        XCTAssertTrue(press(shell, .down, in: first, editor("first", cursor: 1)))

        try await waitUntil("the cursor is asked into the second item") {
            shell.view.outline.focusRequest?.itemId == second
        }
    }

    // 先頭での左矢印は、前の項目の末尾へ移る。末尾での右矢印は、次の項目の先頭へ移る。
    func testLeftAtTheStartAndRightAtTheEndLeaveTheItem() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let (first, second) = (shell.view.outline.rows[0].item.id, shell.view.outline.rows[1].item.id)

        XCTAssertTrue(press(shell, .left, in: second, editor("", cursor: 0)))
        try await waitUntil("the cursor is asked to the end of the first item") {
            shell.view.outline.focusRequest?.itemId == first
        }
        let asked = try XCTUnwrap(shell.view.outline.focusRequest?.cursor)
        // The core asks for the end, which the editor finds in its text
        XCTAssertEqual(utf16OfChars(text: "first", chars: asked), 5)

        XCTAssertTrue(press(shell, .right, in: first, editor("first", cursor: 5)))
        try await waitUntil("the cursor is asked to the start of the second item") {
            shell.view.outline.focusRequest?.itemId == second
        }
        XCTAssertEqual(shell.view.outline.focusRequest?.cursor, 0)
    }

    // 先頭でのBackspaceは、項目を前の項目につなぐ。
    func testBackspaceAtTheStartJoinsTheItemToTheOneBefore() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let second = shell.view.outline.rows[1].item.id
        shell.typed("second", into: second)

        XCTAssertTrue(press(shell, .backspace, in: second, editor("second", cursor: 0)))

        try await waitUntil("the items are joined") { self.bodies(shell) == ["firstsecond"] }
    }

    // 末尾でのDeleteは、次の項目をつなぐ。
    func testDeleteAtTheEndJoinsTheNextItem() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let (first, second) = (shell.view.outline.rows[0].item.id, shell.view.outline.rows[1].item.id)
        shell.typed("second", into: second)
        shell.startEditing(first)

        XCTAssertTrue(press(shell, .delete, in: first, editor("first", cursor: 5)))

        try await waitUntil("the items are joined") { self.bodies(shell) == ["firstsecond"] }
    }

    // タスクの切り替えは、項目をタスクにし、押すたびに状態を進める。
    func testTheTaskKeyMakesTheItemATaskAndStepsItsState() async throws {
        let shell = try await shellWithPage()
        let first = shell.view.outline.rows[0].item.id
        shell.startEditing(first)
        shell.typed("to do", into: first)
        XCTAssertNil(shell.view.outline.rows[0].task)

        XCTAssertTrue(press(shell, .enter, in: first, editor("to do", cursor: 0), holding: .command))
        try await waitUntil("the item is a task") { shell.view.outline.rows[0].task == .notStarted }
        XCTAssertTrue(press(shell, .enter, in: first, editor("to do", cursor: 0), holding: .command))
        try await waitUntil("the task is running") { shell.view.outline.rows[0].task == .running }
    }

    // タスクの印をクリックすると、タスクの状態が進む。項目を編集していなくてもよい。
    func testAClickOnTheMarkOfATaskStepsItsState() async throws {
        let shell = try await shellWithPage()
        let first = shell.view.outline.rows[0].item.id
        shell.startEditing(first)
        shell.typed("to do", into: first)
        XCTAssertTrue(press(shell, .enter, in: first, editor("to do", cursor: 0), holding: .command))
        try await waitUntil("the item is a task") { shell.view.outline.rows[0].task == .notStarted }
        shell.finishEditing(first)

        shell.clickedTask(of: first)
        try await waitUntil("the task is running") { shell.view.outline.rows[0].task == .running }
        shell.clickedTask(of: first)
        try await waitUntil("the task is done") { shell.view.outline.rows[0].task == .done }
    }

    // 子を隠した項目のバレットは、二重の円である。子を表示すると、ふつうのバレットに戻る。
    func testTheBulletOfAnItemWithHiddenChildrenIsADoubleCircle() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let (first, second) = (shell.view.outline.rows[0].item.id, shell.view.outline.rows[1].item.id)
        XCTAssertTrue(press(shell, .tab, in: second, editor("", cursor: 0)))
        XCTAssertEqual(shell.view.outline.rows[0].marks, ["•"])

        XCTAssertTrue(press(shell, .up, in: first, editor("first", cursor: 0), holding: .command))
        try await waitUntil("the child is hidden") { shell.view.outline.rows.count == 1 }
        XCTAssertEqual(shell.view.outline.rows[0].marks, ["◉"])

        XCTAssertTrue(press(shell, .down, in: first, editor("first", cursor: 0), holding: .command))
        try await waitUntil("the child is shown") { shell.view.outline.rows.count == 2 }
        XCTAssertEqual(shell.view.outline.rows[0].marks, ["•"])
    }

    // 折りたたむキーは子の項目を隠し、開くキーは子の項目を表示する。切り替えではない。
    func testTheFoldKeyHidesTheChildrenAndTheUnfoldKeyShowsThem() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let (first, second) = (shell.view.outline.rows[0].item.id, shell.view.outline.rows[1].item.id)
        XCTAssertTrue(press(shell, .tab, in: second, editor("", cursor: 0)))

        // The children are shown already
        XCTAssertTrue(press(shell, .down, in: first, editor("first", cursor: 0), holding: .command))
        XCTAssertFalse(shell.view.outline.rows[0].isCollapsed)
        XCTAssertTrue(press(shell, .up, in: first, editor("first", cursor: 0), holding: .command))
        try await waitUntil("the child is hidden") { shell.view.outline.rows.count == 1 }
        XCTAssertTrue(shell.view.outline.rows[0].isCollapsed)
        // The children are hidden already
        XCTAssertTrue(press(shell, .up, in: first, editor("first", cursor: 0), holding: .command))
        XCTAssertTrue(shell.view.outline.rows[0].isCollapsed)
        XCTAssertTrue(press(shell, .down, in: first, editor("first", cursor: 0), holding: .command))
        try await waitUntil("the child is shown again") { shell.view.outline.rows.count == 2 }
    }

    // 子のない項目には、折りたたむものがない。
    func testAnItemWithoutChildrenIsNotFolded() async throws {
        let shell = try await shellWithPage()
        let first = shell.view.outline.rows[0].item.id

        XCTAssertTrue(press(shell, .up, in: first, editor("", cursor: 0), holding: .command))
        XCTAssertFalse(shell.view.outline.rows[0].isCollapsed)
    }

    // 項目を下へ動かすと、次の項目と入れ替わる。上へ動かすと、戻る。
    func testAnItemIsMovedDownAndUp() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let (first, second) = (shell.view.outline.rows[0].item.id, shell.view.outline.rows[1].item.id)
        shell.typed("second", into: second)
        shell.startEditing(first)

        XCTAssertTrue(press(shell, .down, in: first, editor("first", cursor: 0), holding: [.shift, .option]))
        try await waitUntil("the first item is below") { self.bodies(shell) == ["second", "first"] }
        XCTAssertTrue(press(shell, .up, in: first, editor("first", cursor: 0), holding: [.shift, .option]))
        try await waitUntil("the first item is on top again") { self.bodies(shell) == ["first", "second"] }
    }

    // 項目を右へ動かすと字下げになり、左へ動かすと字上げになる。
    func testAnItemIsMovedInAndOut() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let second = shell.view.outline.rows[1].item

        XCTAssertTrue(press(shell, .right, in: second.id, editor("", cursor: 0), holding: [.shift, .option]))
        XCTAssertEqual(shell.view.outline.rows[1].item.level, second.level + 1)
        XCTAssertTrue(press(shell, .left, in: second.id, editor("", cursor: 0), holding: [.shift, .option]))
        XCTAssertEqual(shell.view.outline.rows[1].item.level, second.level)
    }

    // 元に戻すと、直前の操作が取り消される。やり直すと、もう一度行われる。
    func testUndoTakesBackTheLastChangeAndRedoMakesItAgain() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let second = shell.view.outline.rows[1].item
        XCTAssertTrue(press(shell, .tab, in: second.id, editor("", cursor: 0)))
        XCTAssertEqual(shell.view.outline.rows[1].item.level, second.level + 1)

        XCTAssertTrue(choose(shell, .undo, in: second.id, editor("", cursor: 0)))
        try await waitUntil("the indent is taken back") {
            shell.view.outline.rows[1].item.level == second.level
        }
        XCTAssertTrue(choose(shell, .redo, in: second.id, editor("", cursor: 0)))
        try await waitUntil("the indent is made again") {
            shell.view.outline.rows[1].item.level == second.level + 1
        }
    }

    /// Two items, "first" and "second", with the second one selected from its editor.
    private func secondOfTwoSelected(_ shell: Shell) async throws {
        try await twoItems(shell)
        let second = shell.view.outline.rows[1].item.id
        shell.typed("second", into: second)
        XCTAssertTrue(press(shell, .down, in: second, editor("second", cursor: 0), holding: .shift))
    }

    private func selected(_ shell: Shell) -> [String] {
        shell.view.outline.rows.filter(\.isSelected).map(\.item.body)
    }

    // 最初の行でのShift+上は、その項目を選ぶ。
    func testShiftArrowOnTheEdgeLineSelectsTheItem() async throws {
        let shell = try await shellWithPage()
        try await twoItems(shell)
        let first = shell.view.outline.rows[0].item.id

        XCTAssertTrue(press(shell, .up, in: first, editor("first", cursor: 0), holding: .shift))

        XCTAssertTrue(shell.view.outline.selectionMode)
        XCTAssertEqual(selected(shell), ["first"])
    }

    // 選んでいるあいだのShift+上下は、選択を広げる。Escapeは、選択をやめる。
    func testTheSelectionGrowsAndEscapeEndsIt() async throws {
        let shell = try await shellWithPage()
        try await secondOfTwoSelected(shell)

        press(shell, .up, holding: .shift)
        XCTAssertEqual(selected(shell), ["first", "second"])
        press(shell, .escape)

        XCTAssertFalse(shell.view.outline.selectionMode)
        XCTAssertEqual(selected(shell), [])
        XCTAssertEqual(bodies(shell), ["first", "second"])
    }

    // 選んでいるあいだに項目の外をクリックすると、選択が消える。項目は消えない。
    func testAClickOutsideTheItemsEndsTheSelection() async throws {
        let shell = try await shellWithPage()
        try await secondOfTwoSelected(shell)

        shell.clickedOutsideItems()

        XCTAssertFalse(shell.view.outline.selectionMode)
        XCTAssertEqual(selected(shell), [])
        XCTAssertEqual(bodies(shell), ["first", "second"])
    }

    // 選んだ項目を消せる。
    func testTheSelectedItemsAreDeleted() async throws {
        let shell = try await shellWithPage()
        try await secondOfTwoSelected(shell)

        press(shell, .backspace)

        try await waitUntil("the second item is gone") { self.bodies(shell) == ["first"] }
        XCTAssertFalse(shell.view.outline.selectionMode)
    }

    // 選んだ項目を上へ動かせる。
    func testTheSelectedItemsAreMoved() async throws {
        let shell = try await shellWithPage()
        try await secondOfTwoSelected(shell)

        press(shell, .up, holding: [.shift, .option])

        try await waitUntil("the second item is on top") { self.bodies(shell) == ["second", "first"] }
        XCTAssertEqual(selected(shell), ["second"])
    }

    // 選んだ項目をコピーすると、HTMLもクリップボードに載る。ウィキリンクとハッシュタグは、ブラウザ版のページを開くリンクになる。Markdownのほうは変わらない。
    func testCopyAlsoPutsHTMLOnTheClipboardWithLinksThatOpenInTheBrowser() async throws {
        let copied = Copied()
        let shell = Shell(
            server: server.address, url: "/pages/swift-notes", clipboard: { nil },
            copy: { text, html in
                copied.text = text
                copied.html = html
            })
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }
        let item = shell.view.outline.rows[0].item.id
        shell.startEditing(item)
        shell.typed("see [[Idea]] #topic", into: item)
        XCTAssertTrue(press(shell, .down, in: item, editor("see [[Idea]] #topic", cursor: 0), holding: .shift))

        shell.selection(.copy)

        try await waitUntil("the clipboard has the item") { copied.text != nil }
        XCTAssertEqual(copied.text, "- see [[Idea]] #topic")
        let tags = shell.view.config.tagsDir
        XCTAssertEqual(
            copied.html,
            "<ul>\n<li>see <a href=\"\(server.address)/pages/Idea\">Idea</a> "
                + "<a href=\"\(server.address)/pages/\(tags)topic\">#topic</a></li>\n</ul>\n")
    }

    // 選んだ項目をコピーすると、Markdownがクリップボードに載る。
    func testCopyPutsTheSelectedItemsOnTheClipboard() async throws {
        let copied = Copied()
        let shell = Shell(
            server: server.address, url: "/pages/swift-notes", clipboard: { nil },
            copy: { text, _ in copied.text = text })
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }
        try await secondOfTwoSelected(shell)
        press(shell, .up, holding: .shift)

        shell.selection(.copy)

        try await waitUntil("the clipboard has the items") { copied.text != nil }
        XCTAssertEqual(copied.text, "- first\n- second")
        XCTAssertEqual(bodies(shell), ["first", "second"])
    }

    /// A shell on the page "swift-notes", with a second page "apple-pie" on the server.
    private func shellWithTwoPages() async throws -> Shell {
        let shell = try await shellWithPage()
        shell.open("/pages/apple-pie")
        try await waitUntil("the second page is shown") { shell.title == "apple-pie" }
        shell.open("/pages/swift-notes")
        try await waitUntil("the first page is shown again") { shell.title == "swift-notes" }
        return shell
    }

    private func pagesFound(_ shell: Shell) -> [String] {
        shell.view.search.results.filter { $0.kind == .page }.map(\.title)
    }

    // 検索欄に入力すると、少しあとにページの候補が出る。候補を選ぶと、そのページが開き、検索欄は空になる。
    func testASearchFindsAPageAndPickingItOpensThePage() async throws {
        let shell = try await shellWithTwoPages()

        shell.search("apple")
        XCTAssertEqual(shell.view.search.query, "apple")
        try await waitUntil("the page is found") { self.pagesFound(shell) == ["apple-pie"] }
        XCTAssertTrue(shell.view.search.showDropdown)
        shell.pick(shell.view.search.results[0].position)

        try await waitUntil("the page found is shown") { shell.title == "apple-pie" }
        XCTAssertEqual(shell.view.search.query, "")
        XCTAssertFalse(shell.view.search.showDropdown)
    }

    // 検索の候補は、下矢印で選び、Enterで開く。
    func testAFoundPageIsChosenWithDownAndOpenedWithEnter() async throws {
        let shell = try await shellWithTwoPages()
        shell.search("apple")
        try await waitUntil("the page is found") { self.pagesFound(shell) == ["apple-pie"] }

        shell.search(key: .down)
        XCTAssertEqual(shell.view.search.selectedIndex, 0)
        shell.search(key: .enter)

        try await waitUntil("the page found is shown") { shell.title == "apple-pie" }
    }

    // Escapeは、検索の候補を隠す。
    func testEscapeHidesWhatWasFound() async throws {
        let shell = try await shellWithTwoPages()
        shell.search("apple")
        try await waitUntil("the page is found") { self.pagesFound(shell) == ["apple-pie"] }

        shell.search(key: .escape)

        XCTAssertFalse(shell.view.search.showDropdown)
        XCTAssertEqual(shell.title, "swift-notes")
    }

    // ページを指定しないシェルは、何も開かない。検索はできて、選んだページが開く。開いた場所は、シェルが覚えている。
    func testAShellWithoutAPageOpensNothingUntilAPageIsFound() async throws {
        let other = try await shellWithPage()
        XCTAssertEqual(other.url, "/pages/swift-notes")
        let shell = Shell(server: server.address, url: nil, clipboard: { nil })
        XCTAssertNil(shell.url)
        XCTAssertEqual(shell.view.outline.rows.count, 0)

        shell.search("swift")
        try await waitUntil("the page is found") { self.pagesFound(shell) == ["swift-notes"] }
        shell.pick(shell.view.search.results[0].position)

        try await waitUntil("the page found is shown") { shell.title == "swift-notes" }
        XCTAssertEqual(shell.url, "/pages/swift-notes")
    }

    /// The journal on screen, once it is there.
    private func journal(_ shell: Shell) -> JournalView? {
        if case .journal(let journal) = shell.view.screen { return journal }
        return nil
    }

    /// Makes the journal of a day long ago, with one item, and waits until the server has it.
    private func journalOfADayLongAgo() async throws {
        let old = Shell(server: server.address, url: "/pages/2020-01-02")
        try await waitUntil("the old journal is shown") { !old.view.outline.rows.isEmpty }
        let pageId = try XCTUnwrap(journal(old)?.pageId)
        let item = old.view.outline.rows[0].item.id
        old.startEditing(item)
        old.typed("written long ago", into: item)
        try await waitUntil("the old journal is saved") {
            let saved = (try? self.server.get("/api/pages/\(pageId)")) ?? Data()
            return String(decoding: saved, as: UTF8.self).contains("written long ago")
        }
    }

    // 今日のジャーナルのタイトルは、今日の日付に「(Today)」を付けたものである。
    func testTheTitleOfTheJournalOfTodayIsItsDateMarkedAsToday() async throws {
        let shell = Shell(server: server.address, url: "/today")
        try await waitUntil("the journal is shown") { !shell.view.outline.rows.isEmpty }

        let date = try XCTUnwrap(journal(shell)).displayDate
        XCTAssertFalse(date.isEmpty)
        XCTAssertEqual(shell.title, "\(date) (Today)")
    }

    // 今日でない日のジャーナルのタイトルは、その日付だけである。
    func testTheTitleOfTheJournalOfAnotherDayIsItsDateAlone() async throws {
        let shell = Shell(server: server.address, url: "/pages/2020-01-02")
        try await waitUntil("the journal is shown") { !shell.view.outline.rows.isEmpty }

        XCTAssertEqual(shell.title, "2020年1月2日 木曜日")
    }

    // 今日のジャーナルを前の日までスクロールすると、タイトルはその日の日付になり、「(Today)」は付かない。今日まで戻ると、また付く。
    func testScrollingTheJournalToAnEarlierDayShowsThatDateInTheTitle() async throws {
        try await journalOfADayLongAgo()
        let shell = Shell(server: server.address, url: "/today")
        try await waitUntil("the earlier day is shown under today") {
            shell.view.outline.rows.contains { $0.dateHeader != nil }
        }
        let earlier = try XCTUnwrap(shell.view.outline.rows.first { $0.dateHeader != nil })
        let today = try XCTUnwrap(journal(shell)).displayDate

        shell.scrolled(top: Int(earlier.position), shown: 0...Int(earlier.position))

        XCTAssertEqual(earlier.dateHeader, "2020年1月2日 木曜日")
        XCTAssertEqual(shell.title, "2020年1月2日 木曜日")

        shell.scrolled(top: 0, shown: 0...Int(earlier.position))

        XCTAssertEqual(shell.title, "\(today) (Today)")
    }
}

/// What the shell put on the clipboard.
@MainActor
private final class Copied {
    var text: String?
    var html: String?
}
