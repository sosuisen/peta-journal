import Foundation
import PetaBorder
import PetaDropbox
import PetaSync
import UIKit

/// What the app keeps while it runs: the shell with the journal inside, and the sync with
/// Dropbox. The journal can be open only once.
@MainActor
final class JournalModel: ObservableObject {
    /// How long the app waits after the last change of a page before it syncs.
    private static let afterEdit: TimeInterval = 3

    let shell: Shell
    /// `nil` when the app was built without the App key of Dropbox: it then syncs nothing.
    let cloud: CloudSync?

    /// The Markdown folder in Dropbox, such as `/PetaJournal`.
    private let dropboxFolder: String
    /// Where the last download ended, for the next one to ask only for what changed since.
    private let cursor: URL
    /// Syncs a moment after the last change of a page, and not once for each key.
    private var afterEdit: Timer?

    /// `root` holds all that the app keeps. The tests of the screen name a folder of their own.
    init(root: URL, dropboxKey: String, dropboxFolder: String) throws {
        self.dropboxFolder = dropboxFolder
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

        if dropboxKey.isEmpty {
            cloud = nil
        } else {
            DropboxAccount.setUp(key: dropboxKey)
            // The versions of the files in Dropbox that the app has seen, for the upload
            let record = CloudRecord(file: root.appendingPathComponent("dropbox-record"))
            let downloads = files.temporaryDirectory.appendingPathComponent("downloads")
            let folder = MarkdownFolder(root: markdown, temp: downloads)
            cloud = CloudSync(signedIn: DropboxAccount.client != nil) { cloud in
                SyncRound(
                    downloader: Downloader(cloud: cloud, folder: folder, cursorFile: cursor, record: record),
                    takeIn: { shell.syncAndWait() },
                    uploader: Uploader(cloud: cloud, folder: folder, record: record))
            }
        }
        edited.model = self
        // What an earlier download left, when the app ended before it was taken in
        shell.sync()
        shell.restore()
    }

    /// The user changed a page.
    fileprivate func edited() {
        afterEdit?.invalidate()
        afterEdit = Timer.scheduledTimer(withTimeInterval: JournalModel.afterEdit, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated { self?.sync() }
        }
    }

    /// Syncs with Dropbox: downloads what changed there, and uploads the pages that changed in
    /// the app. Nothing happens before the first sign-in. `inFront` says that the user waits
    /// for this sync: the screen shows it in a dialog.
    func sync(inFront: Bool = false) {
        guard let cloud, let client = DropboxAccount.client else { return }
        cloud.start(DropboxCloud(client: client, folder: dropboxFolder), inFront: inFront)
    }

    /// The app goes to the back: what was written goes up before iOS may stop the app. iOS
    /// gives the app a short time for it. A sync that iOS stops goes on when the app opens
    /// again.
    func syncInTheBack() {
        guard let cloud, DropboxAccount.client != nil else { return }
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
        DropboxAccount.signIn(over: top)
    }

    /// Dropbox opened the app with a URL, as it does at the end of the sign-in.
    func cameBack(_ url: URL) {
        guard cloud != nil else { return }
        DropboxAccount.cameBack(url) { [weak self] signedIn in
            MainActor.assumeIsolated { if signedIn { self?.sync(inFront: true) } }
        }
    }

    /// Signs out of Dropbox. The pages and the images that were downloaded stay, and can be
    /// read. The app syncs nothing until the next sign-in.
    func signOut() {
        guard let cloud else { return }
        DropboxAccount.signOut()
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
