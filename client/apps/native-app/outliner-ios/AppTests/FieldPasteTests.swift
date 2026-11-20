import UIKit
import XCTest

@testable import PetaJournal

/// What the field of an item takes from the keyboard besides text.
@MainActor
final class FieldPasteTests: XCTestCase {
    // 項目の欄は、キーボードのステッカーを文字の中の絵としては受けない。本文はプレーンテキストのMarkdownで、絵の代わりの文字（U+FFFC）だけが残るためである。キーボードは、ステッカーを画像の貼り付けとして送ってくる。
    func testTheFieldTakesNoStickerAsAPictureInTheText() throws {
        guard #available(iOS 18.0, *) else { throw XCTSkip("Stickers come into the text from iOS 18") }

        XCTAssertFalse(ItemEditor.Field().supportsAdaptiveImageGlyph)
    }
}
