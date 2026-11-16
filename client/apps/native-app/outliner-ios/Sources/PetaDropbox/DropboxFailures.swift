import Foundation
import PetaBorder
import SwiftyDropbox

/// What the sync of the core makes of an error of the Dropbox SDK (ADR 054). An error that
/// says that Dropbox could not be reached is `offline`: the screen tells the user that there
/// is no connection. Any other is `other`, with the words of the SDK for the log.
func dropboxFailure(_ error: Error) -> PetaDropboxError {
    if let known = error as? PetaDropboxError { return known }
    let words = String(describing: error)
    if let call = error as? SdkCallError, call.cannotReach { return .offline(why: words) }
    if error is URLError { return .offline(why: words) }
    return .other(why: words)
}

/// An error of a call of the SDK, whatever the call. `CallError` has the error of each call in
/// its type, so it is known here by this.
protocol SdkCallError {
    /// True when the call did not reach Dropbox, as with no connection.
    var cannotReach: Bool { get }
}

extension CallError: SdkCallError {
    var cannotReach: Bool {
        switch self {
        case .clientError(.urlSessionError), .reconnectionError: return true
        default: return false
        }
    }
}
