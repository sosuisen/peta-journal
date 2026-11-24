import ImageIO
import UniformTypeIdentifiers
import XCTest

@testable import PetaShared

/// The images that a browser may not show are made into ones it shows (ADR 071). Each app also
/// tests it through its own `PastedImage`. These tests are here so that the mutation tests of
/// this package see the code.
final class BrowserImageTests: XCTestCase {
    // 透明部分のあるHEICの画像は、PNGになる。
    func testAHeicImageWithTransparencyBecomesAPng() throws {
        let made = try XCTUnwrap(BrowserImage.fromHeic(Self.heic(alpha: 0), of: .heic))

        XCTAssertEqual(made.type, .png)
        XCTAssertEqual(Self.type(of: made.content), .png)
    }

    // 透明部分のないHEICの画像は、JPEGになる。
    func testAHeicImageWithoutTransparencyBecomesAJpeg() throws {
        let made = try XCTUnwrap(BrowserImage.fromHeic(Self.heic(alpha: 255), of: .heic))

        XCTAssertEqual(made.type, .jpeg)
        XCTAssertEqual(Self.type(of: made.content), .jpeg)
    }

    // HEIFの画像も、HEICと同じく直される。システムは、HEICをHEIFの一種とは見ないためである。
    func testAHeifImageIsMadeAsAHeicImageIs() throws {
        let made = try XCTUnwrap(BrowserImage.fromHeic(Self.heic(alpha: 255), of: .heif))

        XCTAssertEqual(made.type, .jpeg)
    }

    // HEICでない画像は、ブラウザがそのまま表示できるので、直さない。
    func testAnImageOfAnotherTypeIsNotMade() throws {
        XCTAssertNil(BrowserImage.fromHeic(try Self.heic(alpha: 0), of: .png))
    }

    /// A picture of one pixel drawn by the test, written as HEIC as the keyboard and Photos give it.
    private static func heic(alpha: UInt8) throws -> CGImageSource {
        // Red, as much as the alpha lets: the colors are multiplied by the alpha
        var pixel = [alpha, 0, 0, alpha]
        let context = try XCTUnwrap(
            CGContext(
                data: &pixel, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4,
                space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: alpha == 255
                    ? CGImageAlphaInfo.noneSkipLast.rawValue : CGImageAlphaInfo.premultipliedLast.rawValue))
        let image = try XCTUnwrap(context.makeImage())
        let data = NSMutableData()
        let destination = try XCTUnwrap(
            CGImageDestinationCreateWithData(data, UTType.heic.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return try XCTUnwrap(CGImageSourceCreateWithData(data, nil))
    }

    private static func type(of content: Data) -> UTType? {
        CGImageSourceCreateWithData(content as CFData, nil)
            .flatMap { CGImageSourceGetType($0) }
            .flatMap { UTType($0 as String) }
    }
}
