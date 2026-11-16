import XCTest

@testable import PetaJournal

/// Which arrow of a pad a touch falls on. The pad here is 90 wide and 60 high.
final class PadTests: XCTestCase {
    private func at(_ x: CGFloat, _ y: CGFloat) -> Arrow? {
        Pad.aim(at: CGPoint(x: x, y: y), in: CGSize(width: 90, height: 60))
    }

    // 端の近くへのタッチは、その向きの矢印になる。
    func testATouchNearAnEdgeIsTheArrowOfThatSide() {
        XCTAssertEqual(at(2, 30), .left)
        XCTAssertEqual(at(88, 30), .right)
        XCTAssertEqual(at(45, 2), .up)
        XCTAssertEqual(at(45, 58), .down)
    }

    // 矢印の絵の上でなくても、角から角への線で分けた範囲なら、その向きになる。
    func testThePartsAreCutByTheLinesFromCornerToCorner() {
        // Near the left top corner: a little more to the left than up
        XCTAssertEqual(at(5, 4), .left)
        // A little more up than to the left
        XCTAssertEqual(at(7, 4), .up)
        // Near the right bottom corner, more down than to the right
        XCTAssertEqual(at(84, 58), .down)
    }

    // 真ん中の3分の1の範囲は、どの向きでもない。
    func testTheMiddleThirdIsNoArrow() {
        XCTAssertNil(at(45, 30))
        XCTAssertNil(at(31, 21))
        XCTAssertNil(at(59, 39))
    }

    // 真ん中の範囲のすぐ外は、矢印になる。
    func testJustOutsideTheMiddleIsAnArrow() {
        XCTAssertEqual(at(29, 30), .left)
        XCTAssertEqual(at(45, 41), .down)
    }
}
