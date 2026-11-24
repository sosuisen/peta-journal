import AppKit
import PetaTypes
import XCTest

@testable import PetaBorder

/// The text of an item as it is shown, with its decoration, drawn from the parts that the core
/// makes of the body (ADR 042). How the core makes the parts is tested in Rust.
@MainActor
final class ShownTextTests: XCTestCase {
    private func run(_ text: String, link: RichLink? = nil) -> RichRun {
        RichRun(text: text, looks: .plain, link: link)
    }

    private func paragraph(
        _ runs: RichRun..., heading: UInt8 = 0, mark: String? = nil, rule: Bool = false, start: UInt32 = 0
    ) -> RichParagraph {
        RichParagraph(
            heading: heading, scale: 1, depth: 0, quoted: 0, indent: 0, mark: mark, rule: rule, runs: runs, start: start
        )
    }

    private func text(_ paragraphs: RichParagraph...) -> RichBlock { .text(paragraphs: paragraphs) }

    private func image(_ path: String, alt: String = "", width: UInt32? = nil, height: UInt32? = nil) -> RichBlock {
        .image(image: RichImage(path: path, alt: alt, width: width, height: height))
    }

    private func shown(
        _ blocks: RichBlock..., zoom: Double = 1, image: @escaping (String) -> NSImage? = { _ in nil }
    ) -> NSAttributedString {
        ShownText.of(RichText(blocks: blocks), zoom: zoom, image: image)
    }

    /// Where the link at the letters `letters` of the text leads.
    private func target(of letters: String, in shown: NSAttributedString) -> RichLink? {
        ShownText.link(in: shown, at: (shown.string as NSString).range(of: letters).location)
    }

    /// The picture at the start of the text.
    private func picture(in shown: NSAttributedString) -> FittedPicture? {
        shown.attribute(.attachment, at: 0, effectiveRange: nil) as? FittedPicture
    }

    // 印は段落の前に、区切り線は線で出る。assetsの画像は、届くまで代替の文字で出る。代替の文字がなければ、ファイルの名前が出る。
    func testMarksAndRulesAreTextAndAnImageIsItsWordsUntilItComes() {
        let shown = shown(
            text(paragraph(run("OPEN"), mark: "☐"), paragraph(rule: true)), image("assets/a.png", alt: "ALT"),
            image("assets/b.png")
        ).string

        XCTAssertTrue(shown.contains("☐ OPEN"))
        XCTAssertTrue(shown.contains("────"))
        XCTAssertTrue(shown.contains("[ALT]"))
        XCTAssertTrue(shown.contains("[assets/b.png]"))
    }

    // 届いたassetsの画像は、絵として出る。大きさは、ページが求めた幅に合わせ、縦横の比を保つ。
    func testAnImageThatCameIsDrawnAtTheWidthThePageAsksFor() {
        let file = NSImage(size: NSSize(width: 100, height: 50))

        let drawn = picture(in: shown(image("assets/a.png", width: 200)) { $0 == "assets/a.png" ? file : nil })

        XCTAssertTrue(drawn?.image === file)
        XCTAssertEqual(drawn?.size(room: 50), CGSize(width: 200, height: 100))
    }

    // 大きさを求めない画像は、付箋の幅に合わせる。元の大きさより大きくはならない。
    func testAnImageWithoutASizeFitsTheStickyAndIsNeverLargerThanItsFile() {
        let file = NSImage(size: NSSize(width: 400, height: 200))

        let drawn = picture(in: shown(image("assets/a.png")) { _ in file })

        XCTAssertEqual(drawn?.size(room: 100), CGSize(width: 100, height: 50))
        XCTAssertEqual(drawn?.size(room: 1000), CGSize(width: 400, height: 200))
    }

    // リンクは、coreが決めた行き先を持ったまま、表示するテキストに残る。
    func testLinksKeepWhereTheCoreSaysTheyLead() {
        let page = RichLink.page(url: "/pages/my page")
        let web = RichLink.web(url: "https://example.com/a")
        let file = RichLink.file(path: "assets/a.pdf")

        let shown = shown(
            text(paragraph(run("Idea", link: page), run(" "), run("out", link: web), run(" "), run("pdf", link: file))))

        XCTAssertEqual(target(of: "Idea", in: shown), page)
        XCTAssertEqual(target(of: "out", in: shown), web)
        XCTAssertEqual(target(of: "pdf", in: shown), file)
        XCTAssertNil(target(of: " ", in: shown))
    }

    // 表示するテキストの終わりに、余分な改行はない。
    func testTheShownTextDoesNotEndWithANewline() {
        XCTAssertEqual(shown(text(paragraph(run("one")), paragraph(run("two")))).string, "one\ntwo")
    }

    // 見出しは、本文より大きな文字で出る。
    func testAHeadingIsLargerThanTheText() {
        let shown = shown(text(paragraph(run("H"), heading: 1), paragraph(run("p"))))
        let size = { (at: Int) in (shown.attribute(.font, at: at, effectiveRange: nil) as? NSFont)?.pointSize ?? 0 }
        XCTAssertGreaterThan(size(0), size(2))
    }

    // ズームすると、表示するテキストの文字が大きくなる。
    func testTheZoomMakesTheShownTextLarger() {
        let usual = shown(text(paragraph(run("text"))), zoom: 1)
        let doubled = shown(text(paragraph(run("text"))), zoom: 2)

        let size = { (text: NSAttributedString) in
            (text.attribute(.font, at: 0, effectiveRange: nil) as? NSFont)?.pointSize
        }
        XCTAssertEqual(size(usual), NSFont.systemFontSize)
        XCTAssertEqual(size(doubled), NSFont.systemFontSize * 2)
    }

    // 段落に触れた位置は、コアが数える表示の文字の数になる。段落の印は数えず、絵文字は1文字と数える（ADR 043）。
    func testATouchInAParagraphCountsTheLettersAsTheCoreDoes() {
        // The shown text is "☐ a😀bc": the mark and its space are not letters of the core
        let shown = shown(text(paragraph(run("a😀"), run("bc"), mark: "☐", start: 5)))

        XCTAssertEqual(ShownText.letter(in: shown, at: 0), 5)
        XCTAssertEqual(ShownText.letter(in: shown, at: 2), 5)
        XCTAssertEqual(ShownText.letter(in: shown, at: 3), 6)
        XCTAssertEqual(ShownText.letter(in: shown, at: 5), 7)
        XCTAssertEqual(ShownText.letter(in: shown, at: 6), 8)
        XCTAssertEqual(ShownText.letter(in: shown, at: 7), 9)
    }

    // 行の末尾に触れた位置は、その行の最後の文字の後ろになる。コードブロックは、その先頭から数える。
    func testATouchAtTheEndOfALineIsAfterItsLastLetterAndCodeCountsFromItsStart() {
        // The shown text is "ab\nlet a\nlet b"
        let shown = shown(text(paragraph(run("ab"))), .code(lines: "let a\nlet b", start: 3))

        XCTAssertEqual(ShownText.letter(in: shown, at: 2), 2)
        XCTAssertEqual(ShownText.letter(in: shown, at: 10), 10)
    }

    // 区切り線だけの項目には、コアの数える文字がない。
    func testARuleHasNoLetterOfTheCore() {
        XCTAssertNil(ShownText.letter(in: shown(text(paragraph(rule: true, start: 0))), at: 2))
    }

    // 本文のない項目は、何も出ない。
    func testAnEmptyItemShowsNothing() {
        XCTAssertEqual(shown().string, "")
    }

    // 中身のないリストの項目も、印だけは出る。
    func testAnEntryWithNothingInItStillShowsItsMark() {
        let list = RichBlock.entries(
            entries: [RichEntry(mark: "1.", blocks: [text(paragraph(run("one")))]), RichEntry(mark: "2.", blocks: [])],
            indent: 0)

        XCTAssertEqual(shown(list).string, "1. one\n2. ")
    }

    private func isBold(_ text: NSAttributedString, at place: Int) -> Bool {
        let font = text.attribute(.font, at: place, effectiveRange: nil) as? NSFont
        return font?.fontDescriptor.symbolicTraits.contains(.bold) ?? false
    }

    // 表のセルは、それぞれ枠に入って出る。見出しの行は太字である。
    func testTheCellsOfATableAreInBoxesAndItsHeaderIsBold() throws {
        let table = RichBlock.table(rows: [
            RichTableRow(header: true, cells: [paragraph(run("Name")), paragraph(run("Age"))]),
            RichTableRow(header: false, cells: [paragraph(run("Ann")), paragraph(run("7"))]),
        ])

        let shown = shown(table)

        XCTAssertEqual(shown.string, "Name\nAge\nAnn\n7")
        let style = try XCTUnwrap(shown.attribute(.paragraphStyle, at: 0, effectiveRange: nil) as? NSParagraphStyle)
        let box = try XCTUnwrap(style.textBlocks.first as? NSTextTableBlock)
        XCTAssertEqual(box.table.numberOfColumns, 2)
        XCTAssertTrue(isBold(shown, at: 0))
        XCTAssertFalse(isBold(shown, at: (shown.string as NSString).range(of: "Ann").location))
    }

    // 行のない表は、何も出ない。
    func testATableWithoutRowsShowsNothing() {
        XCTAssertEqual(shown(.table(rows: [])).string, "")
    }

    // 高さだけを求めた画像は、その高さに合わせ、縦横の比を保つ。
    func testAnImageWithOnlyAHeightKeepsItsShape() {
        let file = NSImage(size: NSSize(width: 100, height: 50))

        let drawn = picture(in: shown(image("assets/a.png", height: 100)) { _ in file })

        XCTAssertEqual(drawn?.size(room: 50), CGSize(width: 200, height: 100))
    }

    private func run(_ text: String, _ change: (inout Looks) -> Void) -> RichRun {
        var looks = Looks.plain
        change(&looks)
        return RichRun(text: text, looks: looks, link: nil)
    }

    // 文字の見た目は、斜体、取り消し線、下線、マーカー、伏せ字、上付き、下付きで出る。上付きと下付きは、小さな文字になる。
    func testTheLooksOfTheLettersAreShown() {
        let shown = shown(
            text(
                paragraph(
                    run("p"), run("i") { $0.italic = true }, run("s") { $0.struck = true },
                    run("u") { $0.underlined = true }, run("m") { $0.marked = true }, run("h") { $0.hidden = true },
                    run("r") { $0.raised = true }, run("l") { $0.lowered = true })))
        func value(_ key: NSAttributedString.Key, of letter: String) -> Any? {
            shown.attribute(key, at: (shown.string as NSString).range(of: letter).location, effectiveRange: nil)
        }
        func font(of letter: String) -> NSFont? { value(.font, of: letter) as? NSFont }
        func offset(of letter: String) -> CGFloat { value(.baselineOffset, of: letter) as? CGFloat ?? 0 }

        XCTAssertEqual(font(of: "i")?.fontDescriptor.symbolicTraits.contains(.italic), true)
        XCTAssertNotNil(value(.strikethroughStyle, of: "s"))
        XCTAssertNotNil(value(.underlineStyle, of: "u"))
        XCTAssertNotNil(value(.backgroundColor, of: "m"))
        XCTAssertNotNil(value(.backgroundColor, of: "h"))
        XCTAssertNotEqual(value(.backgroundColor, of: "m") as? NSColor, value(.backgroundColor, of: "h") as? NSColor)
        XCTAssertGreaterThan(offset(of: "r"), 0)
        XCTAssertLessThan(offset(of: "l"), 0)
        XCTAssertEqual(font(of: "r")?.pointSize, NSFont.systemFontSize * 0.75)
        XCTAssertEqual(font(of: "l")?.pointSize, NSFont.systemFontSize * 0.75)
        for letter in ["p", "i", "s", "u", "m", "h"] {
            XCTAssertEqual(font(of: letter)?.pointSize, NSFont.systemFontSize, letter)
        }
        let looks: [NSAttributedString.Key] = [.strikethroughStyle, .underlineStyle, .backgroundColor, .baselineOffset]
        XCTAssertTrue(looks.allSatisfy { value($0, of: "p") == nil })
        XCTAssertEqual(font(of: "p")?.fontDescriptor.symbolicTraits.contains(.italic), false)
    }
}

/// The texts of the items of a screen, made outside the drawing of SwiftUI.
@MainActor
final class ShownTextsTests: XCTestCase {
    private func shell() -> Shell {
        // Nothing is read from the server: the texts are made from what they are handed
        Shell(server: "http://127.0.0.1:9", url: nil, clipboard: { nil })
    }

    private func rich(_ letters: String) -> RichText {
        RichText(blocks: [
            .text(paragraphs: [
                RichParagraph(
                    heading: 0, scale: 1, depth: 0, quoted: 0, indent: 0, mark: nil, rule: false,
                    runs: [RichRun(text: letters, looks: .plain, link: nil)], start: 0)
            ])
        ])
    }

    /// Lets the main queue run what was put on it before.
    private func nextTurn() async {
        await withCheckedContinuation { done in DispatchQueue.main.async { done.resume() } }
    }

    // 1つだけ頼んだテキストも、描画の外で作られる。
    func testATextAskedForAloneIsMade() async {
        let texts = ShownTexts(shell: shell())

        texts.want("a", rich: rich("one"), zoom: 1)
        await nextTurn()

        XCTAssertEqual(texts.text(of: "a", rich: rich("one"), zoom: 1)?.string, "one")
    }

    // 頼んだテキストは、描画の外で作られる。1回の描画で頼んだ行は、まとめて作られ、1度だけ知らされる。
    func testTheTextsAskedForInOneDrawingAreMadeTogetherAndToldOnce() async {
        let texts = ShownTexts(shell: shell())

        texts.want("a", rich: rich("one"), zoom: 1)
        texts.want("b", rich: rich("two"), zoom: 1)
        XCTAssertNil(texts.text(of: "a", rich: rich("one"), zoom: 1))
        await nextTurn()

        XCTAssertEqual(texts.text(of: "a", rich: rich("one"), zoom: 1)?.string, "one")
        XCTAssertEqual(texts.text(of: "b", rich: rich("two"), zoom: 1)?.string, "two")
        XCTAssertEqual(texts.changes, 1)
    }
}
