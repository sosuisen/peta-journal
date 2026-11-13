import Foundation
import PetaBorder
import PetaTypes
import XCTest

/// The parts of an item as it is shown, drawn from the AST that the core gives with the row.
@MainActor
final class ShownBlocksTests: JournalTestCase {
    /// The parts of the items of the page `notes` with this Markdown, one item after the
    /// other. An empty line ends an item, unless the line of the core stands above the item.
    private func shown(_ markdown: String, tagsDir: String = "tags") throws -> [ShownBlock] {
        try download("notes.md", markdown)
        try openNotes()
        waitUntil("the items are shown") { !self.shell.view.outline.rows.isEmpty }
        return shell.view.outline.rows.flatMap { shownBlocks(ast: $0.ast, tagsDir: tagsDir) }
    }

    /// The pieces of the paragraphs of a text.
    private func pieces(_ block: ShownBlock?) -> [ShownPiece] {
        if case .text(let paragraphs) = block { return paragraphs.flatMap(\.pieces) }
        return []
    }

    /// The letters of a text, without their looks.
    private func letters(_ block: ShownBlock?) -> String {
        pieces(block).map(\.text).joined()
    }

    // 項目は、Markdownの記号ではなく、飾りの付いた文字になる。
    func testAnItemIsTextWithItsDecorationAndNotTheMarksOfMarkdown() throws {
        let blocks = try shown("some **bold** text\n")

        XCTAssertEqual(
            pieces(blocks.first),
            [ShownPiece(text: "some "), ShownPiece(text: "bold", looks: .bold), ShownPiece(text: " text")])
    }

    // コードブロックは、行が分かれたまま、書かれたとおりの部品になる。フェンスの記号は入らない。
    func testACodeBlockKeepsItsLinesAsTheyWereWritten() throws {
        let blocks = try shown("before\n\n```kotlin\nval x = 1\n  val y = 2\n```\n\nafter\n")

        guard blocks.count == 3 else { return XCTFail("\(blocks)") }
        XCTAssertEqual(letters(blocks[0]), "before")
        XCTAssertEqual(blocks[1], .code(lines: "val x = 1\n  val y = 2"))
        XCTAssertEqual(letters(blocks[2]), "after")
    }

    // 表は、セルごとに分かれた部品になる。セルの中の文字の飾りも付く。
    func testATableHasEachCellApart() throws {
        let blocks = try shown("| name | count |\n|---|---|\n| **apple** | 2 |\n")

        guard case .table(let rows) = blocks.first else { return XCTFail("no table: \(blocks)") }
        XCTAssertEqual(rows.map(\.header), [true, false])
        XCTAssertEqual(rows.map(\.cells.count), [2, 2])
        XCTAssertEqual(rows[0].cells[0].pieces, [ShownPiece(text: "name")])
        XCTAssertEqual(rows[1].cells[0].pieces, [ShownPiece(text: "apple", looks: .bold)])
        XCTAssertEqual(rows[1].cells[1].pieces, [ShownPiece(text: "2")])
    }

    // assetsの画像は、前後の文字のあいだの、画像の部品になる。
    func testAnImageOfTheAssetsFolderStandsBetweenTheTextAroundIt() throws {
        let blocks = try shown("before ![a cat](../assets/Cat.png) after\n")

        guard blocks.count == 3 else { return XCTFail("\(blocks)") }
        XCTAssertEqual(letters(blocks[0]), "before")
        XCTAssertEqual(blocks[1], .image(ShownImage(path: "assets/Cat.png", alt: "a cat", width: nil, height: nil)))
        XCTAssertEqual(letters(blocks[2]), "after")
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
        XCTAssertEqual(
            pieces(blocks.first),
            [
                ShownPiece(text: "see "), ShownPiece(text: "a bird", link: "https://example.com/bird.png"),
                ShownPiece(text: " here"),
            ])
    }

    // リストの中のコードブロックも、行が分かれたまま残る。リストの項目は、番号と一緒に残る。
    func testACodeBlockInsideAListKeepsItsLinesAndTheEntriesKeepTheirNumbers() throws {
        let blocks = try shown(above("A") + "1. first\n\n   ```\n   a\n     b\n   ```\n2. second\n")

        guard case .entries(let entries) = blocks.first else { return XCTFail("no entries: \(blocks)") }
        XCTAssertEqual(entries.map(\.mark), ["1.", "2."])
        XCTAssertEqual(entries[0].blocks.count, 2)
        XCTAssertEqual(letters(entries[0].blocks[0]), "first")
        XCTAssertEqual(entries[0].blocks[1], .code(lines: "a\n  b"))
        XCTAssertEqual(letters(entries[1].blocks.first), "second")
    }

    // 引用の中の表も、セルごとに分かれて残る。
    func testATableInsideAQuoteHasEachCellApart() throws {
        let blocks = try shown(above("A") + "> said:\n>\n> | name | count |\n> |---|---|\n> | apple | 2 |\n")

        guard case .quote(let inside) = blocks.first else { return XCTFail("no quote: \(blocks)") }
        XCTAssertEqual(letters(inside.first), "said:")
        guard case .table(let rows) = inside.last else { return XCTFail("no table: \(inside)") }
        XCTAssertEqual(rows.count, 2)
    }

    // リストの中のリストの画像も、画像の部品になる。
    func testAnImageInsideAListInsideAListIsAPicture() throws {
        let blocks = try shown(above("A") + "- outer\n  - look ![a cat](../assets/cat.png)\n- next\n")

        guard case .entries(let outer) = blocks.first else { return XCTFail("no entries: \(blocks)") }
        XCTAssertEqual(outer.map(\.mark), ["•", "•"])
        guard case .entries(let inner) = outer[0].blocks.last else { return XCTFail("no inner list: \(outer[0])") }
        XCTAssertEqual(inner[0].blocks.last, .image(ShownImage(path: "assets/cat.png", alt: "a cat", width: nil, height: nil)))
    }

    // 自分の部品を持たないリストは、分けられずに1つの文字のままである。
    func testAListThatHoldsNoPartOfItsOwnStaysOneText() throws {
        let blocks = try shown(above("A") + "- one\n- two\n")

        guard blocks.count == 1, case .text(let paragraphs) = blocks.first else { return XCTFail("\(blocks)") }
        XCTAssertEqual(paragraphs.map(\.mark), ["•", "•"])
    }

    // リストの中のタスクの項目は、チェックボックスの印を持つ。
    func testTheEntriesOfATaskListHaveACheckbox() throws {
        // An item that starts with the box of a task would be a task itself, and no list
        let blocks = try shown(above("A") + "- plain\n- [ ] open\n\n  ```\n  code\n  ```\n- [x] done\n")

        guard case .entries(let entries) = blocks.last else { return XCTFail("no entries: \(blocks)") }
        XCTAssertEqual(entries.map(\.mark), ["•", "☐", "☑"])
    }

    // ページへのリンクとタグは、そのページのURLへのリンクになる。タグのページは、タグのフォルダーの下にある。
    func testALinkToAPageAndATagAreLinksToTheURLOfTheirPage() throws {
        let links = pieces(try shown("[[other page]] and #topic\n", tagsDir: "tags/").first).compactMap(\.link)

        XCTAssertEqual(links, ["/pages/other%20page", "/pages/tags/topic"])
    }

    // .mdへの相対リンクは、そのページのURLへのリンクになる。
    func testARelativeLinkToAnMdFileIsALinkToItsPage() throws {
        let links = pieces(try shown("[the other](./other.md)\n").first).compactMap(\.link)

        XCTAssertEqual(links, ["/pages/other"])
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
