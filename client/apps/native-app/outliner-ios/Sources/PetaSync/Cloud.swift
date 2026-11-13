import Foundation

/// What the sync needs of the cloud storage: the Markdown folder there, as Dropbox tells of
/// it. A path is relative to that folder, with `/` between its parts and none at its start.
/// The cloud does not tell names apart by upper and lower case.
///
/// Every call waits for the cloud, so none is for the main thread.
public protocol Cloud {
    /// What changed in the folder since `cursor`, in the order it happened. Without a cursor,
    /// or when the cloud no longer knows the cursor, it is everything that is in the folder.
    func changes(since cursor: String?) throws -> Changes

    /// Writes the content of the file to `place`, where no file is yet.
    func download(_ file: CloudFile, to place: URL) throws

    /// Puts the content of `file` at the path. `over` names the version that this content
    /// replaces, as `CloudFile.remote` does, and it is `nil` for a file that the cloud has
    /// not. Answers with the file as the cloud has it now.
    ///
    /// Throws `ChangedInCloud` when the cloud has another version than `over`, or a file
    /// where none was expected. Nothing is written then.
    func upload(path: String, from file: URL, over: String?) throws -> CloudFile
}

/// The cloud has a version of the file that the app has not seen yet.
public struct ChangedInCloud: Error, Equatable {
    public let path: String

    public init(path: String) {
        self.path = path
    }
}

public struct Changes {
    public let entries: [Change]
    /// Names the state after the entries, to ask for the changes after it.
    public let cursor: String
    /// True when the entries are all that the folder holds, and not changes: a file that is
    /// not among them is not in the cloud.
    public let whole: Bool

    public init(entries: [Change], cursor: String, whole: Bool) {
        self.entries = entries
        self.cursor = cursor
        self.whole = whole
    }
}

public enum Change: Equatable {
    /// A file that is new or has new content.
    case file(CloudFile)
    /// A file, or a folder with all that was in it, that is no longer there.
    case gone(path: String)
}

/// One version of a file in the cloud.
public struct CloudFile: Equatable, Codable {
    public let path: String
    /// The hash that Dropbox makes of the content.
    public let contentHash: String
    /// How the cloud finds this very version of the file.
    public let remote: String

    public init(path: String, contentHash: String, remote: String) {
        self.path = path
        self.contentHash = contentHash
        self.remote = remote
    }
}
