import Combine
import Foundation
// The app and its tests name the types of the sync, which the core makes (ADR 054)
@_exported import PetaCore
import os

/// Where the sync with the cloud stands, for the screen.
public typealias SyncState = PetaSyncState

extension PetaSyncState {
    /// True while files move: nothing more is asked of the sync then.
    public var isRunning: Bool {
        switch self {
        case .downloading, .uploading: return true
        case .signedOut, .idle, .failed: return false
        }
    }
}

/// The syncs with the cloud, for the screen. The core runs them, one at a time, on a thread of
/// its own (ADR 054); this keeps where they stand on the main thread.
@MainActor
public final class CloudSync: ObservableObject {
    @Published public private(set) var state: SyncState
    /// Whether the sync that runs is one the user waits for: the app came to the front, and
    /// the pages may be old until the sync rests. The screen then shows where it stands in a
    /// dialog.
    @Published public private(set) var inFront = false

    private let sync: PetaCloudSync

    /// The sync puts the downloads in the Markdown folder of the core of `shell`, and `temp`
    /// is where a download is written before it is whole. `cursor` keeps where the last
    /// download ended, and `record` the versions of the files in the cloud that the app has
    /// seen. The core takes in the files through `shell`, and tells the sync when a page
    /// changed: the sync starts when no page has changed for `afterChange` seconds (ADR 055).
    public init(
        temp: URL, cursor: URL, record: URL, signedIn: Bool, shell: Shell,
        afterChange: TimeInterval
    ) {
        let host = Host(shell: shell)
        sync = PetaCloudSync(
            core: shell.core,
            afterChangeMs: UInt64(afterChange * 1000),
            tempDir: temp.path,
            cursorFile: cursor.path,
            recordFile: record.path,
            signedIn: signedIn,
            host: host)
        state = sync.state()
        host.cloud = self
        // A word that the keyboard composes is not saved yet, and the sync waits for it
        // Not the sync itself: it holds the shell through its host, and the two would keep
        // each other
        shell.onComposing = { [weak self] in self?.sync.typing() }
    }

    /// Starts a sync with the Markdown folder at the path `folder` of the cloud. While one
    /// runs, another starts after it, once. `inFront` says that the user waits for it.
    public func start(_ sdk: PetaDropboxSdk, folder: String, inFront: Bool) {
        sync.start(dropbox: sdk, folder: folder, inFront: inFront)
        // The core tells of it a moment later, on the main queue. The screen must not offer
        // the sign-out meanwhile: the core does not take it while a sync runs
        read()
    }

    /// The user has signed out. What was downloaded stays. The core forgets where the last
    /// download ended: the next sign-in may be to another account. Not for the time a sync
    /// runs: the screen offers no sign-out then.
    public func signOut() {
        sync.signOut()
        read()
    }

    /// Reads where the sync stands from the core. What the core told on its thread may be old
    /// by the time it reaches this queue: a sync may have started since.
    fileprivate func read() {
        state = sync.state()
        inFront = sync.inFront()
    }
}

/// What the sync of the core asks of the app. It is called on the threads of the sync.
private final class Host: PetaSyncHost, @unchecked Sendable {
    private let shell: Shell
    weak var cloud: CloudSync?
    private static let log = Logger(subsystem: "com.sosuisha.petajournal", category: "sync")

    init(shell: Shell) {
        self.shell = shell
    }

    func takeIn() {
        shell.syncAndWait()
    }

    func told(state: PetaSyncState, inFront: Bool) {
        DispatchQueue.main.async { [weak self] in
            MainActor.assumeIsolated { self?.cloud?.read() }
        }
    }

    /// The words of the SDK are long, and mean nothing to the user: they go to the log only.
    func log(failure: String) {
        Host.log.warning("The sync failed: \(failure, privacy: .public)")
    }
}
