import Foundation

#if os(iOS)
import UIKit
#endif

/// The account of the cloud storage that the app is signed in to: the sign-in, the sign-out,
/// and the Markdown folder of the account. The app knows the storage only by this, so a test
/// puts its own account here.
public protocol CloudAccount: AnyObject {
    /// False in an app that was built without what the storage asks of an app: it syncs
    /// nothing.
    var isSetUp: Bool { get }

    var isSignedIn: Bool { get }

    #if os(iOS)
    /// Starts the sign-in over `screen`. The storage opens the app with a URL when it has
    /// ended, which `cameBack` takes.
    @MainActor
    func signIn(over screen: UIViewController)
    #endif

    /// Takes the URL that the storage opened the app with after the sign-in. `then` is called
    /// on the main thread, with true when the user has signed in. False when the URL is not
    /// of the storage.
    @discardableResult
    func cameBack(_ url: URL, then: @escaping (Bool) -> Void) -> Bool

    /// The Markdown folder of the account. `nil` before the sign-in.
    func cloud() -> Cloud?

    /// Lets go of the sign-in, and tells the storage that it is no longer to be taken.
    func signOut()
}
