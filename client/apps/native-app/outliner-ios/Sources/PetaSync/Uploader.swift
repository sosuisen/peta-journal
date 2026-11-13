import Foundation

/// Brings the pages that changed in the app to the folder in the cloud (ADR 031). It uploads
/// the Markdown files whose content is not what the cloud had when the app last heard of it.
/// It deletes and moves nothing in the cloud.
public final class Uploader {
    private let cloud: Cloud
    private let folder: MarkdownFolder
    private let record: CloudRecord

    public init(cloud: Cloud, folder: MarkdownFolder, record: CloudRecord) {
        self.cloud = cloud
        self.folder = folder
        self.record = record
    }

    /// Runs once, after a download and after the core wrote its pages. `onProgress` gets how
    /// many files are uploaded, of how many: first with 0, then after each file.
    ///
    /// `ChangedInCloud` comes out when the cloud has a version of a file that the app has not
    /// seen: the file there is not written over, and the sync is to start again with a
    /// download. What was uploaded before that, or before another failure, stays uploaded.
    public func run(onProgress: (_ done: Int, _ total: Int) -> Void) throws {
        let changed = folder.pages().filter { folder.contentHash($0, by: cloud.contentHash) != record[$0]?.contentHash }
        onProgress(0, changed.count)
        defer { try? record.save() }
        for (done, path) in changed.enumerated() {
            record.put(try cloud.upload(path: path, from: folder.file(path), over: record[path]?.remote))
            onProgress(done + 1, changed.count)
        }
    }
}
