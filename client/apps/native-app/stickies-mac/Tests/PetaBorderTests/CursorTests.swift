import PetaBorder
import XCTest

final class CursorTests: XCTestCase {
    // コアはカーソルを文字（Unicodeスカラー）で数える。絵文字はUTF-16では2つだが、1文字である。
    func testACursorOfAppKitIsCountedInCharactersForTheCore() {
        XCTAssertEqual(Cursor.characters(utf16: 0, in: "日本😀語"), 0)
        XCTAssertEqual(Cursor.characters(utf16: 2, in: "日本😀語"), 2)
        XCTAssertEqual(Cursor.characters(utf16: 4, in: "日本😀語"), 3)
        XCTAssertEqual(Cursor.characters(utf16: 5, in: "日本😀語"), 4)
    }

    // コアのカーソルは、UTF-16の位置に戻せる。本文より先の位置は、末尾になる。
    func testACursorOfTheCoreIsCountedInUTF16ForAppKit() {
        XCTAssertEqual(Cursor.utf16(characters: 0, in: "日本😀語"), 0)
        XCTAssertEqual(Cursor.utf16(characters: 3, in: "日本😀語"), 4)
        XCTAssertEqual(Cursor.utf16(characters: 4, in: "日本😀語"), 5)
        XCTAssertEqual(Cursor.utf16(characters: UInt64(Int.max), in: "日本😀語"), 5)
    }
}
