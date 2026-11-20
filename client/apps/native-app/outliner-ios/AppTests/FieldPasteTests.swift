import UIKit
import XCTest

@testable import PetaJournal

/// What the field of an item takes as a paste.
@MainActor
final class FieldPasteTests: XCTestCase {
    // 項目の欄は、画像の貼り付けを受ける。キーボードのステッカーは、画像の貼り付けとして届くためである。
    func testTheFieldTakesThePasteOfAnImage() {
        let field = ItemEditor.Field()
        let image = UIGraphicsImageRenderer(size: CGSize(width: 1, height: 1)).image { context in
            UIColor.red.setFill()
            context.fill(CGRect(x: 0, y: 0, width: 1, height: 1))
        }

        XCTAssertTrue(field.canPaste([NSItemProvider(object: image)]))
    }
}
