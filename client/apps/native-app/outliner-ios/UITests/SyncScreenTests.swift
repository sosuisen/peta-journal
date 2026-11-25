import XCTest

/// What the screen shows of the sync with the cloud: the sign-in, the dialog while the user
/// waits, the callout, and the menu. The app syncs with the cloud of a test (`TestCloud`), in a
/// folder of the test.
final class SyncScreenTests: XCTestCase {
    private var root: URL!
    private var app: XCUIApplication!

    private static let wait: TimeInterval = 15

    /// How long the file of a test cloud takes to move. The dialog of the download shows only
    /// while it moves: with 2 seconds, it was gone now and then before the test looked for it,
    /// as the app takes a while to start in the simulator (#1176). Below `wait`, so that the
    /// test sees it go
    private static let download: TimeInterval = 6

    override func setUpWithError() throws {
        continueAfterFailure = false
        root = FileManager.default.temporaryDirectory.appendingPathComponent("peta-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        app?.terminate()
        try? FileManager.default.removeItem(at: root)
    }

    /// Starts the app with the cloud of the test: `signedOut`, `signedIn` or `offline`. Each
    /// file takes `delay` seconds to move. `opening` is the URL of a page that the app opens.
    private func start(_ cloud: String, delay: TimeInterval = 0, opening url: String? = nil) {
        app = XCUIApplication()
        app.launchEnvironment["PETA_ROOT"] = root.path
        app.launchEnvironment["PETA_CLOUD"] = cloud
        app.launchEnvironment["PETA_CLOUD_DELAY"] = String(delay)
        if let url { app.launchEnvironment["PETA_OPEN"] = url }
        app.launch()
    }

    /// Something on the screen that shows the text, or a part of it.
    private func shown(_ text: String) -> XCUIElement {
        app.descendants(matching: .any).matching(NSPredicate(format: "label CONTAINS %@", text)).firstMatch
    }

    private func waitFor(_ element: XCUIElement, file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertTrue(
            element.waitForExistence(timeout: SyncScreenTests.wait), "\(element) is not shown", file: file, line: line)
    }

    private func waitUntilGone(_ element: XCUIElement, file: StaticString = #filePath, line: UInt = #line) {
        let gone = expectation(for: NSPredicate(format: "exists == false"), evaluatedWith: element)
        XCTAssertEqual(XCTWaiter.wait(for: [gone], timeout: SyncScreenTests.wait), .completed, file: file, line: line)
    }

    private func menu(_ item: String) {
        let menu = app.buttons["Menu"]
        waitFor(menu)
        menu.tap()
        let button = app.buttons[item]
        waitFor(button)
        button.tap()
    }

    // 新しいアプリは、サインインのボタンだけを見せる。サインインのあとは、ダイアログで同期を待たせ、終わればジャーナルを開く。
    func testANewAppShowsOnlyTheSignInAndWaitsForTheFirstDownloadInADialog() {
        start("signedOut", delay: SyncScreenTests.download)
        waitFor(app.buttons["Sign in to Dropbox"])
        XCTAssertFalse(app.textFields["search"].exists)

        app.buttons["Sign in to Dropbox"].tap()

        waitFor(shown("Downloading"))
        waitUntilGone(shown("Downloading"))
        waitFor(app.textFields["search"])
        XCTAssertTrue(app.buttons["Menu"].exists)
    }

    // アプリが前に来ると同期し、ダイアログで待たせる。終われば、ダウンロードしたページが見える。
    func testTheAppSyncsWhenItComesToTheFrontAndShowsTheDownloadedPage() {
        start("signedIn", delay: SyncScreenTests.download, opening: "/pages/trip")

        waitFor(shown("Downloading"))
        waitUntilGone(shown("Downloading"))
        waitFor(shown("京都で会議がありました。"))
    }

    // つながらなければ、吹き出しでそう伝え、やり直しのボタンを出す。やり直しても、つながらなければ同じである。
    func testAFailedSyncIsToldInTheCalloutWithAButtonToTryAgain() {
        start("offline")
        waitFor(shown("Sync failed: no connection"))
        let again = app.buttons["Try again"]
        XCTAssertTrue(again.exists)

        again.tap()

        waitFor(shown("Sync failed: no connection"))
    }

    // 失敗したあとのメニューには、同期、サインインのやり直し、サインアウトがある。
    func testTheMenuAfterAFailureOffersTheSyncASignInAndTheSignOut() {
        start("offline")
        waitFor(shown("Sync failed"))
        let menu = app.buttons["Menu"]
        menu.tap()

        waitFor(app.buttons["Sync"])
        XCTAssertTrue(app.buttons["Sign in to Dropbox"].exists)
        XCTAssertTrue(app.buttons["Sign out"].exists)
    }

    // サインアウトすると、メニューは消え、ページの上にサインインのボタンが出る。ページは読めるままである。
    func testASignOutLeavesThePageAndPutsTheSignInAboveIt() {
        start("signedIn")
        waitFor(app.textFields["search"])

        menu("Sign out")

        waitFor(app.buttons["Sign in to Dropbox"])
        XCTAssertTrue(app.textFields["search"].exists)
        XCTAssertFalse(app.buttons["Menu"].exists)
    }
}
