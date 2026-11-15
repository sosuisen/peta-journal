import PetaBorder
import XCTest

final class EdgeScrollTests: XCTestCase {
    // 付箋の上端か下端から40の帯では、深いほど速くスクロールする。端と、その外では1回に24である。帯の外ではスクロールしない。
    func testTheListScrollsFasterTheDeeperTheMouseIsInTheBandAtAnEdge() {
        let steps: [(y: Double, step: Double)] = [
            (150, 0), (140, 0), (120, -12), (100, -24), (80, -24),
            (460, 0), (480, 12), (500, 24), (520, 24),
        ]
        for (y, step) in steps {
            XCTAssertEqual(EdgeScroll.step(y: y, top: 100, bottom: 500), step, "at \(y)")
        }
    }
}
