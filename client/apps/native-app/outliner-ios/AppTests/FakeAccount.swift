import CryptoKit
import Foundation
import PetaBorder
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
    let folder = "/PetaJournal"

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

    func calls() -> PetaDropboxSdk? {
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

/// A Dropbox with one page. It keeps the paths of what is uploaded.
final class OnePage: PetaDropboxSdk, @unchecked Sendable {
    private let text = Data("京都で会議がありました。\n".utf8)
    private let lock = NSLock()
    private var paths: [String] = []

    /// The paths that were uploaded, as Dropbox names them.
    var uploaded: [String] { lock.withLock { paths } }

    func list(folder: String) throws -> PetaListing {
        let page = PetaDropboxFile(path: "\(folder)/docs/trip.md", contentHash: contentHash(of: text), rev: "first")
        return PetaListing(entries: [.file(file: page)], cursor: "c1", hasMore: false)
    }

    func listAfter(cursor: String) throws -> PetaListing {
        PetaListing(entries: [], cursor: "c1", hasMore: false)
    }

    func download(rev: String, to place: String) throws {
        try text.write(to: URL(fileURLWithPath: place))
    }

    func upload(path: String, from file: String, over: String?, modifiedMs: Int64) throws -> PetaDropboxFile {
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
