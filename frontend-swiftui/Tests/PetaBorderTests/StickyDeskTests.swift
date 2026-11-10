import AppKit
import PetaBorder
import PetaScreens
import XCTest

/// The stickies of the app: each is a window with a page of its own. These need a real journal
/// server, so they are skipped without `PETA_E2E_SERVER`.
@MainActor
final class StickyDeskTests: XCTestCase {
    private var server: JournalServer!
    private var desk: StickyDesk!

    override func setUp() async throws {
        server = try JournalServer(port: 18098)
        desk = StickyDesk(server: server.address)
    }

    override func tearDown() async throws {
        desk?.stickies.forEach { $0.window.close() }
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

    // 付箋紙を足すと、ページのないウィンドウが1つ開く。
    func testAStickyThatIsAddedIsAWindowWithoutAPage() async throws {
        let sticky = desk.add()

        XCTAssertEqual(desk.stickies.count, 1)
        XCTAssertTrue(sticky.window.isVisible)
        XCTAssertNil(sticky.shell.url)
    }

    // 付箋紙は、それぞれが自分のページを持つ。1つでページを開いても、ほかの付箋紙は変わらない。
    func testEachStickyHasAPageOfItsOwn() async throws {
        let first = desk.add()
        let second = desk.add()

        second.shell.open("/pages/second-sticky")
        try await waitUntil("the page is shown") { second.shell.title == "second-sticky" }

        XCTAssertEqual(desk.stickies.count, 2)
        XCTAssertNil(first.shell.url)
        XCTAssertEqual(first.shell.view.outline.rows.count, 0)
        XCTAssertEqual(second.window.title, "second-sticky")
    }

    // 足した付箋紙は、前の付箋紙にぴったり重ならない。
    func testAStickyThatIsAddedDoesNotCoverTheOneBefore() async throws {
        let first = desk.add()
        let second = desk.add()

        XCTAssertNotEqual(first.window.frame.origin, second.window.frame.origin)
        XCTAssertEqual(first.window.frame.size, second.window.frame.size)
    }

    // 閉じた付箋紙は、なくなる。ほかの付箋紙は残る。
    func testAStickyThatIsClosedIsGone() async throws {
        let first = desk.add()
        let second = desk.add()

        first.window.close()

        XCTAssertEqual(desk.stickies.map(\.id), [second.id])
    }
}
