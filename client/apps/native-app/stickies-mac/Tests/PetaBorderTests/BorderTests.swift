import Foundation
import PetaTypes
import XCTest

@testable import PetaBorder

/// What the core asked of the shell, kept for a test. The core may ask from any thread.
private final class Asked: @unchecked Sendable {
    private let lock = NSLock()
    private var requests: [Request] = []

    func add(_ new: [Request]) {
        lock.lock()
        requests += new
        lock.unlock()
    }

    var all: [Request] {
        lock.lock()
        defer { lock.unlock() }
        return requests
    }

    var navigations: [String] {
        all.compactMap { request in
            if case .navigate(let op) = request.effect { return op.url }
            return nil
        }
    }

    /// The navigations, once there is one. A request does not always come before the call that
    /// caused it returns: the core starts by asking the server from threads of its own, and
    /// when one of them is at work in the core, it is that thread that hands the request over.
    func navigationsOnceAny() -> [String] {
        _ = firstOnceThere { request in
            if case .navigate = request.effect { return true }
            return false
        }
        return navigations
    }

    /// The first request of a kind, once it is there.
    func firstOnceThere(_ isOfTheKind: (Request) -> Bool) -> Request? {
        let deadline = Date().addingTimeInterval(5)
        while true {
            if let found = all.first(where: isOfTheKind) { return found }
            if Date() > deadline { return nil }
            Thread.sleep(forTimeInterval: 0.005)
        }
    }
}

final class BorderTests: XCTestCase {
    /// No server is here: these tests are about the bytes between Swift and Rust.
    private let nowhere = "http://127.0.0.1:9"

    private func border(_ asked: Asked = Asked()) -> Border {
        Border(server: nowhere) { asked.add($0) }
    }

    // Rustが書いた画面の差分を、Swiftが読める。最初の差分には、Swiftが送ったイベントの中身が入っている。
    func testTheChangesOfTheScreenWrittenByRustAreReadInSwift() throws {
        let border = border()

        try border.send(.searchQueryChanged("apple 日本語"))

        let search = try border.changes().panels.compactMap { panel in
            if case .search(let search) = panel { return search }
            return nil
        }
        XCTAssertEqual(search.map(\.query), ["apple 日本語"])
    }

    // Rustが書いたシェルへの依頼を、Swiftが読める。
    func testARequestOfRustIsReadInSwift() throws {
        let asked = Asked()
        let border = border(asked)

        try border.send(.navigateHome)

        XCTAssertEqual(asked.navigationsOnceAny(), ["/today"])
    }

    // Swiftが書いた答えを、Rustが読める。新しいページの名前を答えると、そのページへ移る依頼が来る。
    func testAnAnswerOfSwiftIsReadInRust() throws {
        let asked = Asked()
        let border = border(asked)
        try border.send(.createNewPage(searchQuery: ""))
        let prompt = try XCTUnwrap(
            asked.firstOnceThere { request in
                if case .ui(.prompt) = request.effect { return true }
                return false
            })

        try border.answer(prompt.id, with: .text("Notes"))

        XCTAssertEqual(asked.navigationsOnceAny(), ["/pages/Notes"])
    }

    // 項目の行き先を持つイベントを、Rustが読める。行き先は、Rustでは手で書いたシリアライズを持つ型である。
    func testAnEventWithThePlaceOfAnItemIsReadInRust() throws {
        let border = border()
        let item = Item(
            id: "a", pageId: "p", dirPath: "", name: "notes", itemType: "doc", createdDate: 0,
            modifiedDate: 0, idx: "a", level: 1, body: "本文", collapsed: false,
            types: [], properties: ["tags": .list(["x"]), "title": .text("t")],
            transcludeItemId: nil)

        for place in [ItemPosition.first, .last, .after("x")] {
            XCTAssertNoThrow(
                try border.send(.moved(moved: [item], position: place, edited: nil, label: .move)))
        }
    }

    // URLでページを開ける。何も指さないURLは、エラーになる。
    func testAPageIsOpenedByItsURL() throws {
        let border = border()

        XCTAssertNoThrow(try border.open("/pages/notes"))
        XCTAssertThrowsError(try border.open(""))
    }

    // 待っていない依頼への答えは、エラーになる。
    func testAnAnswerNobodyWaitsForIsAnError() {
        let border = border()

        XCTAssertThrowsError(try border.answer(9_999, with: .done))
    }
}
