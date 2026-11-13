import PetaSync
import XCTest

@testable import PetaDropbox

/// The sign-in and the sign-out. The Dropbox SDK is an SDK of the test: the token of the app
/// itself is not touched.
final class DropboxAccountTests: XCTestCase {
    private var sdk: FakeSdk!

    override func setUp() {
        sdk = FakeSdk()
    }

    private func account(key: String = "app-key") -> DropboxAccount {
        DropboxAccount(key: key, folder: "/PetaJournal", sdk: sdk)
    }

    // App keyなしでビルドしたアプリは、同期の用意ができていない。SDKにも何も頼まない。
    func testAnAppWithoutTheAppKeyIsNotSetUpAndAsksNothingOfTheSdk() {
        sdk.signedIn = true
        let account = account(key: "")

        XCTAssertFalse(account.isSetUp)
        XCTAssertFalse(account.isSignedIn)
        XCTAssertNil(account.cloud())
        XCTAssertFalse(account.cameBack(URL(string: "db-app-key://1/connect")!) { _ in })
        account.signOut()
        XCTAssertEqual(sdk.keys, [])
        XCTAssertEqual(sdk.signOuts, 0)
    }

    // App keyのあるアプリは、そのApp keyをSDKに1回だけ伝える。
    func testAnAppWithTheAppKeyTellsTheSdkTheKeyOnce() {
        let account = account()

        XCTAssertTrue(account.isSetUp)
        XCTAssertEqual(sdk.keys, ["app-key"])
    }

    // サインインの前は、サインインしていない。クラウドもない。
    func testBeforeTheSignInTheAccountIsSignedOutAndHasNoCloud() {
        let account = account()

        XCTAssertFalse(account.isSignedIn)
        XCTAssertNil(account.cloud())
    }

    // サインインのあとのクラウドは、設定のフォルダーを読む。
    func testTheCloudOfTheAccountReadsItsFolder() throws {
        sdk.signedIn = true

        let changes = try XCTUnwrap(account().cloud()).changes(since: nil)

        XCTAssertEqual(changes.entries, [.file(CloudFile(path: "docs/a.md", contentHash: "hash", remote: "rev:r1"))])
    }

    // Dropboxから戻ったURLは、SDKが読む。サインインできたかは、メインスレッドで伝わる。
    func testTheUrlThatDropboxCameBackWithTellsOnTheMainThreadWhetherTheUserSignedIn() {
        let account = account()
        sdk.answer = true
        let told = expectation(description: "the app is told")

        let taken = account.cameBack(URL(string: "db-app-key://1/connect")!) { signedIn in
            XCTAssertTrue(Thread.isMainThread)
            XCTAssertTrue(signedIn)
            told.fulfill()
        }

        XCTAssertTrue(taken)
        wait(for: [told], timeout: 5)
    }

    // サインアウトは、SDKに資格情報を捨てさせる。
    func testTheSignOutLetsTheSdkGoOfTheToken() {
        sdk.signedIn = true
        let account = account()

        account.signOut()

        XCTAssertEqual(sdk.signOuts, 1)
        XCTAssertFalse(account.isSignedIn)
        XCTAssertNil(account.cloud())
    }
}

/// An SDK that keeps what it was asked.
private final class FakeSdk: DropboxSdk {
    var keys: [String] = []
    var signedIn = false
    /// What the SDK makes of a URL that Dropbox came back with.
    var answer = false
    var signOuts = 0

    func setUp(key: String) {
        keys.append(key)
    }

    var isSignedIn: Bool { signedIn }

    func endSignIn(_ url: URL, then: @escaping (Bool) -> Void) -> Bool {
        let answer = answer
        DispatchQueue.global().async { then(answer) }
        return true
    }

    func files() -> DropboxFiles? {
        signedIn ? OneFile() : nil
    }

    func signOut() {
        signOuts += 1
        signedIn = false
    }
}

/// A Dropbox with one file in the folder of the app.
private struct OneFile: DropboxFiles {
    func list(folder: String) throws -> Listing {
        Listing(
            entries: [.file(DropboxFile(path: "\(folder)/docs/a.md", contentHash: "hash", rev: "r1"))],
            cursor: "c1", hasMore: false)
    }

    func list(after cursor: String) throws -> Listing {
        Listing(entries: [], cursor: cursor, hasMore: false)
    }

    func download(rev: String, to place: URL) throws {}

    func upload(path: String, from file: URL, over: String?, modified: Date?) throws -> DropboxFile {
        DropboxFile(path: path, contentHash: "hash", rev: "r2")
    }
}
