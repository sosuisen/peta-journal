import Foundation

/// What the app knows of the files in the cloud: the version and the content hash of each, as
/// the last download or upload left it (ADR 031). A file here whose hash differs from it has
/// changed in the app. An upload names the version, and the cloud refuses when it has another.
///
/// A path is looked up without regard to case, as the cloud does.
public final class CloudRecord: @unchecked Sendable {
    private let file: URL
    private let lock = NSLock()
    private var known: [String: CloudFile]

    /// `file` keeps the record between two runs of the app.
    public init(file: URL) {
        self.file = file
        let kept = (try? Data(contentsOf: file)).flatMap { try? JSONDecoder().decode([String: CloudFile].self, from: $0) }
        known = kept ?? [:]
    }

    /// The file as the cloud had it when the app last heard of it. `nil` for a file it has not.
    public subscript(path: String) -> CloudFile? {
        lock.withLock { known[path.lowercased()] }
    }

    public func put(_ version: CloudFile) {
        lock.withLock { known[version.path.lowercased()] = version }
    }

    /// Forgets the file, or the folder with all that is in it: it has gone from the cloud.
    public func remove(_ path: String) {
        let key = path.lowercased()
        lock.withLock { known = known.filter { $0.key != key && !$0.key.hasPrefix(key + "/") } }
    }

    /// Forgets every file whose path is not in `paths`, which are in lower case.
    public func keepOnly(_ paths: Set<String>) {
        lock.withLock { known = known.filter { paths.contains($0.key) } }
    }

    /// Writes the record to its file.
    public func save() throws {
        let kept = lock.withLock { known }
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        try encoder.encode(kept).write(to: file, options: .atomic)
    }
}
