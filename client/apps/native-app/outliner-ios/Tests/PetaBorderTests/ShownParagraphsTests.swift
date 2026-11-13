import PetaBorder
import XCTest

/// The text of an item is HTML when it leaves the core. A screen of iOS draws paragraphs of
/// pieces with a look each, so the HTML is read into those here, where a Mac can test it.
final class ShownParagraphsTests: XCTestCase {
    private func piece(_ text: String, _ looks: Looks = [], link: String? = nil) -> ShownPiece {
        ShownPiece(text: text, looks: looks, link: link)
    }

    private func pieces(_ html: String) -> [[ShownPiece]] {
        shownParagraphs(html: html).map(\.pieces)
    }

    // 段落は、1つの段落になる。
    func testAParagraphIsOneParagraph() {
        XCTAssertEqual(shownParagraphs(html: "<p>plain</p>\n"), [ShownParagraph(pieces: [piece("plain")])])
    }

    // 段落が2つあれば、2つの段落になる。段落の間の改行は文字にならない。
    func testTwoParagraphsAreTwoParagraphs() {
        XCTAssertEqual(pieces("<p>one</p>\n<p>two</p>\n"), [[piece("one")], [piece("two")]])
    }

    // 太字・斜体・取り消し線・下線・ハイライト・コードは、その部分だけの見た目になる。
    func testTheLooksOfLettersBelongToTheirPieceOnly() {
        let html = "<p>a <strong>b</strong> <em>c</em> <del>d</del> <u>e</u> <mark>f</mark> <code>g</code></p>"
        XCTAssertEqual(pieces(html), [[
            piece("a "), piece("b", .bold), piece(" "), piece("c", .italic), piece(" "), piece("d", .struck),
            piece(" "), piece("e", .underlined), piece(" "), piece("f", .marked), piece(" "), piece("g", .code),
        ]])
    }

    // 入れ子の装飾は、両方の見た目を持つ。
    func testLooksInsideEachOtherAddUp() {
        XCTAssertEqual(
            pieces("<p><strong>a <em>b</em></strong></p>"), [[piece("a ", .bold), piece("b", [.bold, .italic])]])
    }

    // 上付き・下付き・ネタバレも見た目になる。
    func testRaisedLoweredAndHiddenAreLooks() {
        XCTAssertEqual(
            pieces("<p><sup>a</sup><sub>b</sub><span class=\"spoiler\">c</span></p>"),
            [[piece("a", .raised), piece("b", .lowered), piece("c", .hidden)]])
    }

    // リンクは、その部分が行き先を持つ。
    func testALinkGivesItsPieceWhereItLeads() {
        XCTAssertEqual(
            pieces("<p>see <a href=\"/pages/notes\" data-wikilink=\"true\">notes</a>.</p>"),
            [[piece("see "), piece("notes", link: "/pages/notes"), piece(".")]])
    }

    // 文字と行き先の実体参照は、元の文字に戻る。
    func testEntitiesAreTheLettersTheyStandFor() {
        XCTAssertEqual(
            pieces("<p>a &lt;b&gt; &amp; &quot;c&quot; &#39;d&#x27; <a href=\"https://e.example/?a=1&amp;b=2\">e</a></p>"),
            [[piece("a <b> & \"c\" 'd' "), piece("e", link: "https://e.example/?a=1&b=2")]])
    }

    // 見出しは、そのレベルを持つ。
    func testAHeadingHasItsLevel() {
        XCTAssertEqual(
            shownParagraphs(html: "<h2>Title</h2>\n"), [ShownParagraph(heading: 2, pieces: [piece("Title")])])
    }

    // 段落の中の改行は空白になり、改行タグは改行になる。
    func testALineEndIsASpaceAndABreakIsALineBreak() {
        XCTAssertEqual(pieces("<p>one\ntwo<br />\nthree</p>"), [[piece("one two\nthree")]])
    }

    // 箇条書きの項目は、印と深さを持つ段落になる。
    func testTheEntriesOfAListHaveAMarkAndADepth() {
        XCTAssertEqual(
            shownParagraphs(html: "<ul>\n<li>one</li>\n<li>two\n<ul>\n<li>inner</li>\n</ul>\n</li>\n</ul>\n"),
            [
                ShownParagraph(depth: 1, mark: "•", pieces: [piece("one")]),
                ShownParagraph(depth: 1, mark: "•", pieces: [piece("two")]),
                ShownParagraph(depth: 2, mark: "•", pieces: [piece("inner")]),
            ])
    }

    // 番号付きリストの印は、始まりの番号から数える。
    func testTheMarksOfANumberedListCountFromItsStart() {
        XCTAssertEqual(
            shownParagraphs(html: "<ol start=\"3\">\n<li>c</li>\n<li>d</li>\n</ol>\n<ol>\n<li>a</li>\n</ol>\n").map(\.mark),
            ["3.", "4.", "1."])
    }

    // タスクの項目の印は、チェックボックスである。
    func testTheMarkOfATaskEntryIsACheckbox() {
        let html = "<ul>\n<li><input type=\"checkbox\" disabled>open</li>\n<li><input type=\"checkbox\" checked disabled>done</li>\n</ul>\n"
        XCTAssertEqual(
            shownParagraphs(html: html),
            [
                ShownParagraph(depth: 1, mark: "☐", pieces: [piece("open")]),
                ShownParagraph(depth: 1, mark: "☑", pieces: [piece("done")]),
            ])
    }

    // 項目の中の2つ目の段落は、印を持たず、同じ深さにある。
    func testTheSecondParagraphOfAnEntryHasNoMark() {
        XCTAssertEqual(
            shownParagraphs(html: "<ul>\n<li><p>one</p>\n<p>more</p>\n</li>\n</ul>\n"),
            [
                ShownParagraph(depth: 1, mark: "•", pieces: [piece("one")]),
                ShownParagraph(depth: 1, pieces: [piece("more")]),
            ])
    }

    // 引用の中の段落は、引用の深さを持つ。
    func testAParagraphInAQuoteKnowsHowDeepItIsQuoted() {
        XCTAssertEqual(
            shownParagraphs(html: "<blockquote>\n<p>said</p>\n<blockquote>\n<p>before</p>\n</blockquote>\n</blockquote>\n<p>after</p>"),
            [
                ShownParagraph(quoted: 1, pieces: [piece("said")]),
                ShownParagraph(quoted: 2, pieces: [piece("before")]),
                ShownParagraph(pieces: [piece("after")]),
            ])
    }

    // 区切り線は、線だけの段落になる。
    func testAThematicBreakIsAParagraphThatIsARule() {
        XCTAssertEqual(
            shownParagraphs(html: "<p>a</p>\n<hr />\n<p>b</p>\n"),
            [ShownParagraph(pieces: [piece("a")]), ShownParagraph(rule: true), ShownParagraph(pieces: [piece("b")])])
    }

    // 知らないタグは無視され、その中の文字は残る。
    func testAnUnknownTagIsLeftOutAndItsTextStays() {
        XCTAssertEqual(pieces("<div id=\"fn-1\" class=\"footnote\">\n<p>note</p>\n</div>\n"), [[piece("note")]])
    }

    // タグでない「<」は、文字のままである。
    func testALessThanSignThatOpensNoTagIsALetter() {
        XCTAssertEqual(pieces("<p>1 < 2</p>"), [[piece("1 < 2")]])
    }
}
