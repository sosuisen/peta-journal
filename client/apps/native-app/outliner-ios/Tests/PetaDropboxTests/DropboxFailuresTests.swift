import PetaBorder
import SwiftyDropbox
import XCTest

@testable import PetaDropbox

/// What the sync makes of the errors of the Dropbox SDK. The screen tells the user only
/// whether there was no connection (ADR 054).
final class DropboxFailuresTests: XCTestCase {
    // SDKが通信の途中で失敗したときは、通信の失敗になる。iPhoneで通信を切ったときに出た種類である。
    func testAnErrorOfTheUrlSessionOfTheSdkIsOffline() {
        let failure = CallError<Files.ListFolderError>.clientError(.urlSessionError(URLError(.notConnectedToInternet)))
        guard case .offline = dropboxFailure(failure) else { return XCTFail("\(dropboxFailure(failure))") }
    }

    // SDKがつなぎ直しに失敗したときも、通信の失敗になる。
    func testAFailedReconnectionOfTheSdkIsOffline() {
        let failure = CallError<Files.UploadError>.reconnectionError(URLError(.timedOut))
        guard case .offline = dropboxFailure(failure) else { return XCTFail("\(dropboxFailure(failure))") }
    }

    // URLの読み込みの失敗も、通信の失敗になる。
    func testAnErrorOfUrlLoadingIsOffline() {
        guard case .offline = dropboxFailure(URLError(.networkConnectionLost)) else { return XCTFail() }
    }

    // Dropboxが答えたうえでの失敗は、通信の失敗ではない。SDKの言葉を持つ。
    func testAnErrorThatDropboxAnsweredIsAnotherFailureWithTheWordsOfTheSdk() {
        let failure = CallError<Files.ListFolderError>.httpError(500, "the server is busy", nil)
        guard case .other(let why) = dropboxFailure(failure) else { return XCTFail() }
        XCTAssertTrue(why.contains("the server is busy"))
    }

    // 同期の分かる失敗は、そのまま渡る。
    func testAFailureThatTheSyncKnowsGoesOnAsItIs() {
        XCTAssertEqual(dropboxFailure(PetaDropboxError.uploadConflict), .uploadConflict)
    }
}
