import Foundation
import PetaBorder

#if os(iOS)
    import UIKit
#endif

/// The account of Dropbox that the app is signed in to: the sign-in, the sign-out, and the
/// calls of the SDK that the sync of the core makes. All that the app knows of the Dropbox SDK
/// outside this package goes through here.
public final class DropboxAccount: CloudAccount {
    /// The app reads the folder in Dropbox, and writes the pages that changed in the app. It
    /// deletes and moves nothing there (ADR 031).
    private static let scopes = ["files.metadata.read", "files.content.read", "files.content.write"]

    private let key: String
    public let folder: String
    private let sdk: DropboxSdk

    /// The account with the Dropbox SDK. `key` is the App key of the Dropbox app, and it is
    /// empty in an app that was built without one. `folder` is the path of the Markdown folder
    /// in Dropbox, such as `/PetaJournal`. It is made once: the SDK takes its key only once.
    public convenience init(key: String, folder: String) {
        self.init(key: key, folder: folder, sdk: RealDropboxSdk())
    }

    init(key: String, folder: String, sdk: DropboxSdk) {
        self.key = key
        self.folder = folder
        self.sdk = sdk
        if !key.isEmpty { sdk.setUp(key: key) }
    }

    public var isSetUp: Bool { !key.isEmpty }

    public var isSignedIn: Bool { isSetUp && sdk.isSignedIn }

    #if os(iOS)
        @MainActor
        public func signIn(over screen: UIViewController) {
            guard isSetUp else { return }
            sdk.startSignIn(over: screen, scopes: DropboxAccount.scopes)
        }
    #endif

    @discardableResult
    public func cameBack(_ url: URL, then: @escaping (Bool) -> Void) -> Bool {
        guard isSetUp else { return false }
        return sdk.endSignIn(url) { signedIn in DispatchQueue.main.async { then(signedIn) } }
    }

    public func calls() -> PetaDropboxSdk? {
        guard isSetUp else { return nil }
        return sdk.calls()
    }

    public func signOut() {
        guard isSetUp else { return }
        sdk.signOut()
    }
}
