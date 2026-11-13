import Foundation

/// The calls of Dropbox that `DropboxCloud` makes, without the types of the Dropbox SDK. A
/// test puts its own Dropbox here. The app puts `SdkDropboxFiles` here.
///
/// Every call waits for Dropbox, so none is for the main thread.
protocol DropboxFiles {
    /// The first part of all that is in the folder at the path, and in the folders in it.
    func list(folder: String) throws -> Listing

    /// The part after `cursor`: the next part of a list, or what changed since a list ended.
    /// Throws `CursorForgotten` when Dropbox no longer knows the cursor.
    func list(after cursor: String) throws -> Listing

    /// Writes the content of the version `rev` of a file to `place`, where no file is yet.
    func download(rev: String, to place: URL) throws

    /// Puts the content of `file` at the path. `over` is the version that this content
    /// replaces, and it is `nil` for a file that Dropbox has not. Dropbox gives the file no
    /// other name. `modified` is the time of the last change, as the file has it here.
    ///
    /// Throws `UploadConflict` when Dropbox has another version, or a file where none was
    /// expected.
    func upload(path: String, from file: URL, over: String?, modified: Date?) throws -> DropboxFile
}

/// One part of a list. `cursor` names the state after it. `hasMore` says that a part follows.
struct Listing {
    let entries: [DropboxEntry]
    let cursor: String
    let hasMore: Bool
}

/// What Dropbox tells of one path. The path is as it is shown, with the case of the names.
enum DropboxEntry: Equatable {
    case file(DropboxFile)
    case deleted(path: String?)
    case folder(path: String?)
}

/// One version of a file in Dropbox. `contentHash` is `nil` when Dropbox gave none.
struct DropboxFile: Equatable {
    let path: String?
    let contentHash: String?
    let rev: String
}

/// Dropbox forgot a cursor that was too old. It wants the list made anew.
struct CursorForgotten: Error {}

/// Dropbox did not take an upload: the file there is not the version that was to be replaced.
struct UploadConflict: Error {}
