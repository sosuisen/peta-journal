import PetaBorder
import PetaTypes
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
    private func open(_ account: FakeAccount, afterEdit: TimeInterval = JournalModelTests.never) throws -> JournalModel
    {
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
        let journals = try FileManager.default.contentsOfDirectory(
            atPath: root.appendingPathComponent("markdown/journals").path)
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

    // アプリでページを作ると、少し待ってから同期し、そのページがDropboxに上がる。いつ同期するかは、コアが決める（ADR 055）。
    func testAPageMadeInTheAppIsUploadedAMomentLater() throws {
        let account = FakeAccount()
        let model = try open(account, afterEdit: 0.3)
        model.sync(inFront: true)
        waitUntil("the first sync rests") { model.cloud?.state == .idle }

        model.shell.follow(.page(url: "/pages/brand-new"))

        waitUntil("the page goes up") { account.page.uploaded.contains("/PetaJournal/docs/brand-new.md") }
        XCTAssertEqual(account.syncs, 1)
    }

    // キーボードが語を変換している間は、編集のあとの同期は始まらない。変換が終わってから待ち、同期する（#884）。
    func testNoSyncStartsWhileTheKeyboardComposesAWord() throws {
        let account = FakeAccount()
        let wait: TimeInterval = 0.6
        let model = try open(account, afterEdit: wait)
        model.sync(inFront: true)
        waitUntil("the first sync rests") { model.cloud?.state == .idle }
        model.shell.follow(.page(url: "/pages/brand-new"))

        // Twice the wait, with a word composed all the time
        for _ in 0..<6 {
            model.shell.composing()
            RunLoop.main.run(until: Date().addingTimeInterval(wait / 3))
        }

        XCTAssertFalse(account.page.uploaded.contains("/PetaJournal/docs/brand-new.md"))
        waitUntil("the page goes up") { account.page.uploaded.contains("/PetaJournal/docs/brand-new.md") }
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
