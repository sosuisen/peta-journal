import Foundation
import SwiftyDropbox

#if os(iOS)
import UIKit
#endif

/// The sign-in to Dropbox of the app. The SDK keeps the token in the keychain of the device
/// and gets a new access token by itself when the old one has run out.
public enum DropboxAccount {
    /// The app reads the folder in Dropbox, and writes the pages that changed in the app. It
    /// deletes and moves nothing there (ADR 031).
    private static let scopes = ["files.metadata.read", "files.content.read", "files.content.write"]

    /// Tells the SDK the App key of the build. It is called once, before anything else here.
    public static func setUp(key: String) {
        #if os(iOS)
        DropboxClientsManager.setupWithAppKey(key)
        #else
        DropboxClientsManager.setupWithAppKeyDesktop(key)
        #endif
    }

    /// The client of the user who has signed in, or `nil` before the sign-in.
    public static var client: DropboxClient? { DropboxClientsManager.authorizedClient }

    #if os(iOS)
    /// Starts the sign-in, with PKCE: the app has no secret. The Dropbox app signs in when it
    /// is on the device, and else a browser over `screen`. Dropbox comes back with a URL of
    /// the scheme `db-<App key>`, which `cameBack` takes.
    @MainActor
    public static func signIn(over screen: UIViewController) {
        DropboxClientsManager.authorizeFromControllerV2(
            UIApplication.shared,
            controller: screen,
            loadingStatusDelegate: nil,
            openURL: { UIApplication.shared.open($0) },
            scopeRequest: ScopeRequest(scopeType: .user, scopes: scopes, includeGrantedScopes: false)
        )
    }
    #endif

    /// Takes the URL that Dropbox opened the app with after the sign-in. `then` is called on
    /// the main thread, with true when the user has signed in. False when the URL is not of
    /// Dropbox.
    @discardableResult
    public static func cameBack(_ url: URL, then: @escaping (Bool) -> Void) -> Bool {
        DropboxClientsManager.handleRedirectURL(url, includeBackgroundClient: false) { result in
            var signedIn = false
            if case .success = result { signedIn = true }
            DispatchQueue.main.async { then(signedIn) }
        }
    }

    /// Signs out. Dropbox is told that the token is no longer to be taken. Without the
    /// network, Dropbox does not hear of it: the token is then gone from the device only.
    public static func signOut() {
        // The client lives until Dropbox has answered
        let leaving = client
        leaving?.auth.tokenRevoke().response { _, _ in _ = leaving }
        DropboxClientsManager.unlinkClients()
    }
}
