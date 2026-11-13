import Foundation
import SwiftyDropbox

/// Dropbox itself, through the Dropbox SDK. It only passes each call on, and turns the types
/// of the SDK into the types of `DropboxFiles`. No test runs it: all that can be tested
/// without Dropbox is in `DropboxCloud`.
final class SdkDropboxFiles: DropboxFiles {
    private let files: FilesRoutes
    /// Where the SDK answers. Each call waits here for its answer, and is on no main thread.
    private let answers = DispatchQueue(label: "peta-journal.dropbox", attributes: .concurrent)

    init(_ client: DropboxClient) {
        files = client.files
    }

    func list(folder: String) throws -> Listing {
        try inWords {
            listing(try wait { done in
                files.listFolder(path: folder, recursive: true).response(queue: answers, completionHandler: done)
            })
        }
    }

    func list(after cursor: String) throws -> Listing {
        try inWords {
            do {
                return listing(try wait { done in
                    files.listFolderContinue(cursor: cursor).response(queue: answers, completionHandler: done)
                })
            } catch CallError<Files.ListFolderContinueError>.routeError(let why, _, _, _) {
                if case .reset = why.unboxed { throw CursorForgotten() }
                throw DropboxFailure("\(why.unboxed)")
            }
        }
    }

    private func listing(_ result: Files.ListFolderResult) -> Listing {
        Listing(entries: result.entries.map(entry), cursor: result.cursor, hasMore: result.hasMore)
    }

    private func entry(_ metadata: Files.Metadata) -> DropboxEntry {
        if let file = metadata as? Files.FileMetadata { return .file(self.file(file)) }
        if metadata is Files.DeletedMetadata { return .deleted(path: metadata.pathDisplay) }
        return .folder(path: metadata.pathDisplay)
    }

    private func file(_ metadata: Files.FileMetadata) -> DropboxFile {
        DropboxFile(path: metadata.pathDisplay, contentHash: metadata.contentHash, rev: metadata.rev)
    }

    func download(rev: String, to place: URL) throws {
        _ = try inWords {
            try wait { (done: @escaping ((Files.FileMetadata, URL)?, CallError<Files.DownloadError>?) -> Void) in
                // Dropbox takes a version in place of a path, after this word
                files.download(path: "rev:\(rev)", destination: place).response(queue: answers, completionHandler: done)
            }
        }
    }

    func upload(path: String, from file: URL, over: String?, modified: Date?) throws -> DropboxFile {
        // Dropbox takes the content only over the version it is told, or where no file is
        let mode: Files.WriteMode = over.map { .update($0) } ?? .add
        return try inWords {
            do {
                return self.file(try wait { done in
                    files.upload(path: path, mode: mode, autorename: false, clientModified: modified, input: file)
                        .response(queue: answers, completionHandler: done)
                })
            } catch CallError<Files.UploadError>.routeError(let why, _, _, _) {
                if case .path(let failed) = why.unboxed, case .conflict = failed.reason { throw UploadConflict() }
                throw DropboxFailure("\(why.unboxed)")
            }
        }
    }

    /// Runs a call. What the SDK throws is long and made for a log: the screen gets its words.
    private func inWords<Answer>(_ call: () throws -> Answer) throws -> Answer {
        do {
            return try call()
        } catch let known as CursorForgotten {
            throw known
        } catch let known as UploadConflict {
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
