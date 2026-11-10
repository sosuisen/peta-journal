import Foundation
import XCTest

@testable import PetaBorder

/// What is kept of the stickies between two runs of the app.
final class StickyStoreTests: XCTestCase {
    private var file: URL!

    override func setUp() {
        file = FileManager.default.temporaryDirectory
            .appendingPathComponent("peta-stickies-\(UUID().uuidString)")
            .appendingPathComponent("stickies.json")
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: file.deletingLastPathComponent())
    }

    private func sticky(_ id: String, url: String? = nil) -> StickySettings {
        StickySettings(id: id, url: url, x: 10, y: 20, width: 300, height: 400)
    }

    // ファイルがまだないときは、付箋紙は1つもない。
    func testThereAreNoStickiesBeforeAnythingIsKept() {
        XCTAssertEqual(StickyStore(file: file).stickies, [])
    }

    // 足した付箋紙は、読み直しても残る。順番も同じである。
    func testStickiesThatWerePutAreThereAfterReading() {
        let store = StickyStore(file: file)
        store.put(sticky("a", url: "/pages/notes"))
        store.put(sticky("b"))

        XCTAssertEqual(StickyStore(file: file).stickies, [sticky("a", url: "/pages/notes"), sticky("b")])
    }

    // 同じ付箋紙をもう一度入れると、増えずに、中身が変わる。
    func testAStickyThatIsPutAgainIsChangedAndNotAdded() {
        let store = StickyStore(file: file)
        store.put(sticky("a"))
        store.put(sticky("b"))
        var moved = sticky("a", url: "/today")
        moved.x = 99

        store.put(moved)

        XCTAssertEqual(StickyStore(file: file).stickies, [moved, sticky("b")])
    }

    // 消した付箋紙は、読み直しても出てこない。ほかの付箋紙は残る。
    func testAStickyThatIsRemovedIsNotThereAfterReading() {
        let store = StickyStore(file: file)
        store.put(sticky("a"))
        store.put(sticky("b"))

        store.remove("a")

        XCTAssertEqual(StickyStore(file: file).stickies, [sticky("b")])
    }

    // 読めないファイルは、付箋紙がないものとして扱う。次に入れた付箋紙は、保存される。
    func testAFileThatCannotBeReadCountsAsNoStickies() throws {
        try FileManager.default.createDirectory(
            at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        try Data("not json".utf8).write(to: file)
        let store = StickyStore(file: file)
        XCTAssertEqual(store.stickies, [])

        store.put(sticky("a"))

        XCTAssertEqual(StickyStore(file: file).stickies, [sticky("a")])
    }

    // 付箋紙の色は、保存される。色のない古いファイルの付箋紙は、黄色になる。
    func testTheColorIsKeptAndAStickyWithoutOneIsYellow() throws {
        let store = StickyStore(file: file)
        var pink = sticky("a")
        pink.color = "#FFD1DC"
        store.put(pink)
        XCTAssertEqual(StickyStore(file: file).stickies[0].color, "#FFD1DC")

        let old = #"[{"id": "b", "x": 1, "y": 2, "width": 3, "height": 4}]"#
        try Data(old.utf8).write(to: file)

        XCTAssertEqual(StickyStore(file: file).stickies.map(\.color), ["#FFFDD0"])
    }
}
