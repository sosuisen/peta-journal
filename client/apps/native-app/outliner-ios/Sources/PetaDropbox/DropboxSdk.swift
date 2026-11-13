import Foundation
import SwiftyDropbox

#if os(iOS)
import UIKit
#endif

/// What `DropboxAccount` needs of the Dropbox SDK. The SDK keeps the token in the keychain of
/// the device by itself. A test puts its own SDK here. The app puts `RealDropboxSdk` here.
protocol DropboxSdk {
    /// Tells the SDK the App key of the build. It is called once, before anything else here.
    func setUp(key: String)

    /// True when the SDK has the token of a sign-in.
    var isSignedIn: Bool { get }

    #if os(iOS)
    /// Starts the sign-in over `screen`, for the scopes that the app asks for.
    @MainActor
    func startSignIn(over screen: UIViewController, scopes: [String])
    #endif

    /// Takes the URL that Dropbox opened the app with. `then` gets true when the user has
    /// signed in. False when the URL is not of Dropbox.
    func endSignIn(_ url: URL, then: @escaping (Bool) -> Void) -> Bool

    /// The files of the account that has signed in. `nil` before the sign-in.
    func files() -> DropboxFiles?

    /// Lets go of the token, and tells Dropbox that it is no longer to be taken. It does not
    /// wait for the answer.
    func signOut()
}

/// The Dropbox SDK itself. It only passes each call on. No test runs it.
struct RealDropboxSdk: DropboxSdk {
    func setUp(key: String) {
        #if os(iOS)
        DropboxClientsManager.setupWithAppKey(key)
        #else
        DropboxClientsManager.setupWithAppKeyDesktop(key)
        #endif
    }

    // The client gets a new access token by itself when the old one has run out
    var isSignedIn: Bool { DropboxClientsManager.authorizedClient != nil }

    #if os(iOS)
    /// With PKCE: the app has no secret. The Dropbox app signs in when it is on the device,
    /// and else a browser over `screen`. Dropbox comes back with a URL of the scheme
    /// `db-<App key>`.
    @MainActor
    func startSignIn(over screen: UIViewController, scopes: [String]) {
        DropboxClientsManager.authorizeFromControllerV2(
            UIApplication.shared,
            controller: screen,
            loadingStatusDelegate: nil,
            openURL: { UIApplication.shared.open($0) },
            scopeRequest: ScopeRequest(scopeType: .user, scopes: scopes, includeGrantedScopes: false)
        )
    }
    #endif

    func endSignIn(_ url: URL, then: @escaping (Bool) -> Void) -> Bool {
        DropboxClientsManager.handleRedirectURL(url, includeBackgroundClient: false) { result in
            var signedIn = false
            if case .success = result { signedIn = true }
            then(signedIn)
        }
    }

    func files() -> DropboxFiles? {
        DropboxClientsManager.authorizedClient.map(SdkDropboxFiles.init)
    }

    /// Without the network, Dropbox does not hear of it: the token is then gone from the
    /// device only.
    func signOut() {
        // The client lives until Dropbox has answered
        let leaving = DropboxClientsManager.authorizedClient
        leaving?.auth.tokenRevoke().response { _, _ in _ = leaving }
        DropboxClientsManager.unlinkClients()
    }
}
