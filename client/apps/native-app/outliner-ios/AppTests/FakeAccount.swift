import CryptoKit
import Foundation
import PetaSync
import UIKit

/// An account of a cloud with one page. It keeps what the app asked of it.
final class FakeAccount: CloudAccount, @unchecked Sendable {
    let isSetUp: Bool
    private let lock = NSLock()
    private var signedIn: Bool
    private var asked = 0
    /// What the URL that the storage comes back with says: that the user signed in.
    var fresh = false
    let page = OnePage()

    init(isSetUp: Bool = true, signedIn: Bool = true) {
        self.isSetUp = isSetUp
        self.signedIn = signedIn
    }

    var isSignedIn: Bool { lock.withLock { signedIn } }

    /// How often the cloud was asked for: once for each sync that the app started.
    var syncs: Int { lock.withLock { asked } }

    @MainActor
    func signIn(over screen: UIViewController) {}

    func cameBack(_ url: URL, then: @escaping (Bool) -> Void) -> Bool {
        let fresh = fresh
        if fresh { lock.withLock { signedIn = true } }
        DispatchQueue.main.async { then(fresh) }
        return true
    }

    func cloud() -> Cloud? {
        lock.withLock {
            guard signedIn else { return nil }
            asked += 1
            return page
        }
    }

    func signOut() {
        lock.withLock { signedIn = false }
    }
}

/// A cloud with one page. It keeps what is uploaded.
final class OnePage: Cloud, @unchecked Sendable {
    private let text = Data("京都で会議がありました。\n".utf8)

    func changes(since cursor: String?) throws -> Changes {
        let whole = cursor == nil
        let page = CloudFile(path: "docs/trip.md", contentHash: hashOf(text), remote: "first")
        return Changes(entries: whole ? [.file(page)] : [], cursor: "c1", whole: whole)
    }

    func download(_ file: CloudFile, to place: URL) throws {
        try text.write(to: place)
    }

    func upload(path: String, from file: URL, over: String?) throws -> CloudFile {
        CloudFile(path: path, contentHash: contentHash(of: file) ?? "", remote: "next")
    }

    /// The content hash of this cloud. It is not the one of Dropbox: the sync asks the cloud.
    func contentHash(of file: URL) -> String? {
        (try? Data(contentsOf: file)).map(hashOf)
    }

    private func hashOf(_ content: Data) -> String {
        SHA256.hash(data: content).map { String(format: "%02x", $0) }.joined()
    }
}
