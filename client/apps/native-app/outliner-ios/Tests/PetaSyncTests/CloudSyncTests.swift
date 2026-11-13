import Foundation
import PetaSync
import XCTest

final class CloudSyncTests: XCTestCase {
    private var cloud: FakeCloud!
    private var folders: TestFolders!
    /// What waits for the main thread. The test is the main thread, and runs it in `settle`.
    private let posted = Posted()
    /// How often the core was asked to work, and what holds it back in its first work.
    private var rounds = 0
    private let held = DispatchSemaphore(value: 0)
    private let working = DispatchSemaphore(value: 0)
    private var sync: CloudSync!

    override func setUpWithError() throws {
        cloud = FakeCloud()
        folders = try TestFolders()
        let folder = folders.folder
        let record = folders.record
        let cursor = folders.cursor
        var first = true
        let takeIn = { [unowned self] in
            rounds += 1
            working.signal()
            if first { held.wait() }
            first = false
        }
        sync = CloudSync(
            signedIn: true,
            round: { cloud in
                SyncRound(
                    downloader: Downloader(cloud: cloud, folder: folder, cursorFile: cursor, record: record),
                    takeIn: takeIn, uploader: Uploader(cloud: cloud, folder: folder, record: record))
            },
            onMainThread: posted.put)
        cloud.answers[""] = Changes(entries: [], cursor: "c1", whole: true)
        cloud.answers["c1"] = Changes(entries: [], cursor: "c1", whole: false)
    }

    /// Does the work of the main thread until the sync rests.
    private func settle() {
        while let next = posted.take(seconds: 2) { next() }
        XCTAssertEqual(sync.state, .idle)
    }

    private func waitForTheCoreToWork() {
        XCTAssertEqual(working.wait(timeout: .now() + 2), .success)
    }

    // 同期の途中で頼まれた同期は、捨てられない。今の同期が終わったあとに、1回だけ始まる。
    func testASyncThatIsAskedForWhileOneRunsStartsOnceAfterIt() {
        sync.start(cloud)
        waitForTheCoreToWork()

        sync.start(cloud)
        sync.start(cloud)
        held.signal()
        settle()

        XCTAssertEqual(rounds, 2)
    }

    // アプリが前に出たときの同期は、終わるまで「前の同期」である。画面は、その間ダイアログを出す。
    func testASyncOfTheAppThatCameToTheFrontIsInFrontUntilItRests() {
        sync.start(cloud, inFront: true)
        waitForTheCoreToWork()
        XCTAssertTrue(sync.inFront)

        held.signal()
        settle()

        XCTAssertFalse(sync.inFront)
    }

    // 編集のあとの同期は、前の同期ではない。その途中でアプリが前に出ると、前の同期になる。
    func testAQuietSyncComesToTheFrontWhenTheAppDoes() {
        sync.start(cloud)
        waitForTheCoreToWork()
        XCTAssertFalse(sync.inFront)

        sync.start(cloud, inFront: true)

        XCTAssertTrue(sync.inFront)
        held.signal()
        settle()
        XCTAssertFalse(sync.inFront)
    }

    // 途中で頼まれなければ、同期は1回で終わる。
    func testASyncRunsOnceWhenNoOtherIsAskedFor() {
        sync.start(cloud)
        held.signal()
        settle()

        XCTAssertEqual(rounds, 1)
    }

    // 失敗した同期は、その理由を持つ。前の同期ではなくなる。
    func testASyncThatFailedTellsWhyAndIsNoLongerInFront() {
        cloud.answers = [:]
        cloud.failing = true

        sync.start(cloud, inFront: true)
        while let next = posted.take(seconds: 2) { next() }

        guard case .failed = sync.state else { return XCTFail("the sync did not fail: \(sync.state)") }
        XCTAssertFalse(sync.inFront)
    }

    // サインアウトすると、サインインの前の状態に戻る。
    func testASignOutBringsTheStateBeforeTheSignInBack() {
        sync.signOut()

        XCTAssertEqual(sync.state, .signedOut)
    }
}

/// What other threads gave to the main thread of a test.
private final class Posted: @unchecked Sendable {
    private let lock = NSCondition()
    private var waiting: [() -> Void] = []

    func put(_ work: @escaping () -> Void) {
        lock.lock()
        waiting.append(work)
        lock.signal()
        lock.unlock()
    }

    /// The next work, or `nil` when none came in time.
    func take(seconds: TimeInterval) -> (() -> Void)? {
        lock.lock()
        defer { lock.unlock() }
        let until = Date().addingTimeInterval(seconds)
        while waiting.isEmpty {
            if !lock.wait(until: until) { return nil }
        }
        return waiting.removeFirst()
    }
}
