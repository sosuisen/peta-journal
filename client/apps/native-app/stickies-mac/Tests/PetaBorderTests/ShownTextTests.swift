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
            heading: heading, scale: 1, depth: 0, quoted: 0, indent: 0, mark: mark, rule: rule, runs: runs, start: start)
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
}
