import Foundation
import XCTest

@testable import PetaBorder

/// The file that a pasted image is written to, for the core to save it (ADR 071).
final class PastedImageTests: XCTestCase {
    private var written: [URL] = []

    override func tearDown() {
        for file in written { try? FileManager.default.removeItem(at: file.deletingLastPathComponent()) }
    }

    private func write(named name: String?, type: String?) throws -> URL {
        let file = try PastedImage.write(Self.drawing, named: name, type: type)
        written.append(file)
        return file
    }

    // 貼り付けた画像は、一時フォルダーの自分だけのフォルダーに、来たときの名前で書かれる。
    func testAnImageIsWrittenUnderItsNameInAFolderOfItsOwn() throws {
        let file = try write(named: "サンプル 1.png", type: "public.png")

        XCTAssertEqual(file.lastPathComponent, "サンプル 1.png")
        XCTAssertEqual(try Data(contentsOf: file), Self.drawing)
        XCTAssertEqual(
            try FileManager.default.contentsOfDirectory(atPath: file.deletingLastPathComponent().path),
            ["サンプル 1.png"])
    }

    // 名前のない画像は、種類から拡張子を付けたimage.<拡張子>として書かれる。種類も分からなければ、imageである。
    func testAnImageWithoutANameIsWrittenAsImageWithTheExtensionOfItsType() throws {
        XCTAssertEqual(try write(named: nil, type: "public.png").lastPathComponent, "image.png")
        XCTAssertEqual(try write(named: nil, type: "public.jpeg").lastPathComponent, "image.jpeg")
        XCTAssertEqual(try write(named: "", type: "public.heic").lastPathComponent, "image.heic")
        XCTAssertEqual(try write(named: nil, type: nil).lastPathComponent, "image")
    }

    // 拡張子のない名前には、種類の拡張子が付く。写真の名前（IMG_0001など）には拡張子がなく、コアは保存するファイルの拡張子を名前から取るためである。
    func testANameWithoutAnExtensionGetsThatOfTheType() throws {
        XCTAssertEqual(try write(named: "sample", type: "public.jpeg").lastPathComponent, "sample.jpeg")
        XCTAssertEqual(try write(named: "sample", type: nil).lastPathComponent, "sample")
    }

    // 同じ名前の画像を2回貼っても、別々のファイルに書かれる。前の画像がまだ保存されていなくても、上書きしない。
    func testTwoImagesOfTheSameNameAreWrittenToTwoFiles() throws {
        let first = try write(named: "image.png", type: "public.png")
        let second = try write(named: "image.png", type: "public.png")

        XCTAssertNotEqual(first, second)
        XCTAssertTrue(FileManager.default.fileExists(atPath: first.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: second.path))
    }

    /// The start of a PNG: the content is not read, only copied.
    private static let drawing = Data([0x89, 0x50, 0x4E, 0x47])
}
