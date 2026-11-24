import AppKit
import PetaBorder
import PetaScreens
import PetaTypes
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
            for sticky in desk.stickies { sticky.window.close() }
        }
        server?.stop()
        // A test that was skipped for want of a server never got as far as naming the file
        if let file {
            try? FileManager.default.removeItem(at: file.deletingLastPathComponent())
        }
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

    // ページへのリンクをCommandを押してたどると、そのページを開いた付箋紙が1つ増える。たどった付箋紙のページは変わらない。
    func testALinkFollowedWithCommandOpensItsPageInANewSticky() async throws {
        let first = desk.add(url: "/pages/first-sticky")
        try await waitUntil("the page is shown") { first.shell.title == "first-sticky" }

        first.shell.follow(.page(url: "/pages/linked-sticky"), holding: .command)

        XCTAssertEqual(desk.stickies.count, 2)
        let opened = try XCTUnwrap(desk.stickies.last)
        try await waitUntil("the new sticky shows the page") { opened.shell.title == "linked-sticky" }
        XCTAssertEqual(opened.shell.url, "/pages/linked-sticky")
        XCTAssertEqual(first.shell.url, "/pages/first-sticky")
    }

    // 足した付箋紙は、前の付箋紙にぴったり重ならない。
    func testAStickyThatIsAddedDoesNotCoverTheOneBefore() async throws {
        let first = desk.add()
        let second = desk.add()

        XCTAssertNotEqual(first.window.frame.origin, second.window.frame.origin)
        XCTAssertEqual(first.window.frame.size, second.window.frame.size)
    }

    /// How far the top left corner of `new` is to the right of and down from that of `origin`.
    private func step(from origin: Sticky, to new: Sticky) -> CGSize {
        CGSize(
            width: new.window.frame.minX - origin.window.frame.minX,
            height: origin.window.frame.maxY - new.window.frame.maxY)
    }

    /// Two stickies, the second a little down and to the right of the first, as a sticky is
    /// added, and then moved far from there: a new sticky placed by the last one would be far
    /// from the first. `step` is how far the second was from the first.
    private func twoStickies(url: String? = nil) -> (first: Sticky, step: CGSize) {
        let first = desk.add(url: url)
        let second = desk.add()
        let step = step(from: first, to: second)
        second.window.setFrameOrigin(NSPoint(x: first.window.frame.minX + 300, y: first.window.frame.minY - 200))
        return (first, step)
    }

    /// Cmd+N, as AppKit gives it to the window of `sticky`.
    private func pressCommandN(in sticky: Sticky) async throws {
        sticky.window.makeKeyAndOrderFront(nil)
        // SwiftUI takes the keys of its buttons once it has drawn them
        try await Task.sleep(nanoseconds: 300_000_000)
        let event = try XCTUnwrap(
            NSEvent.keyEvent(
                with: .keyDown, location: .zero, modifierFlags: [.command], timestamp: 0,
                windowNumber: sticky.window.windowNumber, context: nil, characters: "n",
                charactersIgnoringModifiers: "n", isARepeat: false, keyCode: 45))
        XCTAssertTrue(sticky.window.performKeyEquivalent(with: event))
    }

    // Cmd+Nで足した付箋紙は、最後に開いた付箋紙ではなく、Cmd+Nを押した付箋紙の左上隅から少し右下に出る。
    func testAStickyAddedWithCommandNComesJustBelowRightOfTheStickyItCameFrom() async throws {
        let (first, usual) = twoStickies()

        try await pressCommandN(in: first)

        XCTAssertEqual(desk.stickies.count, 3)
        XCTAssertEqual(step(from: first, to: try XCTUnwrap(desk.stickies.last)), usual)
    }

    // リンクから開いた付箋紙は、リンクをたどった付箋紙の左上隅から少し右下に出る。
    func testAStickyOpenedByALinkComesJustBelowRightOfTheStickyOfTheLink() async throws {
        let (first, usual) = twoStickies(url: "/pages/placed-from")
        try await waitUntil("the page is shown") { first.shell.title == "placed-from" }

        first.shell.follow(.page(url: "/pages/placed-sticky"), holding: .command)

        XCTAssertEqual(desk.stickies.count, 3)
        XCTAssertEqual(step(from: first, to: try XCTUnwrap(desk.stickies.last)), usual)
    }

    // その位置では付箋紙のスクリーンからはみ出るときは、新しい付箋紙の全体がそのスクリーンに収まる位置まで戻す。
    func testAStickyThatWouldStickOutOfTheScreenIsMovedBackOntoIt() async throws {
        let first = desk.add()
        let screen = try XCTUnwrap(first.window.screen).visibleFrame
        first.window.setFrameOrigin(NSPoint(x: screen.maxX - first.window.frame.width, y: screen.minY))

        try await pressCommandN(in: first)

        let added = try XCTUnwrap(desk.stickies.last).window.frame
        XCTAssertTrue(screen.contains(added), "\(added) is not in \(screen)")
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

    /// A sticky with a page, whose first item has `text` typed into it and still waiting for its
    /// save. Gives the sticky and the id of its page.
    private func stickyWithTyped(_ text: String, on name: String) async throws -> (Sticky, String) {
        let sticky = desk.add()
        sticky.shell.open("/pages/\(name)")
        try await waitUntil("the page is shown") { !sticky.shell.view.outline.rows.isEmpty }
        guard case .page(let page?) = sticky.shell.view.screen else {
            XCTFail("no page on screen")
            throw CancellationError()
        }
        let item = sticky.shell.view.outline.rows[0].item
        sticky.shell.startEditing(item.id)
        sticky.shell.typed(text, into: item.id)
        return (sticky, page.page.id)
    }

    /// True when the server has the text on the page.
    private func saved(_ text: String, on pageId: String) throws -> Bool {
        String(decoding: try server.get("/api/pages/\(pageId)"), as: UTF8.self).contains(text)
    }

    // 付箋紙を閉じると、保存を待っていた本文が保存される（#766）。
    func testClosingAStickySavesTheTextStillWaitingToBeSaved() async throws {
        let (sticky, pageId) = try await stickyWithTyped("typed in a closed sticky", on: "desk-closed")

        sticky.window.close()

        XCTAssertTrue(try saved("typed in a closed sticky", on: pageId))
    }

    // アプリを終えると、どの付箋紙でも、保存を待っていた本文が保存される（#766）。
    func testEndingTheAppSavesTheTextStillWaitingToBeSaved() async throws {
        let (_, pageId) = try await stickyWithTyped("typed before the end", on: "desk-ending")

        desk.end()

        XCTAssertTrue(try saved("typed before the end", on: pageId))
    }

    /// The buttons that close, minimize and zoom the window of the sticky.
    private func windowButtons(_ sticky: Sticky) throws -> [NSButton] {
        try [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton].map {
            try XCTUnwrap(sticky.window.standardWindowButton($0))
        }
    }

    // 付箋紙のツールは、タイトルバーにある。検索のツールは、ウィンドウにフォーカスがないときは隠れ、フォーカスが戻ると出る。新しい付箋紙のボタンがあるので、ツールの置き場所は隠れない。
    func testTheToolsAreInTheTitleBarAndTheSearchToolIsHiddenWithoutTheFocus() async throws {
        let sticky = desk.add()
        let tools = try XCTUnwrap(sticky.window.titlebarAccessoryViewControllers.first)

        desk.windowDidBecomeKey(Notification(name: NSWindow.didBecomeKeyNotification, object: sticky.window))
        XCTAssertTrue(sticky.look.toolsShown)
        XCTAssertFalse(tools.view.isHidden)

        desk.windowDidResignKey(Notification(name: NSWindow.didResignKeyNotification, object: sticky.window))
        XCTAssertFalse(sticky.look.toolsShown)
        XCTAssertFalse(tools.view.isHidden)
    }

    // ウィンドウにフォーカスがないときは、ウィンドウを閉じる・しまう・大きくするボタンは隠れる。フォーカスが戻ると、ボタンが出る。
    func testTheButtonsOfTheWindowAreHiddenWithoutTheFocus() async throws {
        let sticky = desk.add()
        let buttons = try windowButtons(sticky)

        desk.windowDidBecomeKey(Notification(name: NSWindow.didBecomeKeyNotification, object: sticky.window))
        XCTAssertEqual(buttons.map(\.isHidden), [false, false, false])

        desk.windowDidResignKey(Notification(name: NSWindow.didResignKeyNotification, object: sticky.window))
        XCTAssertEqual(buttons.map(\.isHidden), [true, true, true])
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
    /// `count` says which click of a quick row it is: the second makes a double click.
    private func doubleClick(_ sticky: Sticky, fromTop: CGFloat, count: Int = 2) throws {
        try doubleClick(
            sticky, at: NSPoint(x: sticky.window.frame.width / 2, y: sticky.window.frame.height - fromTop),
            count: count)
    }

    /// A double click as AppKit gives it to the window, at `place` in the window.
    private func doubleClick(_ sticky: Sticky, at place: NSPoint, count: Int = 2) throws {
        let click = try XCTUnwrap(
            NSEvent.mouseEvent(
                with: .leftMouseDown, location: place, modifierFlags: [], timestamp: 0,
                windowNumber: sticky.window.windowNumber, context: nil, eventNumber: 0, clickCount: count,
                pressure: 1))
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

    // タイトルバーをすばやくクリックし続けると、2回目でタイトルバーだけになり、4回目で元の大きさに戻る。ウィンドウは大きくならない。
    func testQuickClicksOnTheTitleBarRollTheStickyUpAndDownAndDoNotZoomIt() async throws {
        let sticky = desk.add()
        let frame = NSRect(x: 120, y: 130, width: 500, height: 400)
        sticky.window.setFrame(frame, display: true)

        try doubleClick(sticky, fromTop: 10, count: 2)
        try doubleClick(sticky, fromTop: 10, count: 3)

        XCTAssertTrue(sticky.rolledUp)
        XCTAssertEqual(sticky.window.contentLayoutRect.height, 0)

        try doubleClick(sticky, fromTop: 10, count: 4)

        XCTAssertFalse(sticky.rolledUp)
        XCTAssertEqual(sticky.window.frame, frame)
    }

    /// The text in the title bar that shows the name of the window.
    private func nameInTitleBar(_ sticky: Sticky) -> NSTextField? {
        func look(_ view: NSView) -> NSTextField? {
            if let field = view as? NSTextField, field.stringValue == sticky.window.title { return field }
            return view.subviews.lazy.compactMap(look).first
        }
        return sticky.window.contentView?.superview.flatMap(look)
    }

    // タイトルバーの名前の文字の上をダブルクリックしても、付箋紙はタイトルバーだけになる。ウィンドウは大きくならない。
    func testADoubleClickOnTheNameInTheTitleBarLeavesOnlyTheTitleBar() async throws {
        let sticky = desk.add()
        let frame = NSRect(x: 120, y: 130, width: 500, height: 400)
        sticky.window.setFrame(frame, display: true)
        let name = try XCTUnwrap(nameInTitleBar(sticky))
        let place = name.convert(name.bounds, to: nil)

        try doubleClick(sticky, at: NSPoint(x: place.midX, y: place.midY))

        XCTAssertTrue(sticky.rolledUp)
        XCTAssertEqual(sticky.window.frame.maxY, frame.maxY)
        XCTAssertEqual(sticky.window.frame.width, frame.width)
    }

    // タイトルバーだけにしている間は、ウィンドウを閉じる・しまう・大きくするボタンは隠れる。元に戻すと、ボタンが出る。
    func testTheButtonsOfTheWindowAreHiddenWhileTheStickyIsRolledUp() async throws {
        let sticky = desk.add()
        let buttons = try windowButtons(sticky)
        desk.windowDidBecomeKey(Notification(name: NSWindow.didBecomeKeyNotification, object: sticky.window))
        XCTAssertEqual(buttons.map(\.isHidden), [false, false, false])

        desk.roll(sticky)
        XCTAssertEqual(buttons.map(\.isHidden), [true, true, true])

        desk.roll(sticky)
        XCTAssertEqual(buttons.map(\.isHidden), [false, false, false])
    }

    // タイトルバーだけにしている間は、フォーカスがあっても検索のツールは隠れる。元に戻すと、ツールが出る。
    func testTheSearchToolIsHiddenWhileTheStickyIsRolledUp() async throws {
        let sticky = desk.add()
        let gotFocus = Notification(name: NSWindow.didBecomeKeyNotification, object: sticky.window)
        desk.windowDidBecomeKey(gotFocus)

        desk.roll(sticky)
        XCTAssertFalse(sticky.look.toolsShown)
        desk.windowDidBecomeKey(gotFocus)
        XCTAssertFalse(sticky.look.toolsShown)

        desk.roll(sticky)
        XCTAssertTrue(sticky.look.toolsShown)
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
        XCTAssertEqual(sticky.look.color, "#FFF8D0")

        sticky.look.color = "#D0E8FF"

        try await waitUntil("the window is blue") {
            sticky.window.backgroundColor == StickyLook.color(of: "#D0E8FF")
        }
        let next = nextRun()
        XCTAssertEqual(next.stickies[0].look.color, "#D0E8FF")
        XCTAssertEqual(next.stickies[0].window.backgroundColor, StickyLook.color(of: "#D0E8FF"))
    }

    /// The window shows what is behind it.
    private func seesThrough(_ sticky: Sticky) -> Bool {
        !sticky.window.isOpaque && (sticky.window.backgroundColor.usingColorSpace(.sRGB)?.alphaComponent ?? 1) < 0.05
    }

    // 透明を選ぶと、付箋紙のウィンドウは後ろが透けて見える。次の起動でも透明で戻る。色を戻すと、透けなくなる。
    func testATransparentStickyShowsWhatIsBehindItAndComesBackSo() async throws {
        let sticky = desk.add()
        XCTAssertFalse(seesThrough(sticky))

        sticky.look.color = "#FFFFFF00"

        XCTAssertTrue(seesThrough(sticky))
        let next = nextRun().stickies[0]
        XCTAssertTrue(seesThrough(next))
        next.look.color = "#FFF8D0"
        XCTAssertFalse(seesThrough(next))
    }

    // 付箋紙の色は、Petastiの付箋紙の9色と、そのあとの透明から選ぶ。
    func testTheColorsOfAStickyAreThoseOfPetasti() {
        XCTAssertEqual(
            StickyLook.colors.map(\.name),
            ["Yellow", "Red", "Green", "Blue", "Orange", "Purple", "White", "Gray", "Light Gray", "Transparent"])
        XCTAssertEqual(
            StickyLook.colors.map(\.hex),
            [
                "#FFF8D0", "#FFCAC4", "#EDFFC4", "#D1E1FF", "#FFDD9E", "#F8D0FF", "#FFFFFF", "#E0E0E0", "#F8F8F8",
                "#FFFFFF00",
            ])
    }

    // 色は、#RRGGBBの形で書く。読めない色は、黄色になる。
    func testAColorIsWrittenAsHex() {
        XCTAssertEqual(StickyLook.color(of: "#FF8000"), NSColor(srgbRed: 1, green: 128 / 255, blue: 0, alpha: 1))
        XCTAssertEqual(StickyLook.color(of: "blue"), StickyLook.color(of: "#FFF8D0"))
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

    // タイトルバーのメニューには、常に手前に表示する項目と、付箋紙の色が並ぶ。今の色には印が付く。
    func testTheMenuOfTheTitleBarHasThePinAndTheColors() async throws {
        let sticky = desk.add()

        let menu = desk.menu(of: sticky)

        XCTAssertEqual(
            menu.items.map { $0.isSeparatorItem ? "-" : $0.title },
            ["Keep on Top", "-"] + StickyLook.colors.map(\.name))
        XCTAssertEqual(menu.items.filter { $0.state == .on }.map(\.title), ["Yellow"])
    }

    /// The editors of the items of the sticky, from the top of the window down.
    private func editors(of sticky: Sticky) -> [NSTextView] {
        var found: [NSTextView] = []
        func look(_ view: NSView) {
            if let editor = view as? NSTextView { found.append(editor) }
            view.subviews.forEach(look)
        }
        if let content = sticky.window.contentView { look(content) }
        return found.sorted {
            $0.convert($0.bounds, to: nil).maxY > $1.convert($1.bounds, to: nil).maxY
        }
    }

    /// A right click as AppKit gives it to the window, on the first letter of the editor. The
    /// menu that opens is closed at once, and its entries are the answer. With `control` it is
    /// a click of the left button while the Control key is down.
    private func rightClick(on editor: NSTextView, of sticky: Sticky, control: Bool = false) throws -> [String] {
        let layout = try XCTUnwrap(editor.layoutManager)
        let container = try XCTUnwrap(editor.textContainer)
        let box = layout.boundingRect(forGlyphRange: NSRange(location: 0, length: 1), in: container)
        let origin = editor.textContainerOrigin
        let place = editor.convert(NSPoint(x: origin.x + box.midX, y: origin.y + box.midY), to: nil)
        var shown: [String] = []
        // A menu holds the main thread while it is open: the test would never go on
        let opened = NotificationCenter.default.addObserver(
            forName: NSMenu.didBeginTrackingNotification, object: nil, queue: nil
        ) { note in
            guard let menu = note.object as? NSMenu else { return }
            shown = menu.items.map(\.title)
            // Not at once: a menu that has only begun to open does not hear it
            RunLoop.main.perform(inModes: [.eventTracking, .default]) { menu.cancelTrackingWithoutAnimation() }
        }
        defer { NotificationCenter.default.removeObserver(opened) }
        func mouse(_ type: NSEvent.EventType) throws -> NSEvent {
            try XCTUnwrap(
                NSEvent.mouseEvent(
                    with: type, location: place, modifierFlags: control ? [.control] : [], timestamp: 0,
                    windowNumber: sticky.window.windowNumber, context: nil, eventNumber: 0, clickCount: 1,
                    pressure: 1))
        }
        // A text view waits for the button to come up before it goes on
        if control { NSApp.postEvent(try mouse(.leftMouseUp), atStart: false) }
        sticky.window.sendEvent(try mouse(control ? .leftMouseDown : .rightMouseDown))
        return shown
    }

    /// A sticky with two items: the first is "bold text" with its decoration, and the cursor
    /// is in the second, which is empty.
    private func stickyWithABoldItemAndTheCursorInTheNext() async throws -> Sticky {
        let sticky = desk.add()
        sticky.shell.open("/pages/right-click")
        try await waitUntil("the editor of the item is there") { self.editors(of: sticky).count == 1 }
        let first = editors(of: sticky)[0]
        sticky.window.makeFirstResponder(first)
        first.insertText("**bold** text", replacementRange: first.selectedRange())
        first.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        try await waitUntil("the cursor is in the new item") {
            self.editors(of: sticky).count == 2 && sticky.window.firstResponder === self.editors(of: sticky)[1]
        }
        try await waitUntil("the first item is decorated") { self.editors(of: sticky)[0].string == "bold text" }
        return sticky
    }

    // 編集中でない項目を右クリックすると、付箋紙のメニューが出る。項目は編集に入らず、カーソルは元の項目に残る。
    func testARightClickOnAnItemThatIsNotEditedShowsTheMenuOfTheStickyAndDoesNotEditTheItem() async throws {
        let sticky = try await stickyWithABoldItemAndTheCursorInTheNext()

        let shown = try rightClick(on: editors(of: sticky)[0], of: sticky)

        XCTAssertEqual(shown.first, "Keep on Top")
        XCTAssertEqual(editors(of: sticky)[0].string, "bold text")
        XCTAssertTrue(sticky.window.firstResponder === editors(of: sticky)[1])
    }

    // 編集中でない項目を、Controlキーを押しながらクリックしても、付箋紙のメニューが出る。項目は編集に入らず、カーソルは元の項目に残る。
    func testAControlClickOnAnItemThatIsNotEditedShowsTheMenuOfTheStickyAndDoesNotEditTheItem() async throws {
        let sticky = try await stickyWithABoldItemAndTheCursorInTheNext()

        let shown = try rightClick(on: editors(of: sticky)[0], of: sticky, control: true)

        XCTAssertEqual(shown.first, "Keep on Top")
        XCTAssertEqual(editors(of: sticky)[0].string, "bold text")
        XCTAssertTrue(sticky.window.firstResponder === editors(of: sticky)[1])
    }

    // メニューで常に手前に表示を選ぶと、ウィンドウがほかのウィンドウの上にとどまり、項目に印が付く。もう一度選ぶと、普通のウィンドウに戻る。
    func testThePinOfTheMenuKeepsTheStickyOnTopAndAnotherChoiceEndsIt() async throws {
        let sticky = desk.add()

        desk.menu(of: sticky).performActionForItem(at: 0)

        XCTAssertEqual(sticky.window.level, .floating)
        let menu = desk.menu(of: sticky)
        XCTAssertEqual(menu.items[0].state, .on)

        menu.performActionForItem(at: 0)

        XCTAssertEqual(sticky.window.level, .normal)
    }

    // メニューで色を選ぶと、付箋紙がその色になり、その色に印が付く。
    func testAColorOfTheMenuIsTheColorOfTheSticky() async throws {
        let sticky = desk.add()
        let menu = desk.menu(of: sticky)
        let blue = try XCTUnwrap(menu.items.firstIndex { $0.title == "Blue" })

        menu.performActionForItem(at: blue)

        XCTAssertEqual(sticky.look.color, "#D1E1FF")
        XCTAssertEqual(desk.menu(of: sticky).items.filter { $0.state == .on }.map(\.title), ["Blue"])
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
