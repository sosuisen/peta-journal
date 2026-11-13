import Foundation
import PetaSync
import SwiftyDropbox

/// The Markdown folder in Dropbox, read and written with the Dropbox SDK (ADR 034).
public final class DropboxCloud: Cloud {
    /// Before the version of a file, where Dropbox takes a version in place of a path.
    private static let rev = "rev:"

    private let files: FilesRoutes
    private let folder: String
    /// Where the SDK answers. Each call waits here for its answer, and is on no main thread.
    private let answers = DispatchQueue(label: "peta-journal.dropbox", attributes: .concurrent)

    /// `folder` is the path of the folder in Dropbox, such as `/PetaJournal`.
    public init(client: DropboxClient, folder: String) {
        files = client.files
        var path = folder
        while path.hasSuffix("/") { path.removeLast() }
        self.folder = path
    }

    public func changes(since cursor: String?) throws -> Changes {
        try inWords { try list(since: cursor) }
    }

    private func list(since cursor: String?) throws -> Changes {
        if let cursor {
            do {
                return try read(next(cursor), whole: false)
            } catch CallError<Files.ListFolderContinueError>.routeError(let why, _, _, _) {
                // Dropbox forgets a cursor that is too old, and wants the list made anew
                guard case .reset = why.unboxed else { throw DropboxFailure("\(why.unboxed)") }
            }
        }
        let first = try wait { done in
            files.listFolder(path: folder, recursive: true).response(queue: answers, completionHandler: done)
        }
        return try read(first, whole: true)
    }

    private func next(_ cursor: String) throws -> Files.ListFolderResult {
        try wait { done in files.listFolderContinue(cursor: cursor).response(queue: answers, completionHandler: done) }
    }

    /// Dropbox answers in parts.
    private func read(_ first: Files.ListFolderResult, whole: Bool) throws -> Changes {
        var result = first
        var entries: [Change] = []
        while true {
            entries += result.entries.compactMap(change)
            if !result.hasMore { return Changes(entries: entries, cursor: result.cursor, whole: whole) }
            result = try next(result.cursor)
        }
    }

    /// Nothing for a folder that is there: its files tell of it.
    private func change(_ entry: Files.Metadata) -> Change? {
        // The path as it is shown keeps the case of the name, which names the page
        guard let shown = entry.pathDisplay, shown.count > folder.count else { return nil }
        var path = String(shown.dropFirst(folder.count))
        while path.hasPrefix("/") { path.removeFirst() }
        if path.isEmpty { return nil }
        if let file = entry as? Files.FileMetadata { return .file(version(path, file)) }
        if entry is Files.DeletedMetadata { return .gone(path: path) }
        return nil
    }

    private func version(_ path: String, _ file: Files.FileMetadata) -> CloudFile {
        CloudFile(path: path, contentHash: file.contentHash ?? "", remote: DropboxCloud.rev + file.rev)
    }

    public func download(_ file: CloudFile, to place: URL) throws {
        _ = try inWords {
            try wait { (done: @escaping ((Files.FileMetadata, URL)?, CallError<Files.DownloadError>?) -> Void) in
                files.download(path: file.remote, destination: place).response(queue: answers, completionHandler: done)
            }
        }
    }

    public func upload(path: String, from file: URL, over: String?) throws -> CloudFile {
        try inWords { try put(path, file, over) }
    }

    private func put(_ path: String, _ file: URL, _ over: String?) throws -> CloudFile {
        // Dropbox takes the content only over the version it is told, or where no file is
        let mode: Files.WriteMode = over.map { .update(String($0.dropFirst(DropboxCloud.rev.count))) } ?? .add
        // The Dropbox app of the desk gives its copy this time, as the file has it here
        let changed = try FileManager.default.attributesOfItem(atPath: file.path)[.modificationDate] as? Date
        let seconds = changed.map { Date(timeIntervalSince1970: $0.timeIntervalSince1970.rounded(.down)) }
        do {
            let now = try wait { done in
                files.upload(path: "\(folder)/\(path)", mode: mode, autorename: false, clientModified: seconds, input: file)
                    .response(queue: answers, completionHandler: done)
            }
            return version(path, now)
        } catch CallError<Files.UploadError>.routeError(let why, _, _, _) {
            if case .path(let failed) = why.unboxed, case .conflict = failed.reason { throw ChangedInCloud(path: path) }
            throw DropboxFailure("\(why.unboxed)")
        }
    }

    /// Runs a call. What the SDK throws is long and made for a log: the screen gets its words.
    private func inWords<Answer>(_ call: () throws -> Answer) throws -> Answer {
        do {
            return try call()
        } catch let known as ChangedInCloud {
            throw known
        } catch let known as DropboxFailure {
            throw known
        } catch {
            throw DropboxFailure(String(describing: error))
        }
    }

    /// Asks Dropbox and waits for the answer.
    private func wait<Answer, Failure: Error>(
        _ ask: (@escaping (Answer?, Failure?) -> Void) -> Void
    ) throws -> Answer {
        let done = DispatchSemaphore(value: 0)
        var answer: Answer?
        var failure: Failure?
        ask {
            answer = $0
            failure = $1
            done.signal()
        }
        done.wait()
        if let answer { return answer }
        if let failure { throw failure }
        throw DropboxFailure("Dropbox gave no answer")
    }
}

/// Why Dropbox did not do what it was asked, in words for the screen.
public struct DropboxFailure: LocalizedError {
    public let errorDescription: String?

    init(_ words: String) {
        errorDescription = words
    }
}
