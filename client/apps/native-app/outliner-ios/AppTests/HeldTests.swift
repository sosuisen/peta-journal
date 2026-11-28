import XCTest

@testable import PetaJournal

/// What a touch that is held on an arrow of a pad does, and when it stops.
@MainActor
final class HeldTests: XCTestCase {
    /// Shorter than those of the app, so that the tests are fast.
    private static let hold = Duration.milliseconds(200)
    private static let repeating = Duration.milliseconds(40)

    /// How often the arrow was done.
    private var done = 0

    private func touch() -> Task<Void, Never> {
        Pad.held(hold: HeldTests.hold, repeating: HeldTests.repeating) { [self] in done += 1 }
    }

    /// Lets the main queue and the run loop go on for a while.
    private func pass(_ time: Duration) async {
        try? await Task.sleep(for: time)
    }

    // 短いタッチは、矢印を1回だけ行う。
    func testAShortTouchDoesItsArrowOnce() async {
        let held = touch()
        await pass(.milliseconds(50))
        held.cancel()

        await pass(.milliseconds(400))
        XCTAssertEqual(done, 1)
    }

    // 押し続けたタッチは、矢印を繰り返し、指が上がると止まる。
    func testAHeldTouchRepeatsItsArrowUntilItEnds() async {
        let held = touch()
        await pass(.milliseconds(500))
        held.cancel()

        let whileHeld = done
        XCTAssertGreaterThan(whileHeld, 3)
        await pass(.milliseconds(200))
        XCTAssertEqual(done, whileHeld)
    }
}
