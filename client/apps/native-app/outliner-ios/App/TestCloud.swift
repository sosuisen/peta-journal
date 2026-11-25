import CryptoKit
import Foundation
import PetaBorder
import UIKit

/// A cloud for the tests of the screen, which reach no Dropbox. A test starts the app with
/// `PETA_CLOUD` to have it:
/// - `signedOut`: not signed in yet. The sign-in ends at once, as if the user signed in.
/// - `signedIn`: signed in.
/// - `offline`: signed in, and the cloud cannot be reached.
///
/// `PETA_CLOUD_DELAY` is how many seconds each file takes to move, so that a test sees the
/// sync while it runs. The cloud has one page (`OnePage`).
final class TestCloud: CloudAccount, @unchecked Sendable {
    let isSetUp = true
    let folder = "/PetaJournal"
    private let lock = NSLock()
    private var signedIn: Bool
    private let page: OnePage
    /// Where the app takes the URL that the cloud comes back with after the sign-in.
    var cameBackTo: ((URL) -> Void)?

    /// The cloud that the environment of a test asks for, or nil for none.
    static func asked(_ environment: [String: String]) -> TestCloud? {
        let page = OnePage()
        page.delay = environment["PETA_CLOUD_DELAY"].flatMap(TimeInterval.init) ?? 0
        switch environment["PETA_CLOUD"] {
        case "signedOut": return TestCloud(signedIn: false, page: page)
        case "signedIn": return TestCloud(signedIn: true, page: page)
        case "offline":
            page.offline = true
            return TestCloud(signedIn: true, page: page)
        default: return nil
        }
    }

    private init(signedIn: Bool, page: OnePage) {
        self.signedIn = signedIn
        self.page = page
    }

    var isSignedIn: Bool { lock.withLock { signedIn } }

    /// The user signs in at once: the cloud comes back to the app, as Dropbox does.
    @MainActor
    func signIn(over screen: UIViewController) {
        let cameBackTo = cameBackTo
        DispatchQueue.main.async { cameBackTo?(URL(string: "db-test://1/connect")!) }
    }

    func cameBack(_ url: URL, then: @escaping (Bool) -> Void) -> Bool {
        lock.withLock { signedIn = true }
        DispatchQueue.main.async { then(true) }
        return true
    }

    func calls() -> PetaDropboxSdk? {
        lock.withLock { signedIn } ? page : nil
    }

    func signOut() {
        lock.withLock { signedIn = false }
    }
}

/// A Dropbox with one page, `docs/trip.md`. It keeps the paths of what is uploaded. A test can
/// change the page, as another device does: the next sync downloads it.
final class OnePage: PetaDropboxSdk, @unchecked Sendable {
    private let lock = NSLock()
    private var text = Data("京都で会議がありました。\n".utf8)
    private var rev = "first"
    private var changed = false
    private var paths: [String] = []
    /// How many seconds each file takes to move.
    var delay: TimeInterval = 0
    /// True when the cloud cannot be reached.
    var offline = false

    /// The paths that were uploaded, as Dropbox names them.
    var uploaded: [String] { lock.withLock { paths } }

    /// Another device uploads the page with this text.
    func change(to text: String) {
        lock.withLock {
            self.text = Data(text.utf8)
            rev = "changed-\(UUID().uuidString)"
            changed = true
        }
    }

    private func entry(in folder: String) -> PetaDropboxEntry {
        lock.withLock {
            .file(file: PetaDropboxFile(path: "\(folder)/docs/trip.md", contentHash: contentHash(of: text), rev: rev))
        }
    }

    private func reach() throws {
        if offline { throw PetaDropboxError.offline(why: "The test has no connection") }
    }

    func list(folder: String) throws -> PetaListing {
        try reach()
        return PetaListing(entries: [entry(in: folder)], cursor: "c1", hasMore: false)
    }

    func listAfter(cursor: String) throws -> PetaListing {
        try reach()
        let news = lock.withLock { () -> Bool in
            defer { changed = false }
            return changed
        }
        return PetaListing(entries: news ? [entry(in: "/PetaJournal")] : [], cursor: "c1", hasMore: false)
    }

    func download(rev: String, to place: String) throws {
        try reach()
        Thread.sleep(forTimeInterval: delay)
        try lock.withLock { text }.write(to: URL(fileURLWithPath: place))
    }

    func upload(path: String, from file: String, over: String?, modifiedMs: Int64) throws -> PetaDropboxFile {
        try reach()
        Thread.sleep(forTimeInterval: delay)
        let content = try Data(contentsOf: URL(fileURLWithPath: file))
        lock.withLock { paths.append(path) }
        return PetaDropboxFile(path: path, contentHash: contentHash(of: content), rev: "next")
    }

    /// The content hash of Dropbox, for a file of less than 4 MB: the hash of the hash of its
    /// content. The sync compares it with the hash of the file in the app.
    private func contentHash(of content: Data) -> String {
        let block = Data(SHA256.hash(data: content))
        return SHA256.hash(data: block).map { String(format: "%02x", $0) }.joined()
    }
}
