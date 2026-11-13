import PetaShown
import PetaTypes
import XCTest

/// A screen draws paragraphs of pieces with a look each. They are made of the AST that the core
/// gives (ADR 041).
final class ShownParagraphsTests: XCTestCase {
    private func piece(_ text: String, _ looks: Looks = [], link: String? = nil) -> ShownPiece {
        ShownPiece(text: text, looks: looks, link: link)
    }

    private func text(_ value: String) -> Node { .text(value: value) }

    private func paragraph(_ inline: Node...) -> Node { .paragraph(children: inline) }

    /// The links as the core writes them, with the tags in `tags/`.
    private let links = PageLinks(
        page: { "/pages/" + ($0.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? $0) },
        tag: { "/pages/tags/" + $0 })

    private func shown(_ blocks: Node...) -> [ShownParagraph] {
        shownParagraphs(of: blocks, links: links)
    }

    private func pieces(_ blocks: Node...) -> [[ShownPiece]] {
        shownParagraphs(of: blocks, links: links).map(\.pieces)
    }

    private func list(ordered: Bool = false, start: UInt32 = 1, _ entries: Node...) -> Node {
        .list(listType: ordered ? .ordered : .bullet, start: start, tight: true, delimiter: .period, children: entries)
    }

    private func entry(_ blocks: Node...) -> Node {
        .item(listType: .bullet, start: 1, children: blocks)
    }

    // 段落は、1つの段落になる。
    func testAParagraphIsOneParagraph() {
        XCTAssertEqual(shown(paragraph(text("plain"))), [ShownParagraph(pieces: [piece("plain")])])
    }

    // 段落が2つあれば、2つの段落になる。
    func testTwoParagraphsAreTwoParagraphs() {
        XCTAssertEqual(pieces(paragraph(text("one")), paragraph(text("two"))), [[piece("one")], [piece("two")]])
    }

    // 太字・斜体・取り消し線・下線・ハイライト・コードは、その部分だけの見た目になる。
    func testTheLooksOfLettersBelongToTheirPieceOnly() {
        let line = paragraph(
            text("a "), .strong(children: [text("b")]), text(" "), .emph(children: [text("c")]), text(" "),
            .strikethrough(children: [text("d")]), text(" "), .underline(children: [text("e")]), text(" "),
            .highlight(children: [text("f")]), text(" "), .code(literal: "g"))
        XCTAssertEqual(pieces(line), [[
            piece("a "), piece("b", .bold), piece(" "), piece("c", .italic), piece(" "), piece("d", .struck),
            piece(" "), piece("e", .underlined), piece(" "), piece("f", .marked), piece(" "), piece("g", .code),
        ]])
    }

    // 入れ子の装飾は、両方の見た目を持つ。
    func testLooksInsideEachOtherAddUp() {
        let line = paragraph(.strong(children: [text("a "), .emph(children: [text("b")])]))
        XCTAssertEqual(pieces(line), [[piece("a ", .bold), piece("b", [.bold, .italic])]])
    }

    // 上付き・下付き・ネタバレも見た目になる。
    func testRaisedLoweredAndHiddenAreLooks() {
        let line = paragraph(
            .superscript(children: [text("a")]), .subscriptText(children: [text("b")]),
            .spoileredText(children: [text("c")]))
        XCTAssertEqual(pieces(line), [[piece("a", .raised), piece("b", .lowered), piece("c", .hidden)]])
    }

    // リンクは、その部分が行き先を持つ。ページへのリンクとタグの行き先は、そのページのURLである。
    func testALinkGivesItsPieceWhereItLeads() {
        let line = paragraph(
            .link(url: "https://e.example/?a=1&b=2", title: "", children: [text("e")]), text(" "),
            .wikilink(url: "other page", children: [text("other")]), text(" "), .hashtag(value: "topic"))
        XCTAssertEqual(pieces(line), [[
            piece("e", link: "https://e.example/?a=1&b=2"), piece(" "), piece("other", link: "/pages/other%20page"),
            piece(" "), piece("#topic", link: "/pages/tags/topic"),
        ]])
    }

    // 見出しは、そのレベルを持つ。
    func testAHeadingHasItsLevel() {
        XCTAssertEqual(
            shown(.heading(level: 2, children: [text("Title")])), [ShownParagraph(heading: 2, pieces: [piece("Title")])])
    }

    // 段落の中の改行は空白になり、改行の記号は改行になる。
    func testALineEndIsASpaceAndABreakIsALineBreak() {
        XCTAssertEqual(
            pieces(paragraph(text("one"), .softbreak, text("two"), .linebreak, text("three"))),
            [[piece("one two\nthree")]])
    }

    // 箇条書きの項目は、印と深さを持つ段落になる。
    func testTheEntriesOfAListHaveAMarkAndADepth() {
        XCTAssertEqual(
            shown(list(entry(paragraph(text("one"))), entry(paragraph(text("two")), list(entry(paragraph(text("inner"))))))),
            [
                ShownParagraph(depth: 1, mark: "•", pieces: [piece("one")]),
                ShownParagraph(depth: 1, mark: "•", pieces: [piece("two")]),
                ShownParagraph(depth: 2, mark: "•", pieces: [piece("inner")]),
            ])
    }

    // 番号付きリストの印は、始まりの番号から数える。
    func testTheMarksOfANumberedListCountFromItsStart() {
        let three = list(ordered: true, start: 3, entry(paragraph(text("c"))), entry(paragraph(text("d"))))
        let one = list(ordered: true, entry(paragraph(text("a"))))
        XCTAssertEqual(shown(three, one).map(\.mark), ["3.", "4.", "1."])
    }

    // タスクの項目の印は、チェックボックスである。
    func testTheMarkOfATaskEntryIsACheckbox() {
        let tasks = list(
            .taskItem(symbol: nil, children: [paragraph(text("open"))]),
            .taskItem(symbol: "x", children: [paragraph(text("done"))]))
        XCTAssertEqual(
            shown(tasks),
            [
                ShownParagraph(depth: 1, mark: "☐", pieces: [piece("open")]),
                ShownParagraph(depth: 1, mark: "☑", pieces: [piece("done")]),
            ])
    }

    // 項目の中の2つ目の段落は、印を持たず、同じ深さにある。
    func testTheSecondParagraphOfAnEntryHasNoMark() {
        XCTAssertEqual(
            shown(list(entry(paragraph(text("one")), paragraph(text("more"))))),
            [
                ShownParagraph(depth: 1, mark: "•", pieces: [piece("one")]),
                ShownParagraph(depth: 1, pieces: [piece("more")]),
            ])
    }

    // 引用の中の段落は、引用の深さを持つ。
    func testAParagraphInAQuoteKnowsHowDeepItIsQuoted() {
        let quote = Node.blockQuote(children: [paragraph(text("said")), .blockQuote(children: [paragraph(text("before"))])])
        XCTAssertEqual(
            shown(quote, paragraph(text("after"))),
            [
                ShownParagraph(quoted: 1, pieces: [piece("said")]),
                ShownParagraph(quoted: 2, pieces: [piece("before")]),
                ShownParagraph(pieces: [piece("after")]),
            ])
    }

    // アラートは、題を最初の段落に持つ引用になる。題がなければ、種類の名前が題である。
    func testAnAlertIsAQuoteWithItsTitleFirst() {
        let alert = Node.alert(alertType: .warning, title: nil, children: [paragraph(text("careful"))])
        XCTAssertEqual(
            shown(alert),
            [ShownParagraph(quoted: 1, pieces: [piece("Warning")]), ShownParagraph(quoted: 1, pieces: [piece("careful")])])
    }

    // 区切り線は、線だけの段落になる。
    func testAThematicBreakIsAParagraphThatIsARule() {
        XCTAssertEqual(
            shown(paragraph(text("a")), .thematicBreak, paragraph(text("b"))),
            [ShownParagraph(pieces: [piece("a")]), ShownParagraph(rule: true), ShownParagraph(pieces: [piece("b")])])
    }

    // 脚注は、その名前を前に置いた段落になる。脚注への参照は、上付きの名前になる。
    func testAFootnoteStartsWithItsNameAndAReferenceIsItsNameRaised() {
        let reference = paragraph(text("see"), .footnoteReference(name: "__inline_1", refNum: 1, ix: 1))
        let note = Node.footnoteDefinition(name: "__inline_1", children: [paragraph(text("note"))])
        XCTAssertEqual(pieces(reference, note), [[piece("see"), piece("[1]", .raised)], [piece("[1]: note")]])
    }

    // 未知の種類は、その中の文字が残る。
    func testTheTextInsideAnUnknownKindStays() {
        XCTAssertEqual(pieces(paragraph(.unknown(nodeType: "insert", children: [text("kept")]))), [[piece("kept")]])
    }
}
