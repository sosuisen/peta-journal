import AppKit
import PetaTypes
import XCTest

@testable import PetaBorder

/// The text of an item as it is shown, with its decoration, drawn from the AST that the core gives
/// with the row (ADR 041). The test of every kind takes its body through a real journal server,
/// so it is skipped without `PETA_E2E_SERVER`.
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

    /// The row of an item with this body, as the journal server gives it back: with its AST.
    private func row(withBody body: String) async throws -> Row {
        let server = try JournalServer(port: 18099)
        self.server = server
        let shell = Shell(server: server.address, url: "/pages/shown", clipboard: { nil })
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }
        let first = shell.view.outline.rows[0].item.id
        shell.startEditing(first)
        shell.typed(body, into: first)
        shell.send(.editFinished(itemId: first))
        try await waitUntil("the server gave the AST") { !shell.view.outline.rows[0].item.ast.isEmpty }
        return shell.view.outline.rows[0]
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
        let row = try await row(withBody: body)

        let shown = ShownText.of(row.ast).string

        for (kind, _, seen) in Self.everyKind {
            XCTAssertTrue(row.item.ast.contains("\"type\":\"\(kind)\""), "the body has no node of the kind \(kind)")
            XCTAssertTrue(shown.contains(seen), "the kind \(kind) is not shown: \(seen) is missing")
        }
        XCTAssertTrue(shown.contains("☑ CLOSED"))
    }

    private func text(_ value: String) -> Node { .text(value: value) }

    private func document(_ blocks: Node...) -> Node { .document(children: blocks) }

    private func paragraph(_ inline: Node...) -> Node { .paragraph(children: inline) }

    // タスクリストは箱の文字で、区切り線は線で、画像は代替の文字で出る。
    func testTasksRulesAndImagesAreText() {
        let tasks = Node.list(
            listType: .bullet, start: 1, tight: true, delimiter: .period,
            children: [
                .taskItem(symbol: nil, children: [paragraph(text("OPEN"))]),
                .taskItem(symbol: "x", children: [paragraph(text("CLOSED"))]),
            ])
        let images = paragraph(
            .embed(url: "images/a.png", title: "T", props: [:], children: [text("ALT")]),
            .embed(url: "images/b.png", title: "", props: [:], children: []))

        let shown = ShownText.of(document(tasks, .thematicBreak, images)).string

        XCTAssertTrue(shown.contains("☐ OPEN"))
        XCTAssertTrue(shown.contains("☑ CLOSED"))
        XCTAssertTrue(shown.contains("────"))
        XCTAssertTrue(shown.contains("[ALT]"))
        // An image without words for it is named by its file
        XCTAssertTrue(shown.contains("[images/b.png]"))
    }

    // リンクは、表示するテキストに残る。ウィキリンクはページを、ハッシュタグはタグを指し、外のリンクはそのままである。
    func testLinksStayInTheShownText() {
        let line = paragraph(
            .wikilink(url: "notes/Idea", children: [text("Idea")]), text(" "), .hashtag(value: "todo"), text(" "),
            .link(url: "https://example.com/a", title: "", children: [text("out")]))

        let shown = ShownText.of(document(line))

        var links: [String: URL] = [:]
        shown.enumerateAttribute(.link, in: NSRange(location: 0, length: shown.length)) { link, range, _ in
            if let link = link as? URL { links[(shown.string as NSString).substring(with: range)] = link }
        }
        XCTAssertEqual(links["Idea"].flatMap(ShownText.target), .page("notes/Idea"))
        XCTAssertEqual(links["#todo"].flatMap(ShownText.target), .tag("todo"))
        XCTAssertEqual(links["out"].flatMap(ShownText.target), .out(URL(string: "https://example.com/a")!))
    }

    // 名前に空白のあるページへのリンクも、そのページを指す。
    func testALinkToAPageWithASpaceInItsNameLeadsToThatPage() {
        let shown = ShownText.of(document(paragraph(.wikilink(url: "my page", children: [text("p")]))))
        let link = shown.attribute(.link, at: 0, effectiveRange: nil) as? URL
        XCTAssertEqual(link.flatMap(ShownText.target), .page("my page"))
    }

    // 表示するテキストの終わりに、余分な改行はない。
    func testTheShownTextDoesNotEndWithANewline() {
        XCTAssertEqual(ShownText.of(document(paragraph(text("one")), paragraph(text("two")))).string, "one\ntwo")
    }

    // 見出しは、本文より大きな文字で出る。
    func testAHeadingIsLargerThanTheText() {
        let shown = ShownText.of(document(.heading(level: 1, children: [text("H")]), paragraph(text("p"))))
        let size = { (at: Int) in (shown.attribute(.font, at: at, effectiveRange: nil) as? NSFont)?.pointSize ?? 0 }
        XCTAssertGreaterThan(size(0), size(2))
    }

    // ズームすると、表示するテキストの文字が大きくなる。
    func testTheZoomMakesTheShownTextLarger() {
        let usual = ShownText.of(document(paragraph(text("text"))), zoom: 1)
        let doubled = ShownText.of(document(paragraph(text("text"))), zoom: 2)

        let size = { (text: NSAttributedString) in (text.attribute(.font, at: 0, effectiveRange: nil) as? NSFont)?.pointSize }
        XCTAssertEqual(size(usual), NSFont.systemFontSize)
        XCTAssertEqual(size(doubled), NSFont.systemFontSize * 2)
    }

    // 本文のない項目は、何も出ない。
    func testAnEmptyItemShowsNothing() {
        XCTAssertEqual(ShownText.of(document()).string, "")
    }
}
