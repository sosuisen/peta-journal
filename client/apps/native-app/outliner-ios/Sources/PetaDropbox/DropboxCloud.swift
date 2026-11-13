import Foundation
import PetaSync
import SwiftyDropbox

/// The Markdown folder in Dropbox (ADR 030, ADR 031, ADR 034).
public final class DropboxCloud: Cloud {
    /// Before the version of a file in `CloudFile.remote`.
    private static let rev = "rev:"

    private let files: DropboxFiles
    private let folder: String

    /// The folder, read and written with the Dropbox SDK. `folder` is the path of the folder
    /// in Dropbox, such as `/PetaJournal`.
    public convenience init(client: DropboxClient, folder: String) {
        self.init(files: SdkDropboxFiles(client), folder: folder)
    }

    init(files: DropboxFiles, folder: String) {
        self.files = files
        var path = folder
        while path.hasSuffix("/") { path.removeLast() }
        self.folder = path
    }

    public func changes(since cursor: String?) throws -> Changes {
        if let cursor {
            do {
                return try read(files.list(after: cursor), whole: false)
            } catch is CursorForgotten {
                // Dropbox wants the list made anew
            }
        }
        return try read(files.list(folder: folder), whole: true)
    }

    /// Dropbox answers in parts.
    private func read(_ first: Listing, whole: Bool) throws -> Changes {
        var part = first
        var entries: [Change] = []
        while true {
            entries += part.entries.compactMap(change)
            if !part.hasMore { return Changes(entries: entries, cursor: part.cursor, whole: whole) }
            part = try files.list(after: part.cursor)
        }
    }

    /// Nothing for a folder that is there: its files tell of it.
    private func change(_ entry: DropboxEntry) -> Change? {
        switch entry {
        case .file(let file):
            return path(file.path).map { .file(version($0, file)) }
        case .deleted(let shown):
            return path(shown).map { .gone(path: $0) }
        case .folder:
            return nil
        }
    }

    /// The path inside the folder. The path as it is shown keeps the case of the name, which
    /// names the page. `nil` for the folder itself, and for an entry without a path.
    private func path(_ shown: String?) -> String? {
        guard let shown, shown.count > folder.count else { return nil }
        var path = String(shown.dropFirst(folder.count))
        while path.hasPrefix("/") { path.removeFirst() }
        return path.isEmpty ? nil : path
    }

    private func version(_ path: String, _ file: DropboxFile) -> CloudFile {
        CloudFile(path: path, contentHash: file.contentHash ?? "", remote: DropboxCloud.rev + file.rev)
    }

    public func download(_ file: CloudFile, to place: URL) throws {
        try files.download(rev: String(file.remote.dropFirst(DropboxCloud.rev.count)), to: place)
    }

    public func upload(path: String, from file: URL, over: String?) throws -> CloudFile {
        // The Dropbox app of the desk gives its copy this time, as the file has it here.
        // Dropbox keeps no parts of a second
        let changed = try FileManager.default.attributesOfItem(atPath: file.path)[.modificationDate] as? Date
        let seconds = changed.map { Date(timeIntervalSince1970: $0.timeIntervalSince1970.rounded(.down)) }
        let replaced = over.map { String($0.dropFirst(DropboxCloud.rev.count)) }
        do {
            return version(path, try files.upload(path: "\(folder)/\(path)", from: file, over: replaced, modified: seconds))
        } catch is UploadConflict {
            throw ChangedInCloud(path: path)
        }
    }

    public func contentHash(of file: URL) -> String? {
        dropboxContentHash(of: file)
    }
}
