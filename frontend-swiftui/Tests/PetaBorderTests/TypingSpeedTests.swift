import AppKit
import Foundation
import PetaBorder
import PetaScreens
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
        server = try JournalServer(port: 18095)
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
    private func longPage() async throws -> String {
        let maker = Shell(server: server.address, url: "/pages/long")
        try await waitUntil("the page is made") { !maker.view.outline.rows.isEmpty }
        guard case .page(let page?) = maker.view.screen else { throw CancellationError() }
        for n in 1..<Self.items {
            try await server.post(
                "/api/items",
                ["pageId": page.page.id, "position": "last", "level": n % 3, "body": "Item \(n) with some text to draw"])
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

        // The border alone: the event into the core, and the view model out of it
        var events: [Double] = []
        var views: [Double] = []
        let border = Border(server: server.address) { _ in }
        try border.open(url)
        var seen = try border.view()
        while seen.outline.rows.count < Self.items {
            try await Task.sleep(nanoseconds: 10_000_000)
            seen = try border.view()
        }
        let first = seen.outline.rows[0].item.id
        try border.send(.editRequested(itemId: first, cursor: 0))
        for n in 0..<Self.keys {
            let body = String(repeating: "a", count: n + 1)
            events.append(try timed { try border.send(.bodyChanged(itemId: first, body: body, nowMs: 0)) })
            views.append(try timed { _ = try border.view() })
        }
        report("event into the core", events)
        report("view model out of the core", views)

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
}
