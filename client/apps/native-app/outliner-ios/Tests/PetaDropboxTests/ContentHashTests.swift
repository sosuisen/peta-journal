import CryptoKit
import XCTest

@testable import PetaDropbox

final class ContentHashTests: XCTestCase {
    private var dir: URL!

    override func setUpWithError() throws {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("peta-hash-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try FileManager.default.removeItem(at: dir)
    }

    // 4MBを超えるファイルのハッシュは、4MBごとのハッシュをつないだもののハッシュである。
    func testTheContentHashOfALargeFileIsMadeOfTheHashesOfItsBlocks() throws {
        let block = 4 * 1024 * 1024
        let content = Data((0..<(block + 3)).map { UInt8($0 % 251) })
        let first = sha256(content.prefix(block))
        let second = sha256(content.suffix(from: block))

        XCTAssertEqual(dropboxContentHash(of: try file("large", content)), hex(sha256(first + second)))
    }

    // 4MBまでのファイルのハッシュは、中身のハッシュの、そのまたハッシュである。
    func testTheContentHashOfASmallFileIsTheHashOfTheHashOfItsContent() throws {
        let content = Data("- a".utf8)

        XCTAssertEqual(dropboxContentHash(of: try file("small", content)), hex(sha256(sha256(content))))
    }

    // 空のファイルのハッシュは、空の中身のハッシュである。
    func testTheContentHashOfAnEmptyFileIsTheHashOfNothing() throws {
        XCTAssertEqual(dropboxContentHash(of: try file("empty", Data())), hex(sha256(Data())))
    }

    // 読めないファイルには、ハッシュがない。
    func testAFileThatCannotBeReadHasNoContentHash() {
        XCTAssertNil(dropboxContentHash(of: dir.appendingPathComponent("nowhere")))
    }

    private func file(_ name: String, _ content: Data) throws -> URL {
        let file = dir.appendingPathComponent(name)
        try content.write(to: file)
        return file
    }
}

private func sha256(_ bytes: Data) -> Data {
    Data(SHA256.hash(data: bytes))
}

private func hex(_ bytes: Data) -> String {
    bytes.map { String(format: "%02x", $0) }.joined()
}
