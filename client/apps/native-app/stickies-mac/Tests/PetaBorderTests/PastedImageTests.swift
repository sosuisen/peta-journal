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
        XCTAssertEqual(try Self.pixel(of: written), [255, 0, 0, 255])
    }

    // Finderでコピーした画像のファイルは、同じ名前と中身で一時ファイルに写される。元のファイルは残る。コアが保存のあとに消すのは、写したほうである。
    func testAnImageFileCopiedInTheFinderIsCopiedWithItsNameAndTheOriginalStays() throws {
        let original = try file(named: "サンプル 1.png", content: Self.png())
        pasteboard.writeObjects([original as NSURL])

        let file = try XCTUnwrap(write())

        XCTAssertEqual(file.lastPathComponent, "サンプル 1.png")
        XCTAssertNotEqual(file, original)
        XCTAssertEqual(try Data(contentsOf: file), Self.png())
        XCTAssertTrue(FileManager.default.fileExists(atPath: original.path))
    }

    // 文字もあるペーストボードでは、画像を書かない。Officeのように、文字に画像も添えるアプリがあり、貼りたいのは文字のほうである。
    func testNoImageIsWrittenWhenThePasteboardHasTextToo() {
        pasteboard.declareTypes([.string, .png, .tiff], owner: nil)
        pasteboard.setString("sample", forType: .string)
        pasteboard.setData(Self.png(), forType: .png)
        pasteboard.setData(Self.bitmap().tiffRepresentation, forType: .tiff)

        XCTAssertNil(write())
    }

    // 画像でないファイルのコピーでは、画像を書かない。Finderはファイルのアイコンを画像として添えるが、それは貼りたいものではない。
    func testNoImageIsWrittenForACopiedFileThatIsNotAnImage() throws {
        let text = try file(named: "sample.txt", content: Data("sample".utf8))
        pasteboard.clearContents()
        pasteboard.setString(text.absoluteString, forType: .fileURL)
        pasteboard.addTypes([.tiff], owner: nil)
        pasteboard.setData(Self.bitmap().tiffRepresentation, forType: .tiff)

        XCTAssertNil(write())
    }

    /// A file of the test, in a folder that the test deletes at its end.
    private func file(named name: String, content: Data) throws -> URL {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("peta-test-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let file = folder.appendingPathComponent(name)
        try content.write(to: file)
        written.append(file)
        return file
    }

    /// A picture of one red pixel, drawn by the test.
    static func png() -> Data {
        bitmap().representation(using: .png, properties: [:])!
    }

    /// The red, green, blue and alpha of the one pixel of an image, as they are stored.
    private static func pixel(of image: Data) throws -> [Int] {
        let bitmap = try XCTUnwrap(NSBitmapImageRep(data: image))
        var pixel = [Int](repeating: 0, count: bitmap.samplesPerPixel)
        bitmap.getPixel(&pixel, atX: 0, y: 0)
        return pixel
    }

    private static func bitmap() -> NSBitmapImageRep {
        let bitmap = NSBitmapImageRep(
            bitmapDataPlanes: nil, pixelsWide: 1, pixelsHigh: 1, bitsPerSample: 8, samplesPerPixel: 4,
            hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 4, bitsPerPixel: 32)!
        bitmap.setColor(NSColor(deviceRed: 1, green: 0, blue: 0, alpha: 1), atX: 0, y: 0)
        return bitmap
    }
}
