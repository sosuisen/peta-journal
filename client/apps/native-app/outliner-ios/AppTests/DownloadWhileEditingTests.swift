import PetaBorder
import PetaTypes
import SwiftUI
import UIKit
import XCTest

@testable import PetaJournal

/// A page that a download changes while one of its items is edited (ADR 069). The model has a
/// real core under it, and the field of the item is the field of the app, in a window.
@MainActor
final class DownloadWhileEditingTests: XCTestCase {
    private static let wait: TimeInterval = 10
    private static let first = "京都で会議がありました。"

    private var root: URL!
    private var model: JournalModel!
    private var account: FakeAccount!
    private var window: UIWindow!

    override func setUp() async throws {
        root = FileManager.default.temporaryDirectory.appendingPathComponent("editing-\(UUID().uuidString)")
        account = FakeAccount()
        // Longer than a test runs: no edit syncs by itself
        model = try JournalModel(root: root, account: account, afterEdit: 600)
        let scene = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first
        window = UIWindow(windowScene: scene!)
        window.frame = CGRect(x: 0, y: 0, width: 320, height: 480)
        window.rootViewController = UIHostingController(rootView: EditedRow(shell: model.shell))
        window.makeKeyAndVisible()
    }

    override func tearDown() async throws {
        window.isHidden = true
        window = nil
        model.shell.close()
        model = nil
        try? FileManager.default.removeItem(at: root)
    }

    /// The edited item of the outline, in the field of the app.
    private struct EditedRow: View {
        @ObservedObject var shell: Shell

        var body: some View {
            if let row = shell.view.outline.rows.first(where: { $0.isEditing }) {
                ItemEditor(row: row, shell: shell)
            }
        }
    }

    /// Lets the main queue run until `holds` is true. The sync and the screen tell on it.
    private func waitUntil(_ what: String, _ holds: () -> Bool) {
        let end = Date().addingTimeInterval(DownloadWhileEditingTests.wait)
        while !holds() {
            XCTAssertLessThan(Date(), end, "waited in vain: \(what)")
            if Date() >= end { return }
            RunLoop.main.run(until: Date().addingTimeInterval(0.02))
        }
    }

    /// Syncs, and waits until the sync rests.
    private func sync() {
        model.sync(inFront: true)
        waitUntil("the sync rests") { model.cloud?.state == .idle }
    }

    private var file: URL { root.appendingPathComponent("markdown/docs/trip.md") }

    /// The page `trip` of the cloud, open, with the edit of its item. The file in the app has
    /// the ID of the item then: another device that changes the item keeps it.
    private func editTheTrip() throws -> String {
        sync()
        model.shell.follow(.page(url: "/pages/trip"))
        // Not the count of the rows: the journal of today, shown before, has one item too
        waitUntil("the page shows its item") {
            model.shell.view.outline.rows.map(\.item.body) == [DownloadWhileEditingTests.first]
        }
        waitUntil("the file has the ID of the item") {
            (try? String(contentsOf: file, encoding: .utf8))?.contains("<!--") == true
        }
        let id = model.shell.view.outline.rows[0].item.id
        model.shell.edit(id)
        waitUntil("the field is open") { field?.text == DownloadWhileEditingTests.first }
        return id
    }

    /// The field of the app in the window, while an item is edited.
    private var field: ItemEditor.Field? { find(in: window) }

    private func find(in view: UIView) -> ItemEditor.Field? {
        if let field = view as? ItemEditor.Field { return field }
        return view.subviews.lazy.compactMap { self.find(in: $0) }.first
    }

    /// Another device changes the text of the item in the page, keeping its ID.
    private func changedElsewhere(into text: String) throws {
        let written = try String(contentsOf: file, encoding: .utf8)
        account.page.change(to: written.replacingOccurrences(of: DownloadWhileEditingTests.first, with: text))
    }

    // 編集中の項目をほかの端末が書き換えると、こちらで打っていなければ、ダウンロードのあと入力欄にその文が出る。
    func testAChangeElsewhereToTheEditedItemShowsInItsFieldWhenNothingWasTyped() throws {
        let id = try editTheTrip()

        try changedElsewhere(into: "大阪で会議がありました。")
        sync()

        waitUntil("the field shows the change") { field?.text == "大阪で会議がありました。" }
        XCTAssertTrue(model.shell.view.outline.rows.first { $0.item.id == id }?.isEditing == true)
    }

    // 編集中の項目に打った文は、ほかの端末がその項目を書き換えたページをダウンロードしても残る。
    func testTextTypedIntoTheEditedItemStaysWhenItChangedElsewhere() throws {
        let id = try editTheTrip()
        model.shell.type(id, DownloadWhileEditingTests.first + "楽しかった。")
        waitUntil("the field shows the typing") { field?.text == DownloadWhileEditingTests.first + "楽しかった。" }

        try changedElsewhere(into: "大阪で会議がありました。")
        sync()

        XCTAssertEqual(field?.text, DownloadWhileEditingTests.first + "楽しかった。")
        XCTAssertEqual(model.shell.view.outline.rows[0].item.body, DownloadWhileEditingTests.first + "楽しかった。")
    }
}
