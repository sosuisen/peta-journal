import UIKit
import XCTest

@testable import PetaJournal

/// When the field of an item takes the keyboard, in a window of the app.
@MainActor
final class FieldKeyboardTests: XCTestCase {
    private var window: UIWindow!

    override func setUp() {
        super.setUp()
        let scene = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first
        window = UIWindow(windowScene: scene!)
        window.frame = CGRect(x: 0, y: 0, width: 320, height: 480)
        window.makeKeyAndVisible()
    }

    override func tearDown() {
        window.isHidden = true
        window = nil
        super.tearDown()
    }

    // 項目の欄は、画面に戻っても、検索欄などほかの欄が持つキーボードを取らない。
    func testAFieldBackOnTheScreenDoesNotTakeTheKeyboardFromAnotherField() {
        let field = ItemEditor.Field()
        let search = UITextField()
        window.addSubview(field)
        window.addSubview(search)
        field.takeTheKeyboard()
        XCTAssertTrue(search.becomeFirstResponder())

        field.removeFromSuperview()
        window.addSubview(field)

        XCTAssertTrue(search.isFirstResponder)
        XCTAssertFalse(field.isFirstResponder)
    }

    // 編集が別の項目へ移るときは、新しい項目の欄が、前の項目の欄からキーボードを取る。
    func testTheFieldOfTheNextEditedItemTakesTheKeyboardFromTheFieldBefore() {
        let before = ItemEditor.Field()
        let next = ItemEditor.Field()
        window.addSubview(before)
        before.takeTheKeyboard()

        next.takeTheKeyboard()
        window.addSubview(next)

        XCTAssertTrue(next.isFirstResponder)
    }
}
