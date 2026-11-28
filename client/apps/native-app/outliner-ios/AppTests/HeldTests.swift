import XCTest

@testable import PetaJournal

/// What a touch that is held on an arrow of a pad does, and when it stops.
@MainActor
final class HeldTests: XCTestCase {
    /// Shorter than those of the app, so that the tests are fast.
    private static let hold = Duration.milliseconds(300)
    private static let repeating = Duration.milliseconds(30)

    /// How often the arrow was done.
    private var done = 0

    private func touch() -> Task<Void, Never> {
        Pad.held(hold: HeldTests.hold, repeating: HeldTests.repeating) { [self] in done += 1 }
    }

    /// Lets the main queue and the run loop go on for a while.
    private func pass(_ time: Duration) async {
        try? await Task.sleep(for: time)
    }

    /// Lets the main queue go on until the arrow was done `times` times. Not for a fixed time:
    /// a busy machine does the arrows later.
    private func wait(for times: Int) async {
        let end = ContinuousClock.now + .seconds(10)
        while done < times, ContinuousClock.now < end { await pass(.milliseconds(5)) }
    }

    // 短いタッチは、矢印を1回だけ行う。
    func testAShortTouchDoesItsArrowOnce() async {
        let held = touch()
        await wait(for: 1)
        held.cancel()

        await pass(.milliseconds(500))
        XCTAssertEqual(done, 1)
    }

    // 押し続けたタッチは、矢印を繰り返し、指が上がると止まる。
    func testAHeldTouchRepeatsItsArrowUntilItEnds() async {
        let held = touch()
        await wait(for: 4)
        held.cancel()

        let whileHeld = done
        XCTAssertGreaterThan(whileHeld, 3)
        await pass(.milliseconds(200))
        XCTAssertEqual(done, whileHeld)
    }

    // メインスレッドが忙しい間に指が上がった短いタッチは、矢印を1回だけ行う。
    func testAShortTouchThatEndsWhileTheMainThreadIsBusyDoesItsArrowOnce() async {
        let held = touch()
        await wait(for: 1)

        // The main thread is busy until after the hold, as while the screen draws the next
        // item. The finger went up meanwhile: the pad hears of it on the main queue, where the
        // end of its wait stands first
        Thread.sleep(forTimeInterval: 0.4)
        DispatchQueue.main.async { held.cancel() }

        await pass(.milliseconds(500))
        XCTAssertEqual(done, 1)
    }

    // 押し続けている間にメインスレッドが忙しくなっても、そのあと矢印は繰り返される。
    func testAHeldTouchGoesOnRepeatingAfterTheMainThreadWasBusy() async {
        let held = touch()
        await wait(for: 1)

        Thread.sleep(forTimeInterval: 0.4)
        await wait(for: 4)
        held.cancel()

        XCTAssertGreaterThan(done, 3)
    }
}
