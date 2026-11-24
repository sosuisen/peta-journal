import AppKit
import XCTest

@testable import PetaBorder

/// The images of the pages, read from the journal server. The server is a fake here: the bytes
/// come when the test hands them over.
@MainActor
final class AssetImagesTests: XCTestCase {
    private var asked: [URL] = []
    private var answers: [@Sendable (Data?) -> Void] = []

    private func images() -> AssetImages {
        AssetImages(server: "http://127.0.0.1:8080") { [weak self] url, done in
            self?.asked.append(url)
            self?.answers.append(done)
        }
    }

    /// The bytes of a PNG image.
    private func png() throws -> Data {
        let bitmap = try XCTUnwrap(
            NSBitmapImageRep(
                bitmapDataPlanes: nil, pixelsWide: 2, pixelsHigh: 1, bitsPerSample: 8, samplesPerPixel: 4,
                hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
        return try XCTUnwrap(bitmap.representation(using: .png, properties: [:]))
    }

    private func waitUntil(_ what: String, _ done: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(5)
        while !done() {
            if Date() > deadline {
                XCTFail("never happened: \(what)")
                throw CancellationError()
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    // 画像は、journalサーバーの`/assets/`から読む。届くまでは、ないものとして答える。
    func testAnImageIsReadFromTheServerAndIsThereOnceItCame() async throws {
        let images = images()

        XCTAssertNil(images.image("assets/my picture.png"))
        XCTAssertEqual(asked, [URL(string: "http://127.0.0.1:8080/assets/my%20picture.png")!])

        answers[0](try png())
        try await waitUntil("the image came") { images.arrived == 1 }
        XCTAssertEqual(images.image("assets/my picture.png")?.size, NSSize(width: 2, height: 1))
        XCTAssertEqual(asked.count, 1)
    }

    // 読めなかった画像は、もう一度は読みにいかない。
    func testAnImageThatCannotBeReadIsNotAskedForAgain() async throws {
        let images = images()
        _ = images.image("assets/gone.png")

        answers[0](nil)
        try await waitUntil("the answer came") { images.arrived == 1 }

        XCTAssertNil(images.image("assets/gone.png"))
        XCTAssertEqual(asked.count, 1)
    }

    // 本物のjournalサーバーからは、assetsフォルダーにある画像が届く。ない画像は、ないものとして届く。
    func testTheJournalServerHandsOverTheImagesOfTheAssetsFolder() async throws {
        let server = try JournalServer()
        defer { server.stop() }
        let assets = server.markdown.appendingPathComponent("assets")
        try FileManager.default.createDirectory(at: assets, withIntermediateDirectories: true)
        try png().write(to: assets.appendingPathComponent("here.png"))
        let images = AssetImages(server: server.address)

        _ = images.image("assets/here.png")
        _ = images.image("assets/gone.png")

        try await waitUntil("both answers came") { images.arrived == 2 }
        XCTAssertEqual(images.image("assets/here.png")?.size, NSSize(width: 2, height: 1))
        XCTAssertNil(images.image("assets/gone.png"))
    }
}
