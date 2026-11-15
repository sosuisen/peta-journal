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
}
