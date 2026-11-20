import AppKit
import XCTest

@testable import PetaBorder

/// The file that the image on the clipboard is written to, for the core to upload it (ADR 071).
final class PastedImageTests: XCTestCase {
    /// A clipboard of the test's own, so that the clipboard of the user is left alone.
    private var pasteboard: NSPasteboard!
    private var written: [URL] = []

    override func setUp() {
        pasteboard = NSPasteboard(name: NSPasteboard.Name("peta-test-\(UUID().uuidString)"))
        pasteboard.clearContents()
    }

    override func tearDown() {
        pasteboard.releaseGlobally()
        for file in written { try? FileManager.default.removeItem(at: file.deletingLastPathComponent()) }
    }

    private func write() -> URL? {
        let file = PastedImage.write(from: pasteboard)
        if let file { written.append(file) }
        return file
    }

    // ペーストボードのPNGの画像は、一時フォルダーの自分だけのフォルダーに、同じ中身のimage.pngとして書かれる。
    func testAPngImageIsWrittenAsImagePngInAFolderOfItsOwn() throws {
        let drawing = Self.png()
        pasteboard.setData(drawing, forType: .png)

        let file = try XCTUnwrap(write())

        XCTAssertEqual(file.lastPathComponent, "image.png")
        XCTAssertEqual(try Data(contentsOf: file), drawing)
        XCTAssertEqual(
            try FileManager.default.contentsOfDirectory(atPath: file.deletingLastPathComponent().path), ["image.png"])
    }

    // TIFFだけの画像は、PNGに直して書かれる。サーバーの画像として、ブラウザでも見られるためである。
    func testAnImageOnlyInTiffIsWrittenAsPng() throws {
        pasteboard.setData(Self.bitmap().tiffRepresentation, forType: .tiff)

        let file = try XCTUnwrap(write())

        XCTAssertEqual(file.lastPathComponent, "image.png")
        let written = try Data(contentsOf: file)
        XCTAssertEqual([UInt8](written.prefix(4)), [0x89, 0x50, 0x4E, 0x47])
        let pixel = try XCTUnwrap(NSBitmapImageRep(data: written)?.colorAt(x: 0, y: 0)?.usingColorSpace(.deviceRGB))
        XCTAssertEqual(pixel.redComponent, 1, accuracy: 0.01)
        XCTAssertEqual(pixel.greenComponent, 0, accuracy: 0.01)
    }

    /// A picture of one red pixel, drawn by the test.
    static func png() -> Data {
        bitmap().representation(using: .png, properties: [:])!
    }

    private static func bitmap() -> NSBitmapImageRep {
        let bitmap = NSBitmapImageRep(
            bitmapDataPlanes: nil, pixelsWide: 1, pixelsHigh: 1, bitsPerSample: 8, samplesPerPixel: 4,
            hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 4, bitsPerPixel: 32)!
        bitmap.setColor(.red, atX: 0, y: 0)
        return bitmap
    }
}
