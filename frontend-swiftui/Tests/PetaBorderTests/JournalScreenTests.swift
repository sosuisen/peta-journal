import AppKit
import PetaBorder
import PetaScreens
import PetaTypes
import SwiftUI
import XCTest

/// The screen with a journal in a real window: the days below one another. These need a real
/// journal server, so they are skipped without `PETA_E2E_SERVER`.
@MainActor
final class JournalScreenTests: XCTestCase {
    private var server: JournalServer!
    private var shell: Shell!
    private var window: NSWindow!
    private var host: NSView!

    /// The journal of today with many items, above the journal of a day long ago.
    override func setUp() async throws {
        server = try JournalServer(port: 18100)
        let old = Shell(server: server.address, url: "/journals/2020-01-02")
        try await waitUntil("the old journal is shown") { !old.view.outline.rows.isEmpty }
        try await add(40, "long ago", to: old)
        let today = Shell(server: server.address, url: "/today")
        try await waitUntil("the journal of today is shown") { !today.view.outline.rows.isEmpty }
        try await add(40, "today", to: today)

        shell = Shell(server: server.address, url: "/today")
        try await waitUntil("both days are shown") { self.shell.view.outline.rows.count >= 82 }
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
            backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        host = NSHostingView(rootView: OutlineScreen(shell: shell))
        window.contentView = host
        window.orderFrontRegardless()
        try await Task.sleep(nanoseconds: 300_000_000)
    }

    override func tearDown() async throws {
        window?.close()
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

    private func add(_ count: Int, _ text: String, to journal: Shell) async throws {
        guard case .journal(let shown) = journal.view.screen, let page = shown.pageId else {
            throw CancellationError()
        }
        for n in 1...count {
            try await server.post(
                "/api/items", ["pageId": page, "position": "last", "level": 1, "body": "\(text) \(n)"])
        }
    }

    private func scrollView(in view: NSView) -> NSScrollView? {
        if let scroll = view as? NSScrollView, scroll.documentView != nil { return scroll }
        for child in view.subviews {
            if let found = scrollView(in: child) { return found }
        }
        return nil
    }

    /// Scrolls the items as far as `part` of the way down: 0 is the top, 1 the end.
    private func scroll(to part: CGFloat) throws {
        let scroll = try XCTUnwrap(scrollView(in: host))
        let whole = (scroll.documentView?.frame.height ?? 0) - scroll.contentView.bounds.height
        scroll.contentView.scroll(to: NSPoint(x: 0, y: whole * part))
        scroll.reflectScrolledClipView(scroll.contentView)
    }

    /// The texts that the window shows to the accessibility of macOS.
    private func texts() -> [String] {
        var found: [String] = []
        func look(_ element: Any, _ depth: Int) {
            guard depth < 40, let element = element as? NSAccessibilityElementProtocol & NSObject else { return }
            if let text = element.value(forKey: "accessibilityLabel") as? String, !text.isEmpty { found.append(text) }
            if let text = element.value(forKey: "accessibilityValue") as? String, !text.isEmpty { found.append(text) }
            for child in (element.value(forKey: "accessibilityChildren") as? [Any]) ?? [] { look(child, depth + 1) }
        }
        look(host as Any, 0)
        return found
    }

    // ジャーナルを前の日までスクロールすると、タイトルはその日の日付になる。今日まで戻ると、今日の日付に「(Today)」が付く。
    func testScrollingToAnEarlierDayShowsItsDateInTheTitle() async throws {
        XCTAssertTrue(shell.title.hasSuffix(" (Today)"))

        try scroll(to: 1)

        try await waitUntil("the title is the date of the earlier day") { self.shell.title == "2020年1月2日 木曜日" }

        try scroll(to: 0)

        try await waitUntil("the title is the date of today again") { self.shell.title.hasSuffix(" (Today)") }
    }

    // 前の日の最初の項目の上に、その日の日付が出る。
    func testTheDateOfAnEarlierDayStandsAboveItsFirstItem() async throws {
        try scroll(to: 1)

        try await waitUntil("the date of the earlier day is on screen") {
            self.texts().contains("2020年1月2日 木曜日")
        }
    }
}
