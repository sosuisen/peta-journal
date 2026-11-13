import Combine
import Foundation

/// Where the sync with the cloud stands, for the screen.
public enum SyncState: Equatable {
    /// The user has not signed in to the cloud yet.
    case signedOut
    /// Nothing runs.
    case idle
    case downloading(done: Int, total: Int)
    case uploading(done: Int, total: Int)
    /// The last sync failed. What it had downloaded and uploaded stays.
    case failed(reason: String)

    /// True while a sync runs.
    public var isRunning: Bool {
        switch self {
        case .downloading, .uploading: return true
        case .signedOut, .idle, .failed: return false
        }
    }
}

/// Runs the syncs with the cloud, one at a time, and tells the screen where they stand. It is
/// used on the main thread; a sync runs on a thread of its own.
public final class CloudSync: ObservableObject {
    @Published public private(set) var state: SyncState
    /// Whether the sync that runs is one the user waits for: the app came to the front, and
    /// the pages may be old until the sync rests. The screen then shows where it stands in a
    /// dialog. A sync after an edit is not in front: the user goes on typing.
    @Published public private(set) var inFront = false

    private let round: (Cloud) -> SyncRound
    private let onMainThread: (@escaping () -> Void) -> Void
    /// The cloud of a sync that was asked for while another ran.
    private var next: Cloud?

    /// `round` makes one sync with the cloud: download, work of the core, upload.
    /// `onMainThread` runs what it is given on the main thread, later and in order.
    public init(
        signedIn: Bool, round: @escaping (Cloud) -> SyncRound,
        onMainThread: @escaping (@escaping () -> Void) -> Void = { DispatchQueue.main.async(execute: $0) }
    ) {
        state = signedIn ? .idle : .signedOut
        self.round = round
        self.onMainThread = onMainThread
    }

    /// The user has signed out of the cloud. What was downloaded stays. It is not for the time
    /// a sync runs: the screen offers no sign-out then.
    public func signOut() {
        precondition(!state.isRunning, "a sync runs")
        state = .signedOut
    }

    /// Starts a sync with the cloud. While one runs, another starts after it, once: what was
    /// typed since the running one read the pages is not left behind. A sync that runs comes
    /// to the front when `inFront` asks for it.
    public func start(_ cloud: Cloud, inFront: Bool = false) {
        if inFront { self.inFront = true }
        if state.isRunning {
            next = cloud
            return
        }
        next = nil
        state = .downloading(done: 0, total: 0)
        let round = round(cloud)
        let onMainThread = onMainThread
        Thread.detachNewThread { [weak self] in
            do {
                try round.run(
                    onDownload: { done, total in
                        onMainThread { self?.state = .downloading(done: done, total: total) }
                    },
                    // Most syncs have nothing to upload, and say nothing of it
                    onUpload: { done, total in
                        if total > 0 { onMainThread { self?.state = .uploading(done: done, total: total) } }
                    })
                onMainThread { self?.rested() }
            } catch {
                onMainThread {
                    self?.inFront = false
                    self?.state = .failed(reason: CloudSync.reason(of: error))
                }
            }
        }
    }

    private func rested() {
        state = .idle
        // The sync that waited goes on in front, when this one was
        if let waits = next { start(waits) } else { inFront = false }
    }

    private static func reason(of error: Error) -> String {
        if let changed = error as? ChangedInCloud { return "\(changed.path) changed in the cloud" }
        return error.localizedDescription
    }
}
