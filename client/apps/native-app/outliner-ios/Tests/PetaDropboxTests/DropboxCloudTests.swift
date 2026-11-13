import PetaSync
import XCTest

@testable import PetaDropbox

/// What the app makes of the answers of Dropbox. Dropbox itself is a Dropbox of the test.
final class DropboxCloudTests: XCTestCase {
    private var dir: URL!
    private var dropbox: FakeDropboxFiles!
    private var cloud: DropboxCloud!

    override func setUpWithError() throws {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("peta-dropbox-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        dropbox = FakeDropboxFiles()
        cloud = DropboxCloud(files: dropbox, folder: "/PetaJournal/")
    }

    override func tearDownWithError() throws {
        try FileManager.default.removeItem(at: dir)
    }

    private func file(_ path: String, _ rev: String = "r1") -> DropboxEntry {
        .file(DropboxFile(path: path, contentHash: "hash-\(rev)", rev: rev))
    }

    private func page(_ text: String = "- a") throws -> URL {
        let page = dir.appendingPathComponent("a.md")
        try Data(text.utf8).write(to: page)
        return page
    }

    // カーソルがなければ、フォルダーの全部の一覧を取る。パスは、フォルダーからの相対になる。
    func testWithoutACursorTheChangesAreTheWholeFolderWithPathsInsideIt() throws {
        dropbox.lists["/PetaJournal"] = Listing(entries: [file("/PetaJournal/docs/a.md")], cursor: "c1", hasMore: false)

        let changes = try cloud.changes(since: nil)

        XCTAssertTrue(changes.whole)
        XCTAssertEqual(changes.cursor, "c1")
        XCTAssertEqual(changes.entries, [.file(CloudFile(path: "docs/a.md", contentHash: "hash-r1", remote: "rev:r1"))])
    }

    // カーソルがあれば、そのあとに変わったものだけを取る。
    func testWithACursorTheChangesAreWhatCameAfterIt() throws {
        dropbox.after["c1"] = Listing(entries: [file("/PetaJournal/docs/b.md", "r2")], cursor: "c2", hasMore: false)

        let changes = try cloud.changes(since: "c1")

        XCTAssertFalse(changes.whole)
        XCTAssertEqual(changes.cursor, "c2")
        XCTAssertEqual(paths(changes), ["docs/b.md"])
    }

    // Dropboxがカーソルを忘れていれば、全部の一覧を取り直す。
    func testACursorThatDropboxForgotGivesTheWholeFolderAgain() throws {
        dropbox.forgotten.insert("old")
        dropbox.lists["/PetaJournal"] = Listing(entries: [file("/PetaJournal/docs/a.md")], cursor: "c1", hasMore: false)

        let changes = try cloud.changes(since: "old")

        XCTAssertTrue(changes.whole)
        XCTAssertEqual(paths(changes), ["docs/a.md"])
    }

    // Dropboxは一覧を何回かに分けて答える。全部をつなぎ、最後のカーソルを返す。
    func testAListThatComesInPartsIsReadToItsEnd() throws {
        dropbox.lists["/PetaJournal"] = Listing(entries: [file("/PetaJournal/docs/a.md")], cursor: "part", hasMore: true)
        dropbox.after["part"] = Listing(entries: [file("/PetaJournal/docs/b.md")], cursor: "end", hasMore: false)

        let changes = try cloud.changes(since: nil)

        XCTAssertEqual(paths(changes), ["docs/a.md", "docs/b.md"])
        XCTAssertEqual(changes.cursor, "end")
        XCTAssertTrue(changes.whole)
    }

    // パスは、Dropboxが見せるとおりの大文字と小文字を保つ。ページの名前になるからである。
    func testAPathKeepsTheCaseThatDropboxShows() throws {
        dropbox.lists["/PetaJournal"] = Listing(entries: [file("/petajournal/Docs/Trip.md")], cursor: "c1", hasMore: false)

        XCTAssertEqual(paths(try cloud.changes(since: nil)), ["Docs/Trip.md"])
    }

    // あるフォルダーの知らせは、変更にならない。フォルダーは、その中のファイルで分かる。
    func testAFolderThatIsThereIsNoChange() throws {
        let entries: [DropboxEntry] = [
            .folder(path: "/PetaJournal"), .folder(path: "/PetaJournal/docs"), file("/PetaJournal/docs/a.md"),
        ]
        dropbox.lists["/PetaJournal"] = Listing(entries: entries, cursor: "c1", hasMore: false)

        XCTAssertEqual(paths(try cloud.changes(since: nil)), ["docs/a.md"])
    }

    // 消えたファイルやフォルダーは、「なくなった」という変更になる。
    func testWhatWasDeletedInDropboxIsAChangeThatSaysGone() throws {
        dropbox.after["c1"] = Listing(entries: [.deleted(path: "/PetaJournal/docs")], cursor: "c2", hasMore: false)

        XCTAssertEqual(try cloud.changes(since: "c1").entries, [.gone(path: "docs")])
    }

    // パスやハッシュのない知らせでも、落ちない。パスがなければ捨て、ハッシュがなければ空にする。
    func testAnEntryWithoutAPathIsLeftOutAndOneWithoutAHashGetsAnEmptyOne() throws {
        let entries: [DropboxEntry] = [
            .file(DropboxFile(path: nil, contentHash: "h", rev: "r1")),
            .file(DropboxFile(path: "/PetaJournal/docs/a.md", contentHash: nil, rev: "r2")),
        ]
        dropbox.lists["/PetaJournal"] = Listing(entries: entries, cursor: "c1", hasMore: false)

        XCTAssertEqual(
            try cloud.changes(since: nil).entries,
            [.file(CloudFile(path: "docs/a.md", contentHash: "", remote: "rev:r2"))])
    }

    // ダウンロードは、一覧で見たそのバージョンを取る。
    func testADownloadTakesTheVeryVersionThatTheListShowed() throws {
        dropbox.lists["/PetaJournal"] = Listing(
            entries: [file("/PetaJournal/docs/a.md", "r7")], cursor: "c1", hasMore: false)
        dropbox.contents["r7"] = "- seventh"
        guard case .file(let seen) = try cloud.changes(since: nil).entries.first else { return XCTFail("no file") }
        let place = dir.appendingPathComponent("downloaded.md")

        try cloud.download(seen, to: place)

        XCTAssertEqual(try String(contentsOf: place, encoding: .utf8), "- seventh")
    }

    // Dropboxにないファイルは、置き換えるバージョンなしで上げる。フォルダーのパスを前に付ける。
    func testANewFileIsUploadedOverNoVersionIntoTheFolder() throws {
        let answer = try cloud.upload(path: "docs/a.md", from: try page("- new"), over: nil)

        XCTAssertEqual(dropbox.uploads.map(\.path), ["/PetaJournal/docs/a.md"])
        XCTAssertEqual(dropbox.uploads.map(\.over), [nil])
        XCTAssertEqual(dropbox.uploads.first?.text, "- new")
        XCTAssertEqual(answer, CloudFile(path: "docs/a.md", contentHash: "hash-of-upload", remote: "rev:r9"))
    }

    // Dropboxにあるファイルは、アプリが見たバージョンの上にだけ上げる。
    func testAFileThatDropboxHasIsUploadedOverTheVersionThatTheAppSaw() throws {
        _ = try cloud.upload(path: "docs/a.md", from: try page(), over: "rev:r3")

        XCTAssertEqual(dropbox.uploads.map(\.over), ["r3"])
    }

    // 上げるファイルの更新時刻は、秒までにする。Dropboxは、秒より細かい時刻を受け取らない。
    func testAnUploadTellsTheTimeOfTheLastChangeInWholeSeconds() throws {
        let page = try page()
        try FileManager.default.setAttributes(
            [.modificationDate: Date(timeIntervalSince1970: 1_700_000_000.999)], ofItemAtPath: page.path)

        _ = try cloud.upload(path: "docs/a.md", from: page, over: nil)

        XCTAssertEqual(dropbox.uploads.first?.modified, Date(timeIntervalSince1970: 1_700_000_000))
    }

    // Dropboxのファイルが別のバージョンなら、「クラウドで変わった」というエラーになる。
    func testAnUploadOverAnotherVersionFailsAsChangedInTheCloud() throws {
        dropbox.refusal = UploadConflict()

        XCTAssertThrowsError(try cloud.upload(path: "docs/a.md", from: try page(), over: "rev:r3")) {
            XCTAssertEqual($0 as? ChangedInCloud, ChangedInCloud(path: "docs/a.md"))
        }
    }

    // ほかの失敗は、そのまま伝わる。同期は、その理由を持って止まる。
    func testAnUploadThatFailsForAnotherReasonFailsWithThatReason() throws {
        dropbox.refusal = NetworkGone()

        XCTAssertThrowsError(try cloud.upload(path: "docs/a.md", from: try page(), over: nil)) {
            XCTAssertTrue($0 is NetworkGone)
        }
    }

    // ファイルのハッシュは、Dropboxのコンテンツハッシュである。
    func testTheContentHashOfAFileIsTheOneOfDropbox() throws {
        let page = try page("- a")

        XCTAssertEqual(cloud.contentHash(of: page), dropboxContentHash(of: page))
    }

    private func paths(_ changes: Changes) -> [String] {
        changes.entries.map {
            switch $0 {
            case .file(let file): return file.path
            case .gone(let path): return path
            }
        }
    }
}

private struct NetworkGone: Error {}

/// A Dropbox that answers what the test tells it, and keeps what it was asked.
private final class FakeDropboxFiles: DropboxFiles {
    /// The whole list of each folder, and the parts after each cursor.
    var lists: [String: Listing] = [:]
    var after: [String: Listing] = [:]
    /// The cursors that Dropbox has forgotten.
    var forgotten: Set<String> = []
    var contents: [String: String] = [:]
    /// What stops the next upload, if anything.
    var refusal: Error?
    var uploads: [Upload] = []

    struct Upload {
        let path: String
        let text: String
        let over: String?
        let modified: Date?
    }

    func list(folder: String) throws -> Listing {
        lists[folder]!
    }

    func list(after cursor: String) throws -> Listing {
        if forgotten.contains(cursor) { throw CursorForgotten() }
        return after[cursor]!
    }

    func download(rev: String, to place: URL) throws {
        try Data(contents[rev]!.utf8).write(to: place)
    }

    func upload(path: String, from file: URL, over: String?, modified: Date?) throws -> DropboxFile {
        if let refusal { throw refusal }
        let text = try String(contentsOf: file, encoding: .utf8)
        uploads.append(Upload(path: path, text: text, over: over, modified: modified))
        return DropboxFile(path: path, contentHash: "hash-of-upload", rev: "r9")
    }
}
