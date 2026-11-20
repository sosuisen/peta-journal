import UIKit
import XCTest

@testable import PetaJournal

/// What the field of an item takes from the keyboard besides text.
@MainActor
final class FieldPasteTests: XCTestCase {
    // 項目の欄は、キーボードのステッカーを受けない。文字の中の絵としても、画像の貼り付けとしても受けない。本文はプレーンテキストのMarkdownで、文字の中の絵を表せないためである。
    func testTheFieldTakesNoStickerOfTheKeyboard() {
        let field = ItemEditor.Field()
        let image = UIGraphicsImageRenderer(size: CGSize(width: 1, height: 1)).image { context in
            UIColor.red.setFill()
            context.fill(CGRect(x: 0, y: 0, width: 1, height: 1))
        }

        if #available(iOS 18.0, *) { XCTAssertFalse(field.supportsAdaptiveImageGlyph) }
        XCTAssertFalse(field.canPaste([NSItemProvider(object: image)]))
    }
}
