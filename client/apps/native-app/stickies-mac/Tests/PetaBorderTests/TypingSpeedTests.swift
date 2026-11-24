import AppKit
import Foundation
import PetaBorder
import PetaScreens
import PetaShared
import PetaTypes
import SwiftUI
import XCTest

/// Measures how long one key takes on a long page (ADR 024). It prints the times and fails on
/// none of them. It needs a real journal server, so it is skipped without `PETA_E2E_SERVER`.
///
/// Run it optimized, as the app is used:
///   swift test -c release -Xswiftc -enable-testing --filter TypingSpeed
@MainActor
final class TypingSpeedTests: XCTestCase {
    private static let items = 1000
    private static let keys = 100
    private var server: JournalServer!

    override func setUp() async throws {
        server = try JournalServer()
    }

    override func tearDown() async throws {
        server?.stop()
    }

    private func waitUntil(_ what: String, _ done: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(30)
        while !done() {
            if Date() > deadline {
                XCTFail("never happened: \(what)")
                throw CancellationError()
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    /// A page with many items, each with a line of text. Gives the URL of the page.
    private func longPage(body: (Int) -> String = { "Item \($0) with some text to draw" }) async throws -> String {
        let maker = Shell(server: server.address, url: "/pages/long")
        try await waitUntil("the page is made") { !maker.view.outline.rows.isEmpty }
        guard case .page(let page?) = maker.view.screen else { throw CancellationError() }
        for n in 1..<Self.items {
            try await server.post(
                "/api/items",
                ["pageId": page.page.id, "position": "last", "level": n % 3, "body": body(n)])
        }
        return "/pages/long"
    }

    private func report(_ name: String, _ seconds: [Double]) {
        let ms = seconds.map { $0 * 1000 }.sorted()
        let median = ms[ms.count / 2]
        let worst = ms[ms.count - 1]
        print(String(format: "SPEED %@: median %.2f ms, worst %.2f ms", name, median, worst))
    }

    private func timed(_ work: () throws -> Void) rethrows -> Double {
        let start = CFAbsoluteTimeGetCurrent()
        try work()
        return CFAbsoluteTimeGetCurrent() - start
    }

    // 1000項目のページで、1つのキーにかかる時間を測る。
    func testAKeyOnALongPage() async throws {
        let url = try await longPage()

        // The border alone: the event into the core, and the changes out of it (ADR 076)
        var events: [Double] = []
        var changes: [Double] = []
        let border = Border(server: server.address) { _ in }
        try border.open(url)
        var seen = ViewModel(first: try border.changes())
        while seen.outline.rows.count < Self.items {
            try await Task.sleep(nanoseconds: 10_000_000)
            seen.apply(try border.changes())
        }
        let first = seen.outline.rows[0].item.id
        try border.send(.editRequested(itemId: first, place: .markdown(0)))
        _ = try border.changes()
        for n in 0..<Self.keys {
            let body = String(repeating: "a", count: n + 1)
            events.append(try timed { try border.send(.bodyChanged(itemId: first, body: body, nowMs: 0)) })
            changes.append(try timed { _ = try border.changes() })
        }
        report("event into the core", events)
        report("changes out of the core", changes)

        // The whole of it: the shell takes the key and the window is drawn again
        let shell = Shell(server: server.address, url: url)
        try await waitUntil("the long page is shown") { shell.view.outline.rows.count >= Self.items }
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 800, height: 600), styleMask: [.titled],
            backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        let host = NSHostingView(rootView: OutlineScreen(shell: shell))
        window.contentView = host
        window.orderFrontRegardless()
        try await Task.sleep(nanoseconds: 1_000_000_000)
        let item = shell.view.outline.rows[0].item.id
        shell.startEditing(item)
        var shellTimes: [Double] = []
        var drawTimes: [Double] = []
        for n in 0..<Self.keys {
            let body = String(repeating: "b", count: n + 1)
            shellTimes.append(timed { shell.typed(body, into: item) })
            drawTimes.append(
                timed {
                    host.layoutSubtreeIfNeeded()
                    host.displayIfNeeded()
                    CATransaction.flush()
                })
            // Lets the render request of the core arrive, as between two keys
            try await Task.sleep(nanoseconds: 5_000_000)
        }
        report("shell takes the key", shellTimes)
        report("window is drawn again", drawTimes)
        XCTAssertEqual(shell.view.outline.rows[0].item.body, String(repeating: "b", count: Self.keys))
        window.close()
    }

    private func editors(in view: NSView) -> [NSTextView] {
        var found: [NSTextView] = []
        func look(_ view: NSView) {
            if let editor = view as? NSTextView { found.append(editor) }
            view.subviews.forEach(look)
        }
        look(view)
        return found
    }

    private func scrollView(in view: NSView) -> NSScrollView? {
        if let scroll = view as? NSScrollView, scroll.documentView != nil { return scroll }
        for child in view.subviews {
            if let found = scrollView(in: child) { return found }
        }
        return nil
    }

    // 装飾のある1000項目のページで、スクロールしたときに装飾付きの表示ができるまでの時間を測る（ADR 041）。
    func testScrollingALongPageWithDecoration() async throws {
        let url = try await longPage { "Item \($0) with **bold**, `code`, [[Page \($0 % 7)]] and #tag to draw" }
        let shell = Shell(server: server.address, url: url)
        try await waitUntil("the long page is shown") {
            let rows = shell.view.outline.rows
            return rows.count >= Self.items && rows.dropFirst().allSatisfy { !$0.item.body.isEmpty }
        }

        // The making of the text alone
        let texts = shell.view.outline.rows.dropFirst().prefix(Self.keys).map(\.bodyModel)
        report("text of one item is made", texts.map { rich in timed { _ = ShownText.of(rich) } })

        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 800, height: 600), styleMask: [.titled],
            backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        let host = NSHostingView(rootView: OutlineScreen(shell: shell))
        window.contentView = host
        window.orderFrontRegardless()
        try await Task.sleep(nanoseconds: 1_000_000_000)
        let scroll = try XCTUnwrap(scrollView(in: host))
        func draw() {
            host.layoutSubtreeIfNeeded()
            host.displayIfNeeded()
            CATransaction.flush()
        }
        /// Whether an editor that can be seen still shows Markdown.
        func waits() -> Bool {
            editors(in: host).contains { !$0.visibleRect.isEmpty && $0.string.contains("**") }
        }

        // A screen further at a time: every row that comes into view is new
        var first: [Double] = []
        var settled: [Double] = []
        var made: [Double] = []
        let step = scroll.contentView.bounds.height
        for n in 1...30 {
            let start = CFAbsoluteTimeGetCurrent()
            scroll.contentView.scroll(to: NSPoint(x: 0, y: CGFloat(n) * step))
            scroll.reflectScrolledClipView(scroll.contentView)
            draw()
            // The rows are there, with their Markdown
            first.append(CFAbsoluteTimeGetCurrent() - start)
            var longest = 0.0
            while waits() {
                // The texts are made in one turn of the main queue
                let turn = CFAbsoluteTimeGetCurrent()
                try await Task.sleep(nanoseconds: 1_000_000)
                longest = max(longest, CFAbsoluteTimeGetCurrent() - turn)
                draw()
                if CFAbsoluteTimeGetCurrent() - start > 5 { break }
            }
            made.append(longest)
            settled.append(CFAbsoluteTimeGetCurrent() - start)
        }
        XCTAssertFalse(waits())
        print("SPEED rows in view: \(editors(in: host).filter { !$0.visibleRect.isEmpty }.count)")
        report("a screen further: rows drawn with Markdown", first)
        report("a screen further: longest turn that makes the texts", made)
        report("a screen further: rows drawn with decoration", settled)

        // The cursor goes from one item to another. The waits between the steps are not
        // counted: they let the main queue run, where the rows are told to be drawn again
        var moved: [Double] = []
        var drawn: [Double] = []
        let seen = editors(in: host).filter { !$0.visibleRect.isEmpty }
        window.makeFirstResponder(seen[0])
        for n in 1...20 {
            try await Task.sleep(nanoseconds: 20_000_000)
            draw()
            moved.append(timed { window.makeFirstResponder(seen[n % 2]) })
            var time = timed { draw() }
            for _ in 0..<3 {
                try await Task.sleep(nanoseconds: 2_000_000)
                time += timed { draw() }
            }
            drawn.append(time)
        }
        report("cursor to another item: the shell and the core", moved)
        report("cursor to another item: window is drawn again", drawn)
        window.close()
    }
}
