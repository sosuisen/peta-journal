import UIKit
import XCTest

@testable import PetaJournal

/// Which letter of a shown text a touch falls on, as TextKit lays the letters out.
final class TouchesTests: XCTestCase {
    private let font = UIFont.systemFont(ofSize: 17)

    private func text(_ letters: String) -> NSAttributedString {
        NSAttributedString(string: letters, attributes: [.font: font])
    }

    /// How wide the letters are in one line.
    private func width(_ letters: String) -> CGFloat {
        text(letters).size().width
    }

    // 文字の左端に触れると、その前に文字はない。
    func testATouchAtTheLeftEdgeHasNoLetterBeforeIt() {
        XCTAssertEqual(Touches.letter(at: CGPoint(x: 0, y: 5), in: text("hello"), width: 300), 0)
    }

    // 行の右に触れると、その行の文字がすべて前にある。
    func testATouchRightOfTheLineHasAllItsLettersBeforeIt() {
        XCTAssertEqual(Touches.letter(at: CGPoint(x: 250, y: 5), in: text("hello"), width: 300), 5)
    }

    // 文字の右半分に触れると、その文字の後ろになる。
    func testATouchOnTheRightHalfOfALetterIsAfterIt() {
        let x = width("he") + width("l") * 0.8
        XCTAssertEqual(Touches.letter(at: CGPoint(x: x, y: 5), in: text("hello"), width: 300), 3)
    }

    // 折り返した2行目の先頭に触れると、1行目の文字が前にある。
    func testATouchAtTheStartOfTheSecondLineHasTheFirstLineBeforeIt() {
        let first = "aaaa "
        let letters = first + "bbbb"
        let y = font.lineHeight * 1.5
        XCTAssertEqual(Touches.letter(at: CGPoint(x: 0, y: y), in: text(letters), width: width("aaaa b") - 1), first.count)
    }

    // 文字は、コアと同じくUnicodeのスカラーで数える。
    func testTheLettersAreCountedAsTheCoreCountsThem() {
        XCTAssertEqual(Touches.letter(at: CGPoint(x: 250, y: 5), in: text("😀ab"), width: 300), 3)
    }

    // 項目の中では、触れた文字の前に、その前の文の文字の数が足される。文の脇に触れると、いちばん近い文の行になる。
    func testInAnItemTheLettersOfTheTextsBeforeAreAdded() {
        let lineHeight = font.lineHeight
        let first = LetterTarget(start: 0, frame: CGRect(x: 0, y: 0, width: 300, height: lineHeight), text: text("one"), wraps: true)
        let second = LetterTarget(start: 4, frame: CGRect(x: 0, y: lineHeight + 4, width: 300, height: lineHeight), text: text("two"), wraps: true)

        XCTAssertEqual(Touches.letter(at: CGPoint(x: 0, y: lineHeight + 6), in: [first, second]), 4)
        XCTAssertEqual(Touches.letter(at: CGPoint(x: 350, y: lineHeight + 6), in: [first, second]), 7)
        XCTAssertNil(Touches.letter(at: .zero, in: []))
    }
}
