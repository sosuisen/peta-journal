import Foundation
import PetaBorder
import SwiftyDropbox

/// Dropbox itself, through the Dropbox SDK, for the sync of the core (ADR 054). It only passes
/// each call on, and turns the types of the SDK into the types of the core. No test runs it:
/// the rules of the sync are tested in Rust, and the reading of the failures in
/// `DropboxFailures`.
final class SdkDropbox: PetaDropboxSdk, @unchecked Sendable {
    private let files: FilesRoutes
    /// Where the SDK answers. Each call waits here for its answer, and is on no main thread.
    private let answers = DispatchQueue(label: "peta-journal.dropbox", attributes: .concurrent)

    init(_ client: DropboxClient) {
        files = client.files
    }

    func list(folder: String) throws -> PetaListing {
        try failing {
            listing(try wait { done in
                files.listFolder(path: folder, recursive: true).response(queue: answers, completionHandler: done)
            })
        }
    }

    func listAfter(cursor: String) throws -> PetaListing {
        try failing {
            do {
                return listing(try wait { done in
                    files.listFolderContinue(cursor: cursor).response(queue: answers, completionHandler: done)
                })
            } catch CallError<Files.ListFolderContinueError>.routeError(let why, _, _, _) {
                if case .reset = why.unboxed { throw PetaDropboxError.cursorForgotten }
                throw PetaDropboxError.other(why: "\(why.unboxed)")
            }
        }
    }

    private func listing(_ result: Files.ListFolderResult) -> PetaListing {
        PetaListing(entries: result.entries.map(entry), cursor: result.cursor, hasMore: result.hasMore)
    }

    private func entry(_ metadata: Files.Metadata) -> PetaDropboxEntry {
        if let file = metadata as? Files.FileMetadata { return .file(file: self.file(file)) }
        if metadata is Files.DeletedMetadata { return .deleted(path: metadata.pathDisplay) }
        return .folder(path: metadata.pathDisplay)
    }

    private func file(_ metadata: Files.FileMetadata) -> PetaDropboxFile {
        PetaDropboxFile(path: metadata.pathDisplay, contentHash: metadata.contentHash, rev: metadata.rev)
    }

    func download(rev: String, to place: String) throws {
        _ = try failing {
            try wait { (done: @escaping ((Files.FileMetadata, URL)?, CallError<Files.DownloadError>?) -> Void) in
                // Dropbox takes a version in place of a path, after this word
                files.download(path: "rev:\(rev)", destination: URL(fileURLWithPath: place))
                    .response(queue: answers, completionHandler: done)
            }
        }
    }

    func upload(path: String, from file: String, over: String?, modifiedMs: Int64) throws -> PetaDropboxFile {
        // Dropbox takes the content only over the version it is told, or where no file is
        let mode: Files.WriteMode = over.map { .update($0) } ?? .add
        let modified = Date(timeIntervalSince1970: Double(modifiedMs) / 1000)
        return try failing {
            do {
                return self.file(try wait { done in
                    files.upload(
                        path: path, mode: mode, autorename: false, clientModified: modified,
                        input: URL(fileURLWithPath: file)
                    )
                    .response(queue: answers, completionHandler: done)
                })
            } catch CallError<Files.UploadError>.routeError(let why, _, _, _) {
                if case .path(let failed) = why.unboxed, case .conflict = failed.reason {
                    throw PetaDropboxError.uploadConflict
                }
                throw PetaDropboxError.other(why: "\(why.unboxed)")
            }
        }
    }

    /// Runs a call. What the SDK throws becomes a failure that the core knows.
    private func failing<Answer>(_ call: () throws -> Answer) throws -> Answer {
        do {
            return try call()
        } catch {
            throw dropboxFailure(error)
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
        throw PetaDropboxError.other(why: "Dropbox gave no answer")
    }
}
