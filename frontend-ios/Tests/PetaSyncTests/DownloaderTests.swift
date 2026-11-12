import Foundation
import PetaSync
import XCTest

final class DownloaderTests: XCTestCase {
    private var cloud: FakeCloud!
    private var folders: TestFolders!
    private var downloader: Downloader!

    override func setUpWithError() throws {
        cloud = FakeCloud()
        folders = try TestFolders()
        downloader = Downloader(
            cloud: cloud, folder: folders.folder, cursorFile: folders.cursor, record: folders.record)
    }

    private func download() throws {
        try downloader.run { _, _ in }
    }

    // 最初の回は、クラウドのフォルダーのMarkdownをすべて取り、同じパスに置く。
    func testTheFirstRunDownloadsEveryMarkdownFileToItsPath() throws {
        cloud.answers[""] = Changes(
            entries: [cloud.file("docs/notes.md", "- a"), cloud.file("journals/2026/2026-11-12.md", "- b")],
            cursor: "c1", whole: true)

        try download()

        XCTAssertEqual(folders.here(), ["docs/notes.md": "- a", "journals/2026/2026-11-12.md": "- b"])
    }

    // assetsのフォルダーのファイルは、Markdownでなくても取る。ページの画像である。
    func testAFileOfTheAssetsFolderIsDownloaded() throws {
        cloud.answers[""] = Changes(
            entries: [cloud.file("Assets/Picture.PNG", "png"), cloud.file("docs/Notes.MD", "- a")],
            cursor: "c1", whole: true)

        try download()

        XCTAssertEqual(folders.here(), ["Assets/Picture.PNG": "png", "docs/Notes.MD": "- a"])
    }

    // assetsの外にある、Markdownでないファイルは取らない。
    func testAFileThatIsNeitherMarkdownNorAnAssetIsNotDownloaded() throws {
        cloud.answers[""] = Changes(
            entries: [
                cloud.file("docs/picture.png", "png"), cloud.file(".obsidian/app.json", "{}"),
                cloud.file("assets.png", "png"),
            ],
            cursor: "c1", whole: true)

        try download()

        XCTAssertEqual(folders.here(), [:])
    }

    // 次の回は、前の回のカーソルから後の変更だけを尋ね、変わったファイルだけを取る。
    func testTheNextRunAsksForTheChangesSinceTheLastAndDownloadsOnlyThose() throws {
        cloud.answers[""] = Changes(
            entries: [cloud.file("docs/a.md", "- a"), cloud.file("docs/b.md", "- b")], cursor: "c1", whole: true)
        try download()
        cloud.answers["c1"] = Changes(entries: [cloud.file("docs/b.md", "- b2")], cursor: "c2", whole: false)
        cloud.downloaded = []

        try download()

        XCTAssertEqual(cloud.askedWith, [nil, "c1"])
        XCTAssertEqual(cloud.downloaded, ["docs/b.md"])
        XCTAssertEqual(folders.here(), ["docs/a.md": "- a", "docs/b.md": "- b2"])
    }

    // クラウドで消えたファイルは、アプリのフォルダーからも消える。
    func testAFileThatHasGoneFromTheCloudIsDeletedHere() throws {
        cloud.answers[""] = Changes(
            entries: [cloud.file("docs/a.md", "- a"), cloud.file("docs/b.md", "- b")], cursor: "c1", whole: true)
        try download()
        cloud.answers["c1"] = Changes(entries: [.gone(path: "docs/a.md")], cursor: "c2", whole: false)

        try download()

        XCTAssertEqual(folders.here(), ["docs/b.md": "- b"])
    }

    // クラウドで消えたフォルダーは、中のファイルごと消える。同じ回でその後に来たファイルは残る。
    func testAFolderThatHasGoneTakesItsFilesWithItButNotThoseThatCameAfter() throws {
        cloud.answers[""] = Changes(entries: [cloud.file("docs/work/old.md", "- old")], cursor: "c1", whole: true)
        try download()
        cloud.answers["c1"] = Changes(
            entries: [
                cloud.file("docs/work/lost.md", "- lost"), .gone(path: "docs/work"),
                cloud.file("docs/work/new.md", "- new"),
            ],
            cursor: "c2", whole: false)

        try download()

        XCTAssertEqual(folders.here(), ["docs/work/new.md": "- new"])
    }

    // 名前の大文字と小文字だけが変わったファイルは、新しい名前だけで残る。
    func testAFileWhoseNameChangedOnlyInCaseStaysUnderTheNewNameOnly() throws {
        cloud.answers[""] = Changes(entries: [cloud.file("docs/notes.md", "- a")], cursor: "c1", whole: true)
        try download()
        cloud.answers["c1"] = Changes(entries: [cloud.file("Docs/Notes.md", "- a2")], cursor: "c2", whole: false)

        try download()

        XCTAssertEqual(folders.here(), ["docs/Notes.md": "- a2"])
    }

    // 大文字と小文字が違うパスで消えたと知らされても、そのファイルは消える。
    func testAFileIsDeletedByAPathInAnotherCase() throws {
        cloud.answers[""] = Changes(entries: [cloud.file("docs/Notes.md", "- a")], cursor: "c1", whole: true)
        try download()
        cloud.answers["c1"] = Changes(
            entries: [.gone(path: "docs/notes.md"), .gone(path: "docs/never-here.md")], cursor: "c2", whole: false)

        try download()

        XCTAssertEqual(folders.here(), [:])
    }

    // ダウンロードが失敗すると、エラーが出て、カーソルは進まない。次の回は同じ変更を尋ね、足りないファイルだけを取る。
    func testAfterAFailedDownloadTheNextRunAsksAgainAndDownloadsOnlyWhatIsMissing() throws {
        cloud.answers[""] = Changes(
            entries: [cloud.file("docs/a.md", "- a"), cloud.file("docs/b.md", "- b")], cursor: "c1", whole: true)
        cloud.broken = "docs/b.md"
        XCTAssertThrowsError(try download()) { XCTAssertTrue($0 is NetworkGone) }
        cloud.broken = nil
        cloud.downloaded = []

        try download()

        XCTAssertEqual(cloud.askedWith, [nil, nil])
        XCTAssertEqual(cloud.downloaded, ["docs/b.md"])
        XCTAssertEqual(folders.here(), ["docs/a.md": "- a", "docs/b.md": "- b"])
    }

    // 途中で失敗したファイルは、アプリのフォルダーに半分だけ残ることがない。
    func testADownloadThatFailedLeavesNoFile() throws {
        cloud.answers[""] = Changes(entries: [cloud.file("docs/a.md", "- a")], cursor: "c1", whole: true)
        cloud.broken = "docs/a.md"

        XCTAssertThrowsError(try download())

        XCTAssertEqual(folders.here(), [:])
    }

    // クラウドがカーソルを忘れて全部を答えたときは、そこにないMarkdownがアプリのフォルダーから消える。
    func testAWholeListDeletesTheMarkdownFilesThatAreNotInIt() throws {
        cloud.answers[""] = Changes(
            entries: [cloud.file("docs/a.md", "- a"), cloud.file("docs/b.md", "- b")], cursor: "c1", whole: true)
        try download()
        cloud.answers["c1"] = Changes(entries: [cloud.file("docs/B.md", "- b")], cursor: "c2", whole: true)
        cloud.downloaded = []

        try download()

        XCTAssertEqual(folders.here(), ["docs/b.md": "- b"])
        XCTAssertEqual(cloud.downloaded, [])
    }

    // クラウドが全部を答えたときは、そこにないassetsのファイルも消える。
    func testAWholeListDeletesTheAssetFilesThatAreNotInIt() throws {
        cloud.answers[""] = Changes(
            entries: [cloud.file("assets/a.png", "a"), cloud.file("assets/b.png", "b")], cursor: "c1", whole: true)
        try download()
        cloud.answers["c1"] = Changes(entries: [cloud.file("assets/b.png", "b")], cursor: "c2", whole: true)

        try download()

        XCTAssertEqual(folders.here(), ["assets/b.png": "b"])
    }

    // 進み具合は、最初に0個、そのあと1個取るごとに、全体の数と一緒に伝わる。取らないファイルは数えない。
    func testTheProgressCountsTheFilesToDownloadFromZeroToAll() throws {
        cloud.answers[""] = Changes(entries: [cloud.file("docs/a.md", "- a")], cursor: "c1", whole: true)
        try download()
        cloud.answers["c1"] = Changes(
            entries: [cloud.file("docs/a.md", "- a"), cloud.file("docs/b.md", "- b"), cloud.file("docs/c.md", "- c")],
            cursor: "c2", whole: false)
        var told: [[Int]] = []

        try downloader.run { done, total in told.append([done, total]) }

        XCTAssertEqual(told, [[0, 2], [1, 2], [2, 2]])
    }

    // 4MBを超えるファイルのハッシュは、4MBごとのハッシュをつないだもののハッシュである。
    func testTheContentHashOfALargeFileIsMadeOfTheHashesOfItsBlocks() throws {
        let block = 4 * 1024 * 1024
        let content = Data((0..<(block + 3)).map { UInt8($0 % 251) })
        let file = folders.dir.appendingPathComponent("large")
        try content.write(to: file)
        let first = sha256(content.prefix(block))
        let second = sha256(content.suffix(from: block))

        XCTAssertEqual(dropboxContentHash(of: file), hex(sha256(first + second)))
    }
}
