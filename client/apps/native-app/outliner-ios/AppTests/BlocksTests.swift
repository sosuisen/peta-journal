import PetaBorder
import UIKit
import XCTest

@testable import PetaJournal

/// How the letters of the parts of an item are drawn.
final class BlocksTests: XCTestCase {
    private let plain = Looks(
        bold: false, italic: false, struck: false, underlined: false, marked: false, code: false, raised: false,
        lowered: false, hidden: false)

    private func paragraph(heading: UInt8, scale: Float) -> RichParagraph {
        RichParagraph(
            heading: heading, scale: scale, depth: 0, quoted: 0, indent: 0, mark: nil, rule: false, runs: [], start: 0)
    }

    // 見出しは、コアが決めた倍率の大きさで、太字で出る。見出しでない段落は、ふつうの大きさである（#795）。
    func testAHeadingIsAsLargeAsTheCoreSaysAndBold() {
        let heading = Paragraphs.font(plain, of: paragraph(heading: 6, scale: 1.5))
        let text = Paragraphs.font(plain, of: paragraph(heading: 0, scale: 1))

        XCTAssertEqual(heading.pointSize, Letters.plain * 1.5, accuracy: 0.001)
        XCTAssertTrue(heading.fontDescriptor.symbolicTraits.contains(.traitBold))
        XCTAssertEqual(text.pointSize, Letters.plain)
        XCTAssertFalse(text.fontDescriptor.symbolicTraits.contains(.traitBold))
    }
}
