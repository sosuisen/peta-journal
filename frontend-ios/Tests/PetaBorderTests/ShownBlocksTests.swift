import Foundation
import PetaBorder
import PetaTypes
import XCTest

/// The parts of an item as it is shown, read from the AST that the core gives.
@MainActor
final class ShownBlocksTests: JournalTestCase {
    /// The parts of the first item of the page `notes` with this Markdown.
    private func shown(_ markdown: String, tagsDir: String = "tags") throws -> [ShownBlock] {
        try download("notes.md", markdown)
        try openNotes()
        let item = shell.view.outline.rows[0].item
        waitUntil("the item has its AST") { !self.shell.view.outline.rows[0].item.ast.isEmpty }
        return shownBlocks(body: item.body, ast: shell.view.outline.rows[0].item.ast, tagsDir: tagsDir)
    }

    private func html(_ block: ShownBlock?) -> String {
        if case .text(let html) = block { return html }
        return ""
    }

    // ASTのない項目は、本文がそのまま見える。
    func testAnItemWithoutAnASTShowsItsBodyAsItIs() {
        XCTAssertEqual(shownBlocks(body: "**body**", ast: "", tagsDir: ""), [.plain("**body**")])
    }

    // ASTがJSONとして読めない項目は、本文がそのまま見える。
    func testAnItemWhoseASTIsNotJSONShowsItsBodyAsItIs() {
        XCTAssertEqual(shownBlocks(body: "**body**", ast: "{", tagsDir: ""), [.plain("**body**")])
    }

    // 項目は、Markdownの記号ではなく、飾りの付いたHTMLになる。
    func testAnItemIsHTMLWithItsDecorationAndNotTheMarksOfMarkdown() throws {
        let blocks = try shown("some **bold** text\n")

        XCTAssertEqual(blocks.count, 1)
        XCTAssertTrue(html(blocks.first).contains("<strong>bold</strong>"), html(blocks.first))
        XCTAssertFalse(html(blocks.first).contains("**"))
    }

    // コードブロックは、行が分かれたまま、書かれたとおりの部品になる。フェンスの記号は入らない。
    func testACodeBlockKeepsItsLinesAsTheyWereWritten() throws {
        let blocks = try shown("before\n\n```kotlin\nval x = 1\n  val y = 2\n```\n\nafter\n")

        XCTAssertEqual(blocks.count, 3)
        XCTAssertTrue(html(blocks[0]).contains("before"))
        XCTAssertEqual(blocks[1], .code(lines: "val x = 1\n  val y = 2"))
        XCTAssertTrue(html(blocks[2]).contains("after"))
    }

    // 表は、セルごとに分かれた部品になる。セルの中の文字の飾りも付く。
    func testATableHasEachCellApart() throws {
        let blocks = try shown("| name | count |\n|---|---|\n| **apple** | 2 |\n")

        guard case .table(let rows) = blocks.first else { return XCTFail("no table: \(blocks)") }
        XCTAssertEqual(rows.map(\.header), [true, false])
        XCTAssertEqual(rows.map(\.cells.count), [2, 2])
        XCTAssertTrue(rows[0].cells[0].contains("name"))
        XCTAssertTrue(rows[1].cells[0].contains("<strong>apple</strong>"), rows[1].cells[0])
        XCTAssertTrue(rows[1].cells[1].contains("2"))
    }

    // assetsの画像は、前後の文字のあいだの、画像の部品になる。
    func testAnImageOfTheAssetsFolderStandsBetweenTheTextAroundIt() throws {
        let blocks = try shown("before ![a cat](../assets/Cat.png) after\n")

        XCTAssertEqual(blocks.count, 3)
        XCTAssertTrue(html(blocks[0]).contains("before"))
        XCTAssertEqual(blocks[1], .image(ShownImage(path: "assets/Cat.png", alt: "a cat", width: nil, height: nil)))
        XCTAssertTrue(html(blocks[2]).contains("after"))
    }

    // 画像は、ページが決めた幅と高さを持つ。決めていない辺は、ない。
    func testAnImageHasTheSidesThatThePageAsksFor() throws {
        let both = try shown("![both](../assets/cat.png#w=120&h=30)\n")
        XCTAssertEqual(both, [.image(ShownImage(path: "assets/cat.png", alt: "both", width: 120, height: 30))])
    }

    // 高さだけを決めた画像は、幅を持たない。
    func testAnImageWithOnlyAHeightHasNoWidth() throws {
        let tall = try shown("![tall](../assets/cat.png#h=60)\n")
        XCTAssertEqual(tall, [.image(ShownImage(path: "assets/cat.png", alt: "tall", width: nil, height: 60))])
    }

    // Web上の画像は、画像の部品にならない。代わりの文字のリンクになる。
    func testAnImageOfTheWebIsALinkAndNotAPicture() throws {
        let blocks = try shown("see ![a bird](https://example.com/bird.png) here\n")

        XCTAssertEqual(blocks.count, 1)
        let text = html(blocks.first)
        XCTAssertTrue(text.contains("href=\"https://example.com/bird.png\""), text)
        XCTAssertTrue(text.contains(">a bird</a>"), text)
        XCTAssertFalse(text.contains("<img"))
    }

    // リストの中のコードブロックも、行が分かれたまま残る。リストの項目は、番号と一緒に残る。
    func testACodeBlockInsideAListKeepsItsLinesAndTheEntriesKeepTheirNumbers() throws {
        let blocks = try shown("1. first\n\n   ```\n   a\n     b\n   ```\n2. second\n")

        guard case .entries(let entries) = blocks.first else { return XCTFail("no entries: \(blocks)") }
        XCTAssertEqual(entries.map(\.mark), ["1.", "2."])
        XCTAssertEqual(entries[0].blocks.count, 2)
        XCTAssertTrue(html(entries[0].blocks[0]).contains("first"))
        XCTAssertEqual(entries[0].blocks[1], .code(lines: "a\n  b"))
        XCTAssertTrue(html(entries[1].blocks.first).contains("second"))
    }

    // 引用の中の表も、セルごとに分かれて残る。
    func testATableInsideAQuoteHasEachCellApart() throws {
        let blocks = try shown("> said:\n>\n> | name | count |\n> |---|---|\n> | apple | 2 |\n")

        guard case .quote(let inside) = blocks.first else { return XCTFail("no quote: \(blocks)") }
        XCTAssertTrue(html(inside.first).contains("said:"))
        guard case .table(let rows) = inside.last else { return XCTFail("no table: \(inside)") }
        XCTAssertEqual(rows.count, 2)
    }

    // リストの中のリストの画像も、画像の部品になる。
    func testAnImageInsideAListInsideAListIsAPicture() throws {
        let blocks = try shown("- outer\n  - look ![a cat](../assets/cat.png)\n- next\n")

        guard case .entries(let outer) = blocks.first else { return XCTFail("no entries: \(blocks)") }
        XCTAssertEqual(outer.map(\.mark), ["•", "•"])
        guard case .entries(let inner) = outer[0].blocks.last else { return XCTFail("no inner list: \(outer[0])") }
        XCTAssertEqual(inner[0].blocks.last, .image(ShownImage(path: "assets/cat.png", alt: "a cat", width: nil, height: nil)))
    }

    // 自分の部品を持たないリストは、分けられずに1つの文字のままである。
    func testAListThatHoldsNoPartOfItsOwnStaysOneText() throws {
        let blocks = try shown("- one\n- two\n")

        XCTAssertEqual(blocks.count, 1)
        XCTAssertTrue(html(blocks.first).contains("<li>"), html(blocks.first))
    }

    // リストの中のタスクの項目は、チェックボックスの印を持つ。
    func testTheEntriesOfATaskListHaveACheckbox() throws {
        let blocks = try shown("- [ ] open\n\n  ```\n  code\n  ```\n- [x] done\n")

        guard case .entries(let entries) = blocks.first else { return XCTFail("no entries: \(blocks)") }
        XCTAssertEqual(entries.map(\.mark), ["☐", "☑"])
    }

    // ページへのリンクとタグは、そのページのURLへのリンクになる。タグのページは、タグのフォルダーの下にある。
    func testALinkToAPageAndATagAreLinksToTheURLOfTheirPage() throws {
        let text = html(try shown("[[other page]] and #topic\n", tagsDir: "tags").first)

        XCTAssertTrue(text.contains("href=\"/pages/other%20page\""), text)
        XCTAssertTrue(text.contains("href=\"/pages/tags/topic\""), text)
    }

    // .mdへの相対リンクは、そのページのURLへのリンクになる。
    func testARelativeLinkToAnMdFileIsALinkToItsPage() throws {
        let text = html(try shown("[the other](./other.md)\n").first)

        XCTAssertTrue(text.contains("href=\"/pages/other\""), text)
    }

    // リンクの行き先は、Web、ページ、assetsのファイルに分かれる。ほかのリンクは、どこにも行かない。
    func testALinkLeadsToTheWebToAPageOrToAFileOfTheAssetsFolder() {
        func target(_ href: String) -> LinkTarget? {
            URL(string: href, relativeTo: ShownText.base).flatMap { LinkTarget(of: $0.absoluteURL) }
        }

        XCTAssertEqual(target("https://example.com/a?b=c"), .web(URL(string: "https://example.com/a?b=c")!))
        XCTAssertEqual(target("/pages/other%20page"), .page("/pages/other page"))
        XCTAssertEqual(target("/pages/notes#2020-01-01-00-00-00-AAAAAAA"), .page("/pages/notes#2020-01-01-00-00-00-AAAAAAA"))
        XCTAssertEqual(target("../assets/Report%201.pdf"), .file("assets/Report 1.pdf"))
        XCTAssertNil(target("file:///C:/notes.txt"))
        XCTAssertNil(target("../other/report.pdf"))
        XCTAssertNil(target("mailto:someone@example.com"))
    }
}
