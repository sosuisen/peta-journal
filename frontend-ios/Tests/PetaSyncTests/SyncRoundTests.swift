import Foundation
import PetaSync
import XCTest

final class SyncRoundTests: XCTestCase {
    private static let notes = "docs/notes.md"

    private var cloud: FakeCloud!
    private var folders: TestFolders!
    /// What the core does with the files in a round, in place of the core.
    private var takeIn: () throws -> Void = {}
    private var round: SyncRound!

    override func setUpWithError() throws {
        cloud = FakeCloud()
        folders = try TestFolders()
        let record = folders.record
        let downloader = Downloader(cloud: cloud, folder: folders.folder, cursorFile: folders.cursor, record: record)
        let uploader = Uploader(cloud: cloud, folder: folders.folder, record: record)
        round = SyncRound(downloader: downloader, takeIn: { [unowned self] in try self.takeIn() }, uploader: uploader)
        cloud.answers[""] = Changes(entries: [cloud.file(Self.notes, "- a")], cursor: "c1", whole: true)
    }

    private func here() -> String {
        folders.here()[Self.notes] ?? ""
    }

    private func write(_ text: String) throws {
        try folders.write(Self.notes, text)
    }

    /// The desk changes the file in the cloud, and the cloud tells of it after `since`.
    private func deskWrites(_ text: String, since: String, next: String) {
        cloud.answers[since] = Changes(entries: [cloud.file(Self.notes, text)], cursor: next, whole: false)
    }

    // 1回の同期は、ダウンロード、コアの取り込みと書き出し、アップロードの順に進む。
    func testARoundDownloadsThenLetsTheCoreWorkThenUploads() throws {
        takeIn = { try self.write(self.here() + " and what the phone wrote") }

        try round.run(onDownload: { _, _ in }, onUpload: { _, _ in })

        XCTAssertEqual(cloud.text(Self.notes), "- a and what the phone wrote")
    }

    // アップロードの前にクラウドのファイルが変わったら、ダウンロードからやり直す。クラウドの変更は消えない。
    func testARoundStartsAgainWhenTheCloudChangedBeforeTheUpload() throws {
        var rounds = 0
        takeIn = {
            rounds += 1
            if rounds == 1 { self.deskWrites("- desk", since: "c1", next: "c2") }
            try self.write(self.here() + " + phone")
        }

        try round.run(onDownload: { _, _ in }, onUpload: { _, _ in })

        XCTAssertEqual(cloud.askedWith, [nil, "c1"])
        XCTAssertEqual(cloud.text(Self.notes), "- desk + phone")
    }

    // クラウドが3回続けて断ったら、やめてエラーを出す。クラウドのファイルは上書きしない。
    func testARoundGivesUpAfterTheCloudRefusedThreeTimes() throws {
        var rounds = 0
        takeIn = {
            rounds += 1
            self.deskWrites("- desk \(rounds)", since: rounds == 1 ? "c1" : "c\(rounds)", next: "c\(rounds + 1)")
            try self.write("- phone")
        }

        XCTAssertThrowsError(try round.run(onDownload: { _, _ in }, onUpload: { _, _ in })) {
            XCTAssertTrue($0 is ChangedInCloud)
        }

        XCTAssertEqual(rounds, 3)
        XCTAssertEqual(cloud.text(Self.notes), "- desk 3")
    }

    // 進み具合は、ダウンロードとアップロードで別々に伝わる。
    func testTheProgressOfTheDownloadAndOfTheUploadAreToldApart() throws {
        takeIn = { try self.write("- phone") }
        let lock = NSLock()
        var told: [String] = []

        try round.run(
            onDownload: { done, total in lock.withLock { told.append("down \(done)/\(total)") } },
            onUpload: { done, total in lock.withLock { told.append("up \(done)/\(total)") } })

        XCTAssertEqual(told, ["down 0/1", "down 1/1", "up 0/1", "up 1/1"])
    }
}
