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
}
