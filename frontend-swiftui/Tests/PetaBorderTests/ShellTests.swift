import Foundation
import PetaTypes
import XCTest

@testable import PetaBorder

/// The shell against a real journal server. These are skipped without `PETA_E2E_SERVER`.
@MainActor
final class ShellTests: XCTestCase {
    private var server: JournalServer!

    override func setUp() async throws {
        server = try JournalServer(port: 18097)
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

    /// A shell with a page of its own on screen.
    private func shellWithPage(clipboard: @escaping () -> String? = { nil }) async throws -> Shell {
        let shell = Shell(server: server.address, url: "/pages/swift-notes", clipboard: clipboard)
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }
        return shell
    }

    /// Types into the first item and presses Enter at the end of it, which makes a second item.
    private func twoItems(_ shell: Shell) async throws {
        let first = shell.view.outline.rows[0].item.id
        shell.startEditing(first)
        shell.typed("first", into: first)
        shell.send(.enterPressed(itemId: first, cursor: 5, nowMs: 0))
        try await waitUntil("there are two items") { shell.view.outline.rows.count == 2 }
    }

    // コアが入力を求めると、質問が出る。答えた文字列で、コアが先へ進む。
    func testAPromptIsAnsweredWithText() async throws {
        let shell = try await shellWithPage()

        shell.send(.createNewPage(searchQuery: ""))
        try await waitUntil("the shell is asked for a title") { shell.question != nil }
        XCTAssertEqual(shell.question?.message, "Page title:")
        XCTAssertEqual(shell.question?.kind, .prompt)
        shell.answer(text: "made-by-prompt")

        XCTAssertNil(shell.question)
        try await waitUntil("the new page is shown") { shell.title == "made-by-prompt" }
    }

    // コアが確認を求めると、質問が出る。「いいえ」では何も起きず、「はい」でコアが先へ進む。
    func testAConfirmationIsAnsweredWithYesOrNo() async throws {
        let shell = try await shellWithPage()
        let pageId = try XCTUnwrap(page(shell)).page.id

        shell.send(.deleteCurrentPage)
        try await waitUntil("the shell is asked to confirm") { shell.question != nil }
        XCTAssertEqual(shell.question?.message, "Delete \"swift-notes\"?")
        XCTAssertEqual(shell.question?.kind, .confirm)
        shell.answer(yes: false)
        XCTAssertNil(shell.question)
        shell.send(.deleteCurrentPage)
        try await waitUntil("the shell is asked again") { shell.question != nil }
        XCTAssertNoThrow(try server.get("/api/pages/\(pageId)"))
        shell.answer(yes: true)

        let server = try XCTUnwrap(self.server)
        try await waitUntil("the page is gone from the server") {
            (try? server.get("/api/pages/\(pageId)")) == nil
        }
    }

    // Enterで項目を分けると、コアが新しい項目にカーソルを置くように頼む。
    func testEnterAsksTheCursorIntoTheNewItem() async throws {
        let shell = try await shellWithPage()

        try await twoItems(shell)

        let second = shell.view.outline.rows[1].item.id
        try await waitUntil("the cursor is asked into the new item") { shell.focus?.itemId == second }
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

    // ほかで表示されている項目を消すときは、元に戻しにくい操作として確認を求められる。
    func testDeletingATranscludedItemIsAskedAsHardToUndo() async throws {
        var copied: String?
        let shell = try await shellWithPage(clipboard: { copied })
        try await twoItems(shell)
        let (first, second) = (shell.view.outline.rows[0].item.id, shell.view.outline.rows[1].item.id)
        copied = first
        shell.send(.pasteAsTransclusion(itemId: second))
        try await waitUntil("the second item shows the first") {
            shell.view.outline.rows.contains { $0.item.id == second && $0.item.transcludeItemId == first }
        }

        shell.send(.deleteItem(itemId: first))

        try await waitUntil("the shell is asked to confirm") { shell.question != nil }
        XCTAssertEqual(shell.question?.kind, .confirmDestructive)
    }
}
