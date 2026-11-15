import Foundation
import PetaSync
import XCTest

final class UploaderTests: XCTestCase {
    private var cloud: FakeCloud!
    private var folders: TestFolders!
    private var downloader: Downloader!
    private var uploader: Uploader!

    override func setUpWithError() throws {
        cloud = FakeCloud()
        folders = try TestFolders()
        open()
    }

    /// Makes the parts of the sync, as the app does when it opens.
    private func open() {
        let record = folders.record
        downloader = Downloader(cloud: cloud, folder: folders.folder, cursorFile: folders.cursor, record: record)
        uploader = Uploader(cloud: cloud, folder: folders.folder, record: record)
    }

    /// Downloads the files, as the first sync of the app does.
    private func download(_ files: KeyValuePairs<String, String>) throws {
        cloud.answers[""] = Changes(entries: files.map { cloud.file($0.key, $0.value) }, cursor: "c1", whole: true)
        try downloader.run { _, _ in }
    }

    /// Downloads what changed in the cloud after the cursor `since`.
    private func download(since: String, next: String, whole: Bool = false, _ changes: Change...) throws {
        cloud.answers[since] = Changes(entries: changes, cursor: next, whole: whole)
        try downloader.run { _, _ in }
    }

    private func upload() throws {
        try uploader.run { _, _ in }
    }

    /// The paths of the files under the Markdown folder.
    private func here() -> [String] {
        folders.here().keys.sorted()
    }

    // アプリで変わったページのファイルは、クラウドへ上がる。
    func testTheFileOfAPageThatChangedInTheAppIsUploaded() throws {
        try download(["docs/notes.md": "- a", "docs/other.md": "- b"])
        try folders.write("docs/notes.md", "- a2")

        try upload()

        XCTAssertEqual(cloud.uploaded, ["docs/notes.md"])
        XCTAssertEqual(cloud.text("docs/notes.md"), "- a2")
    }

    // アプリで作ったページのファイルは、新しいファイルとして上がる。
    func testTheFileOfAPageThatWasMadeInTheAppIsUploadedAsANewFile() throws {
        try download(["docs/notes.md": "- a"])
        try folders.write("journals/2026-11-12.md", "- today")

        try upload()

        XCTAssertEqual(cloud.text("journals/2026-11-12.md"), "- today")
    }

    // 上げたファイルは、変わらないかぎり、次の回に上がらない。
    func testAnUploadedFileIsNotUploadedAgainWhileItStaysTheSame() throws {
        try download(["docs/notes.md": "- a"])
        try folders.write("docs/notes.md", "- a2")
        try folders.write("docs/new.md", "- new")
        try upload()
        cloud.uploaded = []

        try upload()
        try folders.write("docs/notes.md", "- a3")
        try upload()

        XCTAssertEqual(cloud.uploaded, ["docs/notes.md"])
        XCTAssertEqual(cloud.text("docs/notes.md"), "- a3")
    }

    // Markdownでないファイルは、変わっていても上がらない。
    func testAFileThatIsNotMarkdownIsNotUploaded() throws {
        try download(["assets/picture.png": "png"])
        try folders.write("assets/picture.png", "other")
        try folders.write("assets/new.png", "new")

        try upload()

        XCTAssertEqual(cloud.uploaded, [])
    }

    // アプリで変わったsettings/properties.jsonは、クラウドへ上がる。この端末で足したプロパティの定義を、ほかの端末に届けるためである（ADR 046）。
    func testTheFileOfThePropertyDefinitionsThatChangedInTheAppIsUploaded() throws {
        try download(["settings/properties.json": "{}"])
        try folders.write("settings/properties.json", "{\"definitions\": {}}")
        try folders.write("settings/other.json", "{}")

        try upload()

        XCTAssertEqual(cloud.uploaded, ["settings/properties.json"])
        XCTAssertEqual(cloud.text("settings/properties.json"), "{\"definitions\": {}}")
    }

    // クラウドのファイルが、アプリの知らない版になっていたら、上書きせずにエラーが出る。
    func testAFileThatChangedInTheCloudMeanwhileIsNotWrittenOver() throws {
        try download(["docs/notes.md": "- a"])
        try folders.write("docs/notes.md", "- from the phone")
        cloud.file("docs/notes.md", "- from the desk")

        XCTAssertThrowsError(try upload()) { XCTAssertTrue($0 is ChangedInCloud) }

        XCTAssertEqual(cloud.text("docs/notes.md"), "- from the desk")
    }

    // アプリで作ったページと同じ名前のファイルがクラウドにできていたら、上書きせずにエラーが出る。
    func testANewFileDoesNotWriteOverAFileThatAppearedInTheCloud() throws {
        try download(["docs/notes.md": "- a"])
        try folders.write("docs/new.md", "- from the phone")
        cloud.file("docs/New.md", "- from the desk")

        XCTAssertThrowsError(try upload()) { XCTAssertTrue($0 is ChangedInCloud) }

        XCTAssertEqual(cloud.text("docs/new.md"), "- from the desk")
    }

    // 途中で失敗しても、それまでに上げたファイルは、次の回に上がらない。
    func testTheFilesUploadedBeforeAFailureAreNotUploadedAgain() throws {
        try download(["docs/a.md": "- a", "docs/b.md": "- b"])
        try folders.write("docs/a.md", "- a2")
        try folders.write("docs/b.md", "- b2")
        cloud.broken = "docs/b.md"
        XCTAssertThrowsError(try upload()) { XCTAssertTrue($0 is NetworkGone) }
        cloud.broken = nil
        cloud.uploaded = []

        try upload()

        XCTAssertEqual(cloud.uploaded, ["docs/b.md"])
    }

    // アプリを開き直しても、クラウドと同じ中身のファイルは上がらない。
    func testNothingIsUploadedAfterTheAppOpensAgain() throws {
        try download(["docs/notes.md": "- a"])
        try folders.write("docs/new.md", "- new")
        try upload()
        cloud.uploaded = []

        open()
        try upload()

        XCTAssertEqual(cloud.uploaded, [])
    }

    // クラウドで消えたファイルと同じ名前のページをアプリで作ると、新しいファイルとして上がる。
    func testAPageMadeUnderTheNameOfAFileThatHasGoneIsUploadedAsANewFile() throws {
        try download(["docs/notes.md": "- a", "docs/work/plan.md": "- b"])
        try download(since: "c1", next: "c2", cloud.gone("docs/Notes.md"), cloud.gone("docs/work"))
        try folders.write("docs/notes.md", "- again")
        try folders.write("docs/work/plan.md", "- again")

        try upload()

        XCTAssertEqual(cloud.uploaded, ["docs/notes.md", "docs/work/plan.md"])
    }

    // クラウドが全部を答えたときも、そこにないファイルと同じ名前のページは、新しいファイルとして上がる。
    func testAPageMadeUnderTheNameOfAFileThatAWholeListLacksIsUploadedAsANewFile() throws {
        try download(["docs/notes.md": "- a", "docs/other.md": "- b"])
        cloud.gone("docs/notes.md")
        try download(since: "c1", next: "c2", whole: true, cloud.file("docs/other.md", "- b"))
        try folders.write("docs/notes.md", "- again")

        try upload()

        XCTAssertEqual(cloud.uploaded, ["docs/notes.md"])
    }

    // クラウドで消えたファイルでも、アプリで変えてまだ上げていないものは消えない。新しいファイルとして上がる。
    func testAFileThatChangedHereStaysWhenTheCloudLostItAndIsUploadedAsANewFile() throws {
        try download(["docs/notes.md": "- a", "docs/other.md": "- b"])
        try folders.write("docs/notes.md", "- from the phone")

        try download(since: "c1", next: "c2", cloud.gone("docs/Notes.md"), cloud.gone("docs/other.md"))
        try upload()

        XCTAssertEqual(here(), ["docs/notes.md"])
        XCTAssertEqual(cloud.text("docs/notes.md"), "- from the phone")
    }

    // クラウドでフォルダーごと消えたときも、その中の、アプリで変えたファイルと作ったファイルは残る。
    func testTheFilesThatChangedHereStayWhenTheCloudLostTheirFolder() throws {
        try download(["docs/work/plan.md": "- a", "docs/work/old.md": "- b", "docs/work/deep/old.md": "- c"])
        try folders.write("docs/work/plan.md", "- from the phone")
        try folders.write("docs/work/new.md", "- made on the phone")

        try download(since: "c1", next: "c2", cloud.gone("docs/work"))

        XCTAssertEqual(here(), ["docs/work/new.md", "docs/work/plan.md"])
        XCTAssertFalse(FileManager.default.fileExists(atPath: folders.root.appendingPathComponent("docs/work/deep").path))
    }

    // クラウドが全部を答えたときも、そこにないファイルのうち、アプリで変えたものと作ったものは残る。
    func testTheFilesThatChangedHereStayWhenAWholeListLacksThem() throws {
        try download(["docs/notes.md": "- a", "docs/old.md": "- b", "docs/other.md": "- c"])
        try folders.write("docs/notes.md", "- from the phone")
        try folders.write("docs/new.md", "- made on the phone")
        cloud.gone("docs/notes.md")
        cloud.gone("docs/old.md")

        try download(since: "c1", next: "c2", whole: true, cloud.file("docs/other.md", "- c"))
        try upload()

        XCTAssertEqual(here(), ["docs/new.md", "docs/notes.md", "docs/other.md"])
        XCTAssertEqual(cloud.uploaded, ["docs/new.md", "docs/notes.md"])
    }

    // 進み具合は、最初に0個、そのあと1個上げるごとに、全体の数と一緒に伝わる。
    func testTheProgressCountsTheFilesToUploadFromZeroToAll() throws {
        try download(["docs/a.md": "- a", "docs/b.md": "- b", "docs/c.md": "- c"])
        try folders.write("docs/a.md", "- a2")
        try folders.write("docs/b.md", "- b2")
        var told: [[Int]] = []

        try uploader.run { done, total in told.append([done, total]) }

        XCTAssertEqual(told, [[0, 2], [1, 2], [2, 2]])
    }
}
