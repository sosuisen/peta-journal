import Foundation

/// One sync with the cloud (ADR 031): the download, then the work of the core, then the
/// upload. The download comes first, so the core has taken in what the cloud changed before
/// it writes its pages.
public final class SyncRound {
    private let downloader: Downloader
    private let takeIn: () throws -> Void
    private let uploader: Uploader
    private let tries: Int

    /// `takeIn` lets the core take in the files of the Markdown folder and write the pages
    /// that changed in the app. It returns when the core is done. `tries` is how often the
    /// round runs when the cloud refuses an upload.
    public init(downloader: Downloader, takeIn: @escaping () throws -> Void, uploader: Uploader, tries: Int = 3) {
        self.downloader = downloader
        self.takeIn = takeIn
        self.uploader = uploader
        self.tries = tries
    }

    /// Runs once. When the cloud has a version of a file that the app has not seen, the round
    /// starts again: the download brings the version, and the core puts the two together.
    /// `ChangedInCloud` comes out when the cloud refused in every try.
    public func run(
        onDownload: @escaping (_ done: Int, _ total: Int) -> Void, onUpload: (_ done: Int, _ total: Int) -> Void
    ) throws {
        for _ in 1..<tries {
            do {
                return try once(onDownload, onUpload)
            } catch is ChangedInCloud {
                // The next try downloads what changed
            }
        }
        try once(onDownload, onUpload)
    }

    private func once(_ onDownload: @escaping (Int, Int) -> Void, _ onUpload: (Int, Int) -> Void) throws {
        try downloader.run(onProgress: onDownload)
        try takeIn()
        try uploader.run(onProgress: onUpload)
    }
}
