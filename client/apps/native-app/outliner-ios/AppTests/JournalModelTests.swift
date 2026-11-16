import PetaBorder
import XCTest

@testable import PetaJournal

/// When the app syncs, and what the sign-in and the sign-out do to it. The model has a real
/// core under it, on the folders of a test. The cloud is the account of a test.
@MainActor
final class JournalModelTests: XCTestCase {
    /// Longer than a test runs: no edit syncs.
    private static let never: TimeInterval = 600
    private static let wait: TimeInterval = 10

    private var root: URL!
    private var opened: JournalModel?

    override func setUp() async throws {
        root = FileManager.default.temporaryDirectory.appendingPathComponent("model-\(UUID().uuidString)")
    }

    override func tearDown() async throws {
        opened?.shell.close()
        opened = nil
        try? FileManager.default.removeItem(at: root)
    }

    /// Opens the app on the folders of the test. An edit syncs after `afterEdit`.
    private func open(_ account: FakeAccount, afterEdit: TimeInterval = JournalModelTests.never) throws -> JournalModel {
        let model = try JournalModel(root: root, account: account, afterEdit: afterEdit)
        opened = model
        return model
    }

    /// Lets the main queue run until `holds` is true. The sync tells the model on it.
    private func waitUntil(_ what: String, _ holds: () -> Bool) {
        let end = Date().addingTimeInterval(JournalModelTests.wait)
        while !holds() {
            XCTAssertLessThan(Date(), end, "waited in vain: \(what)")
            if Date() >= end { return }
            RunLoop.main.run(until: Date().addingTimeInterval(0.02))
        }
    }

    private func exists(_ path: String) -> Bool {
        FileManager.default.fileExists(atPath: root.appendingPathComponent(path).path)
    }

    private var isJournal: (JournalModel) -> Bool = { model in
        if case .journal = model.shell.view.screen { return true }
        return false
    }

    // クラウドの用意がないアプリは、何も同期しない。今日のジャーナルが開く。
    func testAnAppWithoutACloudSyncsNothingAndOpensAtToday() throws {
        let account = FakeAccount(isSetUp: false)

        let model = try open(account)
        model.sync(inFront: true)
        model.signOut()

        XCTAssertNil(model.cloud)
        XCTAssertEqual(account.syncs, 0)
        XCTAssertTrue(isJournal(model))
    }

    // サインインの前は、今日のジャーナルを作らない。最初のダウンロードが、その日のジャーナルを持ってくるかもしれない。
    func testANewAppMakesNoJournalBeforeTheSignIn() throws {
        let model = try open(FakeAccount(signedIn: false))

        XCTAssertTrue(model.shell.unopened)
        XCTAssertEqual(model.cloud?.state, .signedOut)
        let journals = try FileManager.default.contentsOfDirectory(atPath: root.appendingPathComponent("markdown/journals").path)
        XCTAssertEqual(journals, [])
    }

    // サインインしているアプリは、今日のジャーナルを開く。
    func testAnAppThatIsSignedInOpensAtToday() throws {
        let model = try open(FakeAccount())

        XCTAssertFalse(model.shell.unopened)
        XCTAssertTrue(isJournal(model))
    }

    // サインインから戻ったアプリは、ユーザーが待つ同期を始める。
    func testTheAppThatComesBackFromTheSignInSyncsInFront() throws {
        let account = FakeAccount(signedIn: false)
        account.fresh = true
        let model = try open(account)

        model.cameBack(URL(string: "db-app-key://1/connect")!)

        waitUntil("the sync starts") { model.cloud?.inFront == true }
        XCTAssertTrue(account.isSignedIn)
        waitUntil("the page is downloaded") { exists("markdown/docs/trip.md") && model.cloud?.state == .idle }
    }

    // サインインの前の同期は、何もしない。
    func testASyncBeforeTheSignInDoesNothing() throws {
        let account = FakeAccount(signedIn: false)
        let model = try open(account)

        model.sync(inFront: true)

        XCTAssertEqual(account.syncs, 0)
        XCTAssertEqual(model.cloud?.state, .signedOut)
    }

    // 始めた同期は、コアの知らせを待たずに、画面で動いていると分かる。その間、画面はサインアウトを出さない。コアは、同期の途中のサインアウトを受け付けない。
    func testASyncThatStartedRunsOnTheScreenBeforeTheCoreTellsOfIt() throws {
        let model = try open(FakeAccount())

        model.sync(inFront: true)

        XCTAssertEqual(model.cloud?.state.isRunning, true)
        XCTAssertEqual(model.cloud?.inFront, true)
        waitUntil("the sync rests") { model.cloud?.state == .idle }
        XCTAssertEqual(model.cloud?.inFront, false)
    }

    // 続けて起きた変更は、最後の変更から少し待って、1回だけ同期する。キーを打つたびには同期しない。
    func testChangesThatFollowEachOtherSyncOnceAMomentAfterTheLast() throws {
        let account = FakeAccount()
        let model = try open(account, afterEdit: 0.3)

        model.shell.today()
        model.shell.today()

        XCTAssertEqual(account.syncs, 0)
        waitUntil("the app syncs") { account.syncs > 0 }
        RunLoop.main.run(until: Date().addingTimeInterval(0.9))
        XCTAssertEqual(account.syncs, 1)
    }

    // サインアウトは、ダウンロードがどこまで進んだかを忘れる。次のサインインは、別のアカウントかもしれない。そのあとは同期しない。
    func testTheSignOutForgetsWhereTheDownloadEndedAndTheAppSyncsNoMore() throws {
        let account = FakeAccount()
        let model = try open(account)
        model.sync()
        waitUntil("the download ends") { exists("dropbox-cursor") && model.cloud?.state == .idle }

        model.signOut()
        model.sync()

        XCTAssertFalse(exists("dropbox-cursor"))
        XCTAssertFalse(account.isSignedIn)
        XCTAssertEqual(model.cloud?.state, .signedOut)
        XCTAssertEqual(account.syncs, 1)
        XCTAssertTrue(exists("markdown/docs/trip.md"))
    }
}
