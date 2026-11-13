import AppKit
import PetaTypes
import XCTest

@testable import PetaBorder

/// The text of an item as it is shown, with its decoration, drawn from the parts that the shared
/// reader makes of the AST that the core gives with the row (ADR 041). The test of every kind takes its body through a real journal server,
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

    /// The row of an item with this body, typed into a page of a real journal server.
    private func row(withBody body: String) async throws -> Row {
        let server = try JournalServer(port: 18099)
        self.server = server
        let shell = Shell(server: server.address, url: "/pages/shown", clipboard: { nil })
        try await waitUntil("the page is shown") { !shell.view.outline.rows.isEmpty }
        let first = shell.view.outline.rows[0].item.id
        shell.startEditing(first)
        shell.typed(body, into: first)
        shell.send(.editFinished(itemId: first))
        try await waitUntil("the edit is over") {
            let row = shell.view.outline.rows[0]
            return !row.isEditing && row.item.body == body
        }
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

        let shown = Self.shown(row.ast).string

        let kinds = Self.kinds(of: row.ast)
        for (kind, _, seen) in Self.everyKind {
            XCTAssertTrue(kinds.contains(Self.caseName(of: kind)), "the body has no node of the kind \(kind)")
            XCTAssertTrue(shown.contains(seen), "the kind \(kind) is not shown: \(seen) is missing")
        }
        XCTAssertTrue(shown.contains("☑ CLOSED"))
    }

    /// The names of the cases of the nodes in the tree, such as `codeBlock`.
    private static func kinds(of node: Node) -> Set<String> {
        // A case without values has no label of its own: it is its own name
        let name = Mirror(reflecting: node).children.first?.label ?? String(describing: node)
        return children(of: node).reduce([name]) { $0.union(kinds(of: $1)) }
    }

    /// The name of the case of `Node` for a kind of the AST, such as `codeBlock` for `code_block`.
    private static func caseName(of kind: String) -> String {
        if kind == "subscript" { return "subscriptText" }
        let words = kind.split(separator: "_")
        return String(words[0]) + words.dropFirst().map(\.capitalized).joined()
    }

    /// The links as the core writes them, with the tags in `tags/`.
    private static let links = PageLinks(
        page: { "/pages/" + ($0.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? $0) },
        tag: { "/pages/tags/" + $0 })

    /// The text of the AST, at this zoom and with these images.
    private static func shown(
        _ ast: Node, zoom: Double = 1, image: @escaping (String) -> NSImage? = { _ in nil }
    ) -> NSAttributedString {
        ShownText.of(shownBlocks(ast: ast, links: links), zoom: zoom, image: image)
    }

    private func shown(_ ast: Node, zoom: Double = 1) -> NSAttributedString {
        Self.shown(ast, zoom: zoom)
    }

    /// Where the link at the letters `letters` of the text leads.
    private func target(of letters: String, in shown: NSAttributedString) -> LinkTarget? {
        let at = (shown.string as NSString).range(of: letters).location
        return (shown.attribute(.link, at: at, effectiveRange: nil) as? URL).flatMap { LinkTarget(of: $0) }
    }

    private func text(_ value: String) -> Node { .text(value: value) }

    private func document(_ blocks: Node...) -> Node { .document(children: blocks) }

    private func paragraph(_ inline: Node...) -> Node { .paragraph(children: inline) }

    // タスクリストは箱の文字で、区切り線は線で出る。assetsの画像は、届くまで代替の文字で出る。
    func testTasksAndRulesAreTextAndAnImageIsItsWordsUntilItComes() {
        let tasks = Node.list(
            listType: .bullet, start: 1, tight: true, delimiter: .period,
            children: [
                .taskItem(symbol: nil, children: [paragraph(text("OPEN"))]),
                .taskItem(symbol: "x", children: [paragraph(text("CLOSED"))]),
            ])
        let images = paragraph(
            .embed(url: "../assets/a.png", title: "T", props: [:], children: [text("ALT")]),
            .embed(url: "../assets/b.png", title: "", props: [:], children: []))

        let shown = shown(document(tasks, .thematicBreak, images)).string

        XCTAssertTrue(shown.contains("☐ OPEN"))
        XCTAssertTrue(shown.contains("☑ CLOSED"))
        XCTAssertTrue(shown.contains("────"))
        XCTAssertTrue(shown.contains("[ALT]"))
        // An image without words for it is named by its file
        XCTAssertTrue(shown.contains("[assets/b.png]"))
    }

    // 届いたassetsの画像は、絵として出る。大きさは、ページが求めた幅に合わせ、縦横の比を保つ。
    func testAnImageThatCameIsDrawnAtTheWidthThePageAsksFor() {
        let picture = NSImage(size: NSSize(width: 100, height: 50))
        let asked = paragraph(.embed(url: "../assets/a.png", title: "", props: ["w": .text("200")], children: []))

        let shown = Self.shown(document(asked)) { $0 == "assets/a.png" ? picture : nil }

        let attachment = shown.attribute(.attachment, at: 0, effectiveRange: nil) as? NSTextAttachment
        XCTAssertTrue(attachment?.image === picture)
        XCTAssertEqual(attachment?.bounds.size, NSSize(width: 200, height: 100))
    }

    // リンクは、表示するテキストに残る。ウィキリンクとハッシュタグはアプリの中のページを指し、外のリンクはWebを指す。
    func testLinksStayInTheShownText() {
        let line = paragraph(
            .wikilink(url: "notes/Idea", children: [text("Idea")]), text(" "), .hashtag(value: "todo"), text(" "),
            .link(url: "https://example.com/a", title: "", children: [text("out")]))

        let shown = shown(document(line))

        XCTAssertEqual(target(of: "Idea", in: shown), .page("/pages/notes/Idea"))
        XCTAssertEqual(target(of: "#todo", in: shown), .page("/pages/tags/todo"))
        XCTAssertEqual(target(of: "out", in: shown), .web(URL(string: "https://example.com/a")!))
    }

    // 名前に空白のあるページへのリンクも、そのページを指す。
    func testALinkToAPageWithASpaceInItsNameLeadsToThatPage() {
        let shown = shown(document(paragraph(.wikilink(url: "my page", children: [text("p")]))))
        XCTAssertEqual(target(of: "p", in: shown), .page("/pages/my page"))
    }

    // 表示するテキストの終わりに、余分な改行はない。
    func testTheShownTextDoesNotEndWithANewline() {
        XCTAssertEqual(shown(document(paragraph(text("one")), paragraph(text("two")))).string, "one\ntwo")
    }

    // 見出しは、本文より大きな文字で出る。
    func testAHeadingIsLargerThanTheText() {
        let shown = shown(document(.heading(level: 1, children: [text("H")]), paragraph(text("p"))))
        let size = { (at: Int) in (shown.attribute(.font, at: at, effectiveRange: nil) as? NSFont)?.pointSize ?? 0 }
        XCTAssertGreaterThan(size(0), size(2))
    }

    // ズームすると、表示するテキストの文字が大きくなる。
    func testTheZoomMakesTheShownTextLarger() {
        let usual = shown(document(paragraph(text("text"))), zoom: 1)
        let doubled = shown(document(paragraph(text("text"))), zoom: 2)

        let size = { (text: NSAttributedString) in (text.attribute(.font, at: 0, effectiveRange: nil) as? NSFont)?.pointSize }
        XCTAssertEqual(size(usual), NSFont.systemFontSize)
        XCTAssertEqual(size(doubled), NSFont.systemFontSize * 2)
    }

    // 本文のない項目は、何も出ない。
    func testAnEmptyItemShowsNothing() {
        XCTAssertEqual(shown(document()).string, "")
    }
}
