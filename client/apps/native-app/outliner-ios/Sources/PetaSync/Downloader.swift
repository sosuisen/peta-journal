import Foundation

/// Brings the Markdown folder of the app up to date with the folder in the cloud. What the
/// cloud has is right, and what it lost is deleted here (ADR 030). It writes down the versions
/// it saw, for the upload (ADR 031).
///
/// A file that the cloud lost stays here when it holds what the cloud never had: a change of
/// the app that is not uploaded yet, or a page that was made in the app. The upload brings it
/// to the cloud as a new file. What was written on the phone is not lost in silence.
public final class Downloader {
    private let cloud: Cloud
    private let folder: MarkdownFolder
    private let cursorFile: URL
    private let record: CloudRecord
    private let threads: Int

    /// `cursorFile` keeps where the last run ended, so that the next one asks only for what
    /// changed since. `threads` is how many files are downloaded at the same time. One after
    /// the other, a file takes about a second.
    public init(cloud: Cloud, folder: MarkdownFolder, cursorFile: URL, record: CloudRecord, threads: Int = 8) {
        self.cloud = cloud
        self.folder = folder
        self.cursorFile = cursorFile
        self.record = record
        self.threads = threads
    }

    /// Runs once. `onProgress` gets how many files are downloaded, of how many: first with 0,
    /// then after each file, in order but from other threads.
    ///
    /// When the cloud or a download fails, the error comes out and the cursor stays. The next
    /// run asks for the same changes again, and downloads only what is still missing.
    public func run(onProgress: @escaping (_ done: Int, _ total: Int) -> Void) throws {
        let changes = try cloud.changes(since: try? String(contentsOf: cursorFile, encoding: .utf8))
        // By the path in lower case, as the cloud tells no case apart. In the order they came
        var files: [String: CloudFile] = [:]
        var order: [String] = []
        for change in changes.entries {
            switch change {
            case .gone(let path):
                let key = path.lowercased()
                order.removeAll { $0 == key || $0.hasPrefix(key + "/") }
                files = files.filter { order.contains($0.key) }
                folder.remove(path, keep: isUnsent)
                record.remove(path)
            case .file(let file):
                let key = file.path.lowercased()
                guard MarkdownFolder.keeps(key) else { continue }
                if files[key] == nil { order.append(key) }
                files[key] = file
            }
        }
        if changes.whole {
            folder.keepOnly(Set(order), keep: isUnsent)
            record.keepOnly(Set(order))
        }
        let all = order.compactMap { files[$0] }
        try download(all.filter { hashHere($0.path) != $0.contentHash }, onProgress)
        all.forEach(record.put)
        try record.save()
        try Data(changes.cursor.utf8).write(to: cursorFile, options: .atomic)
    }

    /// True for a file here whose content the cloud never had.
    private func isUnsent(_ path: String) -> Bool {
        hashHere(path) != record[path]?.contentHash
    }

    /// The hash of the file here, as the cloud makes it. `nil` when it is not here.
    private func hashHere(_ path: String) -> String? {
        folder.contentHash(path, by: cloud.contentHash)
    }

    private func download(_ files: [CloudFile], _ onProgress: @escaping (Int, Int) -> Void) throws {
        onProgress(0, files.count)
        let told = NSLock()
        var done = 0
        var failure: Error?
        let pool = OperationQueue()
        pool.maxConcurrentOperationCount = threads
        for file in files {
            pool.addOperation { [cloud, folder] in
                do {
                    try folder.put(file.path) { try cloud.download(file, to: $0) }
                    told.withLock {
                        done += 1
                        onProgress(done, files.count)
                    }
                } catch {
                    told.withLock { failure = failure ?? error }
                }
            }
        }
        pool.waitUntilAllOperationsAreFinished()
        if let failure { throw failure }
    }
}
