import PetaTypes
import XCTest

@testable import PetaBorder

/// The rows that the changes of the core make (ADR 076). That the changes make the screen of the
/// core is tested in Rust, in `client-native-bridge`.
final class AppliedTests: XCTestCase {
    private func row(_ id: String, _ body: String = "") -> Row {
        Row(
            item: Item(
                id: id, pageId: "page", dirPath: "", name: "", itemType: "", createdDate: 0, modifiedDate: 0,
                idx: "", level: 1, body: body, collapsed: false, types: [], properties: [:], transcludeItemId: nil),
            bodyModel: RichText(blocks: []), position: 0, indentPx: 0, isEditing: false, isSelected: false,
            isCollapsed: false, hasChildren: false, headingLevel: 0, isHeading: false, headingLine: false,
            task: nil, hasOwnZoom: false, isTransclusion: false, isBrokenTransclusion: false, showBullet: true,
            effectiveZoom: 1, dateHeader: nil, suggestionsVisible: false)
    }

    private func ids(_ rows: [Row]) -> [String] {
        rows.map(\.item.id)
    }

    // 行の操作は順に当たる。挿入、削除、移動、更新の位置は、その前の操作を当てたあとの行で数える。
    func testTheChangesAreAppliedInOrderAndEachPlaceCountsAfterTheChangesBefore() {
        let rows = [row("a"), row("b"), row("c")]

        let applied = rows.applying([
            .insert(at: 1, item: row("x")),
            .remove(at: 0),
            .move(from: 2, to: 0),
            .update(at: 1, item: row("x", "changed")),
        ])

        XCTAssertEqual(ids(applied), ["c", "x", "b"])
        XCTAssertEqual(applied[1].item.body, "changed")
    }

    // 入れ替えは、前の行をすべて捨てて、届いた行にする。
    func testAResetPutsTheRowsThatCameInPlaceOfAllBefore() {
        let rows = [row("a"), row("b")]

        let applied = rows.applying([.reset(items: [row("y"), row("z")])])

        XCTAssertEqual(ids(applied), ["y", "z"])
    }
}
