import Foundation
import PetaBorder
import PetaTypes
import XCTest

/// The parts of an item as it is shown come from the core with the row (ADR 042). How the core
/// makes them is tested in Rust; here, that they reach the app and that their links work.
@MainActor
final class RichTextTests: JournalTestCase {
    /// The parts of the item of the page `notes` with this Markdown.
    private func shown(_ markdown: String) throws -> [RichBlock] {
        try download("notes.md", markdown)
        try openNotes()
        waitUntil("the items are shown") { !self.shell.view.outline.rows.isEmpty }
        return shell.view.outline.rows.flatMap(\.bodyModel.blocks)
    }

    // 項目は、coreが作った部品として届く。Markdownの記号ではなく、飾りの付いた文字になる。
    func testAnItemReachesTheAppAsThePartsThatTheCoreMakes() throws {
        let blocks = try shown("some **bold** text\n")

        guard case .text(let paragraphs) = blocks.first else { return XCTFail("no text: \(blocks)") }
        XCTAssertEqual(paragraphs.flatMap(\.runs).map(\.text), ["some ", "bold", " text"])
        XCTAssertEqual(paragraphs.flatMap(\.runs).map(\.looks.bold), [false, true, false])
    }

    // タグのリンクは、設定のタグのフォルダーの下のページへ行く。
    func testATagLeadsToItsPageUnderTheFolderOfTheTags() throws {
        let blocks = try shown("#topic\n")

        guard case .text(let paragraphs) = blocks.first else { return XCTFail("no text: \(blocks)") }
        let tagsDir = shell.view.config.tagsDir
        XCTAssertEqual(paragraphs.flatMap(\.runs).compactMap(\.link), [.page(url: "/pages/\(tagsDir)topic")])
    }

    // リンクは、URLにしてから戻しても、同じ行き先になる。ページの中の項目への印と、符号化した文字も残る。外へのリンクは、どのスキームでもそのまま残る（#1071）。
    func testALinkIsTheSameAfterItIsMadeAURLAndBack() {
        let links: [RichLink] = [
            .web(url: "https://example.com/a?b=c#d"),
            .web(url: "mailto:someone@example.com"),
            .web(url: "obsidian://open?vault=notes"),
            .page(url: "/pages/other%20page#2020-01-01-00-00-00-AAAAAAA"),
            .file(path: "assets/Report 1.pdf"),
        ]
        for link in links {
            XCTAssertEqual(RichLinks.url(of: link).flatMap(RichLinks.link(of:)), link)
        }
    }

    // アプリのアドレスで、アプリが作っていないものは、どこへも行かない。
    func testAnAddressOfTheAppThatTheAppDidNotMakeLeadsNowhere() {
        XCTAssertNil(RichLinks.link(of: URL(string: "peta-journal://other?to=x")!))
        XCTAssertNil(RichLinks.link(of: URL(string: "peta-journal://page")!))
    }
}
