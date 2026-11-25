import Foundation
import PetaBorder
import UIKit

@testable import PetaJournal

/// An account of a cloud with one page (`OnePage` of the app). It keeps what the app asked of it.
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
