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
    /// Where the stickies are kept between two runs of the app.
    private var file: URL!
    /// The desks of a test that plays a second run of the app.
    private var later: [StickyDesk] = []

    override func setUp() async throws {
        server = try JournalServer(port: 18098)
        file = FileManager.default.temporaryDirectory
            .appendingPathComponent("peta-stickies-\(UUID().uuidString)")
            .appendingPathComponent("stickies.json")
        desk = StickyDesk(server: server.address, store: StickyStore(file: file))
    }

    override func tearDown() async throws {
        for desk in [desk].compactMap({ $0 }) + later {
            desk.stickies.forEach { $0.window.close() }
        }
        server?.stop()
        try? FileManager.default.removeItem(at: file.deletingLastPathComponent())
    }

    /// The app as it starts the next time, with what the file kept.
    private func nextRun() -> StickyDesk {
        let desk = StickyDesk(server: server.address, store: StickyStore(file: file))
        later.append(desk)
        desk.open()
        return desk
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
        try await waitUntil("the window has the name of the app") { sticky.window.title == "PetaStickies" }
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
        try await waitUntil("the window has the name of the page") { second.window.title == "second-sticky" }
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

    // 何も保存されていないときは、ページのない付箋紙が1つ開く。
    func testTheDeskOpensOneStickyWhenNothingIsKept() async throws {
        desk.open()

        XCTAssertEqual(desk.stickies.count, 1)
        XCTAssertNil(desk.stickies[0].shell.url)
    }

    // 付箋紙のページ、位置、大きさは、次の起動で戻る。
    func testStickiesComeBackWithTheirPageAndTheirFrame() async throws {
        let sticky = desk.add()
        desk.add()
        sticky.shell.open("/pages/kept")
        try await waitUntil("the page is shown") { sticky.shell.title == "kept" }
        let frame = NSRect(x: 120, y: 130, width: 400, height: 500)
        sticky.window.setFrame(frame, display: true)

        let next = nextRun()

        XCTAssertEqual(next.stickies.map(\.id), desk.stickies.map(\.id))
        XCTAssertEqual(next.stickies[0].window.frame, frame)
        XCTAssertNil(next.stickies[1].shell.url)
        try await waitUntil("the page is shown again") { next.stickies[0].shell.title == "kept" }
    }

    // 閉じた付箋紙は、次の起動で戻らない。
    func testAStickyThatIsClosedDoesNotComeBack() async throws {
        let first = desk.add()
        let second = desk.add()

        first.window.close()

        XCTAssertEqual(nextRun().stickies.map(\.id), [second.id])
    }

    // アプリを終えるときに閉じられる付箋紙は、次の起動で戻る。アプリを終えると、macOSがウィンドウをすべて閉じるためである。
    func testStickiesThatAreClosedAsTheAppEndsComeBack() async throws {
        let first = desk.add()
        let second = desk.add()

        desk.end()
        first.window.close()
        second.window.close()

        XCTAssertEqual(nextRun().stickies.map(\.id), [first.id, second.id])
    }

    // 付箋紙のツールは、タイトルバーにある。ウィンドウにフォーカスがないときは隠れ、フォーカスが戻ると出る。
    func testTheToolsAreInTheTitleBarAndAreHiddenWithoutTheFocus() async throws {
        let sticky = desk.add()
        let tools = try XCTUnwrap(sticky.window.titlebarAccessoryViewControllers.first)

        desk.windowDidBecomeKey(Notification(name: NSWindow.didBecomeKeyNotification, object: sticky.window))
        XCTAssertFalse(tools.view.isHidden)

        desk.windowDidResignKey(Notification(name: NSWindow.didResignKeyNotification, object: sticky.window))
        XCTAssertTrue(tools.view.isHidden)
    }

    // タイトルバーの検索のツールを使うと、付箋紙に検索欄が出る。
    func testTheSearchToolOfTheTitleBarOpensTheSearchBox() async throws {
        let sticky = desk.add(url: "/pages/with-tools")
        try await waitUntil("the page is shown") { sticky.window.title == "with-tools" }
        // The screen follows the shell a moment later, and closes the search box when a page comes
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertFalse(hasSearchBox(sticky))

        sticky.look.searching = true

        try await waitUntil("the search box is there") { self.hasSearchBox(sticky) }
    }

    private func hasSearchBox(_ sticky: Sticky) -> Bool {
        func look(_ view: NSView) -> Bool {
            if let field = view as? NSTextField, field.accessibilityIdentifier() == "search" { return true }
            return view.subviews.contains(where: look)
        }
        return sticky.window.contentView.map(look) ?? false
    }

    /// A double click as AppKit gives it to the window, `fromTop` points under its top edge.
    private func doubleClick(_ sticky: Sticky, fromTop: CGFloat) throws {
        let place = NSPoint(x: sticky.window.frame.width / 2, y: sticky.window.frame.height - fromTop)
        let click = try XCTUnwrap(
            NSEvent.mouseEvent(
                with: .leftMouseDown, location: place, modifierFlags: [], timestamp: 0,
                windowNumber: sticky.window.windowNumber, context: nil, eventNumber: 0, clickCount: 2, pressure: 1))
        sticky.window.sendEvent(click)
    }

    // タイトルバーをダブルクリックすると、付箋紙はタイトルバーだけになる。上の端と幅は変わらない。もう一度ダブルクリックすると、元の大きさに戻る。
    func testADoubleClickOnTheTitleBarLeavesOnlyTheTitleBarAndAnotherBringsTheStickyBack() async throws {
        let sticky = desk.add()
        let frame = NSRect(x: 120, y: 130, width: 500, height: 400)
        sticky.window.setFrame(frame, display: true)

        try doubleClick(sticky, fromTop: 10)

        XCTAssertTrue(sticky.rolledUp)
        XCTAssertEqual(sticky.window.contentLayoutRect.height, 0)
        XCTAssertEqual(sticky.window.frame.maxY, frame.maxY)
        XCTAssertEqual(sticky.window.frame.width, frame.width)

        try doubleClick(sticky, fromTop: 10)

        XCTAssertFalse(sticky.rolledUp)
        XCTAssertEqual(sticky.window.frame, frame)
    }

    // 紙の上のダブルクリックでは、付箋紙の大きさは変わらない。
    func testADoubleClickOnThePaperDoesNotChangeTheSticky() async throws {
        let sticky = desk.add()
        let frame = sticky.window.frame

        try doubleClick(sticky, fromTop: 200)

        XCTAssertFalse(sticky.rolledUp)
        XCTAssertEqual(sticky.window.frame, frame)
    }

    // タイトルバーだけにした付箋紙は、次の起動でもタイトルバーだけで戻る。ダブルクリックすると、元の大きさに戻る。
    func testAStickyThatIsRolledUpComesBackRolledUpAndUnrollsToItsSize() async throws {
        let sticky = desk.add()
        let frame = NSRect(x: 120, y: 130, width: 500, height: 400)
        sticky.window.setFrame(frame, display: true)
        try doubleClick(sticky, fromTop: 10)

        let next = nextRun().stickies[0]

        XCTAssertTrue(next.rolledUp)
        XCTAssertEqual(next.window.frame, sticky.window.frame)
        try doubleClick(next, fromTop: 10)
        XCTAssertEqual(next.window.frame, frame)
    }

    // 新しい付箋紙は黄色である。色を変えると、ウィンドウの色が変わり、次の起動でもその色で戻る。
    func testTheColorOfAStickyIsChangedAndComesBack() async throws {
        let sticky = desk.add()
        XCTAssertEqual(sticky.look.color, "#FFFDD0")

        sticky.look.color = "#D0E8FF"

        try await waitUntil("the window is blue") {
            sticky.window.backgroundColor == StickyLook.color(of: "#D0E8FF")
        }
        let next = nextRun()
        XCTAssertEqual(next.stickies[0].look.color, "#D0E8FF")
        XCTAssertEqual(next.stickies[0].window.backgroundColor, StickyLook.color(of: "#D0E8FF"))
    }

    // 色は、#RRGGBBの形で書く。読めない色は、黄色になる。
    func testAColorIsWrittenAsHex() {
        XCTAssertEqual(StickyLook.color(of: "#FF8000"), NSColor(srgbRed: 1, green: 128 / 255, blue: 0, alpha: 1))
        XCTAssertEqual(StickyLook.color(of: "blue"), StickyLook.color(of: "#FFFDD0"))
    }

    // 付箋紙を常に手前に表示すると、ウィンドウがほかのウィンドウの上にとどまる。次の起動でも同じである。やめると、普通のウィンドウに戻る。
    func testAStickyStaysOnTopOfOtherWindowsAndComesBackSo() async throws {
        let sticky = desk.add()
        XCTAssertEqual(sticky.window.level, .normal)

        sticky.look.alwaysOnTop = true

        XCTAssertEqual(sticky.window.level, .floating)
        let next = nextRun()
        XCTAssertTrue(next.stickies[0].look.alwaysOnTop)
        XCTAssertEqual(next.stickies[0].window.level, .floating)
        sticky.look.alwaysOnTop = false
        XCTAssertEqual(sticky.window.level, .normal)
    }

    // ズームは、1回で1割ずつ変わる。3割より小さくも、3倍より大きくもならない。等倍に戻せる。
    func testTheZoomChangesByATenthAndStaysBetweenThreeTenthsAndThree() {
        let look = StickyLook()
        XCTAssertEqual(look.zoom, 1)

        look.zoomIn()
        look.zoomIn()
        XCTAssertEqual(look.zoom, 1.2)
        look.zoomOut()
        XCTAssertEqual(look.zoom, 1.1)
        look.resetZoom()
        XCTAssertEqual(look.zoom, 1)
        for _ in 0..<30 { look.zoomIn() }
        XCTAssertEqual(look.zoom, 3)
        for _ in 0..<40 { look.zoomOut() }
        XCTAssertEqual(look.zoom, 0.3)
    }

    // 付箋紙のズームは、次の起動で戻る。
    func testTheZoomOfAStickyComesBack() async throws {
        let sticky = desk.add()

        sticky.look.zoomIn()

        XCTAssertEqual(nextRun().stickies[0].look.zoom, 1.1)
    }
}
