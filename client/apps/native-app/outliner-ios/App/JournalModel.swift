import Foundation
import PetaBorder
import PetaSync
import UIKit

/// What the app keeps while it runs: the shell with the journal inside, and the sync with
/// Dropbox. The journal can be open only once.
@MainActor
final class JournalModel: ObservableObject {
    let shell: Shell
    /// `nil` when the account is not set up, as in an app that was built without the App key
    /// of Dropbox: it then syncs nothing.
    let cloud: CloudSync?

    /// The account of the cloud storage.
    private let account: CloudAccount
    /// How long the app waits after the last change of a page before it syncs.
    private let waitAfterEdit: TimeInterval
    /// Where the last download ended, for the next one to ask only for what changed since.
    private let cursor: URL
    /// Syncs a moment after the last change of a page, and not once for each key.
    private var afterEdit: Timer?

    /// `root` holds all that the app keeps. The tests of the screen name a folder of their own.
    /// `afterEdit` is how long the app waits after the last change of a page before it syncs.
    init(root: URL, account: CloudAccount, afterEdit: TimeInterval = 3) throws {
        self.account = account
        waitAfterEdit = afterEdit
        let files = FileManager.default
        // The folders of ADR 030. The app makes the folders of the pages: the core takes in
        // nothing from a Markdown folder that has neither of them
        let markdown = root.appendingPathComponent("markdown")
        for name in ["docs", "journals", "assets"] {
            try files.createDirectory(at: markdown.appendingPathComponent(name), withIntermediateDirectories: true)
        }
        let data = root.appendingPathComponent("data")
        try files.createDirectory(at: data, withIntermediateDirectories: true)
        // The data of the app is a copy of Dropbox, so it is in no backup (ADR 030)
        var kept = root
        var noBackup = URLResourceValues()
        noBackup.isExcludedFromBackup = true
        try? kept.setResourceValues(noBackup)

        cursor = root.appendingPathComponent("dropbox-cursor")
        let cursor = cursor
        let edited = Edited()
        let shell = try Shell(dataDir: data, markdownDir: markdown) { edited.tell() }
        self.shell = shell

        if !account.isSetUp {
            cloud = nil
        } else {
            // The versions of the files in Dropbox that the app has seen, for the upload
            let record = CloudRecord(file: root.appendingPathComponent("dropbox-record"))
            let downloads = files.temporaryDirectory.appendingPathComponent("downloads")
            let folder = MarkdownFolder(root: markdown, temp: downloads)
            cloud = CloudSync(signedIn: account.isSignedIn) { cloud in
                SyncRound(
                    downloader: Downloader(cloud: cloud, folder: folder, cursorFile: cursor, record: record),
                    takeIn: { shell.syncAndWait() },
                    uploader: Uploader(cloud: cloud, folder: folder, record: record))
            }
        }
        edited.model = self
        // What an earlier download left, when the app ended before it was taken in
        shell.sync()
        // Before the sign-in, the journal of today is not made: the first download may bring it
        shell.restore(make: cloud == nil || account.isSignedIn)
    }

    /// The user changed a page.
    fileprivate func edited() {
        afterEdit?.invalidate()
        afterEdit = Timer.scheduledTimer(withTimeInterval: waitAfterEdit, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated { self?.sync() }
        }
    }

    /// Syncs with Dropbox: downloads what changed there, and uploads the pages that changed in
    /// the app. Nothing happens before the first sign-in. `inFront` says that the user waits
    /// for this sync: the screen shows it in a dialog.
    func sync(inFront: Bool = false) {
        guard let cloud, let folder = account.cloud() else { return }
        cloud.start(folder, inFront: inFront)
    }

    /// The app goes to the back: what was written goes up before iOS may stop the app. iOS
    /// gives the app a short time for it. A sync that iOS stops goes on when the app opens
    /// again.
    func syncInTheBack() {
        guard let cloud, account.isSignedIn else { return }
        afterEdit?.invalidate()
        var task = UIBackgroundTaskIdentifier.invalid
        var watch: Timer?
        let end = {
            watch?.invalidate()
            if task != .invalid { UIApplication.shared.endBackgroundTask(task) }
            task = .invalid
        }
        task = UIApplication.shared.beginBackgroundTask(withName: "sync", expirationHandler: end)
        sync()
        // The sync tells the screen where it stands, and nobody else: the task asks
        watch = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { _ in
            MainActor.assumeIsolated { if !cloud.state.isRunning { end() } }
        }
    }

    /// Starts the sign-in to Dropbox, over the screen that is in front.
    func signIn() {
        let scene = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first
        guard var top = scene?.keyWindow?.rootViewController else { return }
        while let over = top.presentedViewController { top = over }
        account.signIn(over: top)
    }

    /// Dropbox opened the app with a URL, as it does at the end of the sign-in.
    func cameBack(_ url: URL) {
        guard cloud != nil else { return }
        account.cameBack(url) { [weak self] signedIn in
            MainActor.assumeIsolated { if signedIn { self?.sync(inFront: true) } }
        }
    }

    /// Signs out of Dropbox. The pages and the images that were downloaded stay, and can be
    /// read. The app syncs nothing until the next sign-in.
    func signOut() {
        guard let cloud else { return }
        account.signOut()
        // The next sign-in may be to another account, where this cursor means nothing. Without
        // it, the next download lists everything: it takes only the files that differ, and
        // deletes the copies of what the account does not hold. The record of the versions
        // stays: it tells a copy from what was written here and is not uploaded yet
        try? FileManager.default.removeItem(at: cursor)
        cloud.signOut()
    }
}

/// Where the shell finds the model, which is made after the shell.
private final class Edited {
    weak var model: JournalModel?

    func tell() {
        MainActor.assumeIsolated { model?.edited() }
    }
}
