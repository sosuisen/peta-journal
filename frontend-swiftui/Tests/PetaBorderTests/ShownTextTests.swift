import AppKit
import PetaTypes
import XCTest

@testable import PetaBorder

/// The text of an item as it is shown, with its decoration (ADR 025). The AST comes from a real
/// journal server, so the tests that need one are skipped without `PETA_E2E_SERVER`.
@MainActor
final class ShownTextTests: XCTestCase {
    private var server: JournalServer?

    override func tearDown() async throws {
        server?.stop()
    }

    private func waitUntil(_ what: String, _ done: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(10)
        while !done() {
            if Date() > deadline {
                XCTFail("never happened: \(what)")
                throw CancellationError()
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    /// An item with this body, as the journal server gives it back: with its AST.
    private func item(withBody body: String) async throws -> Item {
        let server = try JournalServer(port: 18099)
        self.server = server
        let shell = Shell(server: server.address, url: "/pages/shown", clipboard: { nil })
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }
        let first = shell.view.outline.rows[0].item.id
        shell.startEditing(first)
        shell.typed(body, into: first)
        shell.send(.editFinished(itemId: first))
        try await waitUntil("the server gave the AST") { !shell.view.outline.rows[0].item.ast.isEmpty }
        return shell.view.outline.rows[0].item
    }

    /// A body with every kind of node that the AST has, and for each kind what of it has to be
    /// seen. `document`, `item`, `table_row` and `table_cell` are seen by what is in them.
    private static let everyKind: [(kind: String, markdown: String, seen: String)] = [
        ("heading", "## HEADING", "HEADING"),
        ("paragraph", "PARAGRAPH", "PARAGRAPH"),
        ("text", "TEXT", "TEXT"),
        ("softbreak", "SOFT\nBREAK", "BREAK"),
        ("linebreak", "LINE\\\nBROKEN", "BROKEN"),
        ("emph", "*EMPH*", "EMPH"),
        ("strong", "**STRONG**", "STRONG"),
        ("strikethrough", "~~STRUCK~~", "STRUCK"),
        ("underline", "__UNDERLINED__", "UNDERLINED"),
        ("highlight", "==HIGHLIGHTED==", "HIGHLIGHTED"),
        ("superscript", "x^SUPER^", "SUPER"),
        ("subscript", "x~SUB~", "SUB"),
        ("spoilered_text", "||SPOILER||", "SPOILER"),
        ("code", "`CODE`", "CODE"),
        ("link", "[LINK](https://example.com/)", "LINK"),
        ("embed", "![ALT](images/a.png)", "ALT"),
        ("wikilink", "[[WIKI]]", "WIKI"),
        ("hashtag", "#hashtag", "#hashtag"),
        ("shortcode", ":smile:", "😄"),
        ("math", "$`MATH`$", "MATH"),
        ("html_inline", "<kbd>KEY</kbd>", "KEY"),
        ("footnote_reference", "NOTED[^note]", "[note]"),
        ("footnote_definition", "[^note]: FOOTNOTE", "FOOTNOTE"),
        ("code_block", "```rust\nCODEBLOCK\n```", "CODEBLOCK"),
        ("block_quote", "> QUOTED", "QUOTED"),
        ("multiline_block_quote", ">>>\nMULTIQUOTED\n>>>", "MULTIQUOTED"),
        ("alert", "> [!TIP]\n> ALERTED", "Tip"),
        ("list", "1. LISTED", "LISTED"),
        ("task_item", "- [ ] OPEN\n- [x] CLOSED", "☐ OPEN"),
        ("table", "| HEAD |\n|---|\n| CELL |", "CELL"),
        ("thematic_break", "---", "────"),
        ("subtext", "-# SUBTEXT", "SUBTEXT"),
        ("html_block", "<details>\nHTMLBLOCK\n</details>", "HTMLBLOCK"),
    ]

    // ASTのどの種類の中身も、表示するテキストに入っている。
    func testWhatIsInEveryKindOfNodeIsInTheShownText() async throws {
        let body = Self.everyKind.map(\.markdown).joined(separator: "\n\n")
        let item = try await item(withBody: body)

        let shown = ShownText.of(item).string

        for (kind, _, seen) in Self.everyKind {
            XCTAssertTrue(item.ast.contains("\"type\":\"\(kind)\""), "the body has no node of the kind \(kind)")
            XCTAssertTrue(shown.contains(seen), "the kind \(kind) is not shown: \(seen) is missing")
        }
        XCTAssertTrue(shown.contains("☑ CLOSED"))
    }

    // 本文の中のタスクリストは箱の文字で、区切り線は線で、画像は代替の文字で出る。macOSの変換は、この3つを落とす。
    func testWhatMacOSWouldDropIsPutIntoText() {
        let html =
            "<ul>\n<li><input type=\"checkbox\" disabled>OPEN</li>\n<li><input type=\"checkbox\" checked disabled>CLOSED</li>\n</ul>\n"
            + "<hr />\n<p><img src=\"images/a.png\" alt=\"ALT\" title=\"T\" /><img src=\"images/b.png\" alt=\"\" /></p>\n"

        let shown = ShownText.text(ofHTML: html, zoom: 1).string

        XCTAssertTrue(shown.contains("☐ OPEN"))
        XCTAssertTrue(shown.contains("☑ CLOSED"))
        XCTAssertTrue(shown.contains("────"))
        XCTAssertTrue(shown.contains("[ALT]"))
        // An image without words for it is named by its file
        XCTAssertTrue(shown.contains("[images/b.png]"))
    }

    // リンクは、表示するテキストに残る。ウィキリンクとハッシュタグはページを指し、外のリンクはそのままである。
    func testLinksStayInTheShownText() {
        let html =
            "<p><a href=\"notes/Idea\" data-wikilink=\"true\">Idea</a> <a href=\"todo\" class=\"hashtag\">#todo</a> "
            + "<a href=\"https://example.com/a\">out</a></p>\n"

        let shown = ShownText.text(ofHTML: html, zoom: 1)

        var links: [String: URL] = [:]
        shown.enumerateAttribute(.link, in: NSRange(location: 0, length: shown.length)) { link, range, _ in
            if let link = link as? URL { links[(shown.string as NSString).substring(with: range)] = link }
        }
        XCTAssertEqual(links["Idea"].flatMap(ShownText.page), "notes/Idea")
        XCTAssertEqual(links["#todo"].flatMap(ShownText.page), "todo")
        XCTAssertEqual(links["out"], URL(string: "https://example.com/a"))
        XCTAssertNil(links["out"].flatMap(ShownText.page))
    }

    // 表示するテキストの終わりに、余分な改行はない。
    func testTheShownTextDoesNotEndWithANewline() {
        XCTAssertEqual(ShownText.text(ofHTML: "<p>one</p>\n<p>two</p>\n", zoom: 1).string, "one\ntwo")
    }

    // ズームすると、表示するテキストの文字が大きくなる。
    func testTheZoomMakesTheShownTextLarger() {
        let usual = ShownText.text(ofHTML: "<p>text</p>\n", zoom: 1)
        let doubled = ShownText.text(ofHTML: "<p>text</p>\n", zoom: 2)

        let size = { (text: NSAttributedString) in (text.attribute(.font, at: 0, effectiveRange: nil) as? NSFont)?.pointSize }
        XCTAssertEqual(size(usual), NSFont.systemFontSize)
        XCTAssertEqual(size(doubled), NSFont.systemFontSize * 2)
    }

    // ASTが読めない項目は、本文がそのまま出る。本文のない項目は、何も出ない。
    func testAnItemWhoseASTCannotBeReadShowsItsBody() {
        func item(body: String, ast: String) -> Item {
            Item(
                id: "a", pageId: "p", dirPath: "", name: "notes", itemType: "doc", createdDate: 0,
                modifiedDate: 0, idx: "a", level: 1, body: body, ast: ast, collapsed: false, types: [],
                properties: [:], transcludeItemId: nil)
        }

        XCTAssertEqual(ShownText.of(item(body: "**as it is** <b>", ast: "{\"type\":")).string, "**as it is** <b>")
        XCTAssertEqual(ShownText.of(item(body: "", ast: "")).string, "")
    }
}
