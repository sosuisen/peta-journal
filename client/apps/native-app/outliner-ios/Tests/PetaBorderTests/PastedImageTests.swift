import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers
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

    // 透明部分のあるHEICの画像（キーボードのステッカー）は、PNGに直して書かれる。HEICを表示できないブラウザがあるためである。透明部分は残る。
    func testAHeicImageWithTransparencyIsWrittenAsPng() throws {
        let file = try PastedImage.write(Self.heic(alpha: 0), named: "sticker.heic", type: "public.heic")
        written.append(file)

        XCTAssertEqual(file.lastPathComponent, "sticker.png")
        let image = try XCTUnwrap(CGImageSourceCreateWithURL(file as CFURL, nil))
        XCTAssertEqual(CGImageSourceGetType(image) as String?, UTType.png.identifier)
        XCTAssertEqual(try Self.alpha(of: file), 0)
    }

    /// A picture of one pixel drawn by the test, written as HEIC as the keyboard and Photos give it.
    private static func heic(alpha: UInt8) throws -> Data {
        // Red, as much as the alpha lets: the colors are multiplied by the alpha
        var pixel = [alpha, 0, 0, alpha]
        let context = try XCTUnwrap(
            CGContext(
                data: &pixel, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4,
                space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: alpha == 255 ? CGImageAlphaInfo.noneSkipLast.rawValue : CGImageAlphaInfo.premultipliedLast.rawValue))
        let image = try XCTUnwrap(context.makeImage())
        let data = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(data, UTType.heic.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }

    /// The alpha of the one pixel of the image in the file, drawn over nothing.
    private static func alpha(of file: URL) throws -> UInt8 {
        let source = try XCTUnwrap(CGImageSourceCreateWithURL(file as CFURL, nil))
        let image = try XCTUnwrap(CGImageSourceCreateImageAtIndex(source, 0, nil))
        var pixel = [UInt8](repeating: 0, count: 4)
        let context = try XCTUnwrap(
            CGContext(
                data: &pixel, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4,
                space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        context.draw(image, in: CGRect(x: 0, y: 0, width: 1, height: 1))
        return pixel[3]
    }

    /// The start of a PNG: the content is not read, only copied.
    private static let drawing = Data([0x89, 0x50, 0x4E, 0x47])
}
