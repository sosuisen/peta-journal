import PetaBorder
import PetaTypes
import XCTest

/// The drag that starts on the text of an item, without a window. The places are in the list,
/// from its top down.
@MainActor
final class ItemDragTests: XCTestCase {
    private var sent: [Event] = []
    private var drag: ItemDrag!

    override func setUp() {
        sent = []
        drag = ItemDrag { [unowned self] in sent.append($0) }
    }

    // 押した項目のテキストの中と、その上下10までで動くあいだは、何も送らない。文字を選ぶドラッグである。
    func testMovingWithin10OfThePressedTextSendsNothing() {
        for y in [90.0, 115, 140] {
            drag.pressed("a", top: 100, bottom: 130)

            drag.moved(y: y, itemUnder: "b")

            XCTAssertEqual(sent, [], "at \(y)")
            XCTAssertFalse(drag.selects)
        }
    }

    // テキストの上か下へ10より大きく出ると、編集を終えてから、押した項目の選択を始める。編集を終えるのは、打った文字を保存するためである。
    func testLeavingTheTextByMoreThan10EndsTheEditAndSelectsThePressedItem() {
        for y in [89.0, 141] {
            sent = []
            drag.pressed("a", top: 100, bottom: 130)

            drag.moved(y: y, itemUnder: "a")

            XCTAssertEqual(sent, [.editFinished(itemId: "a"), .dragBeyondBounds(itemId: "a")], "at \(y)")
            XCTAssertTrue(drag.selects)
            drag.released()
        }
    }

    // 選択中に別の項目へ入ると、選択がそこまで伸びる。同じ項目へは2度送らない。
    func testEnteringAnotherItemDuringTheSelectionExtendsItThereOnce() {
        drag.pressed("a", top: 100, bottom: 130)
        drag.moved(y: 141, itemUnder: "a")
        sent = []

        drag.moved(y: 150, itemUnder: "b")
        drag.moved(y: 155, itemUnder: "b")
        drag.moved(y: 160, itemUnder: nil)
        drag.moved(y: 180, itemUnder: "c")

        XCTAssertEqual(sent, [.itemMouseEnter(itemId: "b"), .itemMouseEnter(itemId: "c")])
    }

    // ボタンを離すと、ドラッグが終わる。そのあとの動きでは、何も送らない。
    func testReleasingTheButtonEndsTheDragAndLaterMovesSendNothing() {
        drag.pressed("a", top: 100, bottom: 130)
        drag.moved(y: 141, itemUnder: "b")
        sent = []

        drag.released()
        drag.moved(y: 180, itemUnder: "c")

        XCTAssertEqual(sent, [.selectionMouseUp])
        XCTAssertFalse(drag.selects)
    }
}
