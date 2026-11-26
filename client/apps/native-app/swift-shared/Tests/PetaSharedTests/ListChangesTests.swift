import Foundation
import PetaTypes
import XCTest

@testable import PetaShared

/// The examples of the core that every shell applies as the core does (ADR 089, ADR 090 D1).
/// They are in `client/tools/shell-check/common/list-changes.json`, which the core writes.
final class ListChangesTests: XCTestCase {
    /// One example, as the bytes of `Changes` that cross the border of a native shell.
    private struct Example: Decodable {
        let id: String
        let before: String
        let changes: String
        let after: String
    }

    private struct Part: Decodable {
        let rows: [Example]
    }

    private struct File: Decodable {
        let native: Part
    }

    /// The file of the examples, found from this file in the repository.
    private func examplesFile() -> URL {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<6 {
            url.deleteLastPathComponent()
        }
        return url.appendingPathComponent("tools/shell-check/common/list-changes.json")
    }

    /// The changes in the hexadecimal `hex`, read as a native shell reads them from the core.
    private func changes(_ hex: String) throws -> Changes {
        var bytes: [UInt8] = []
        var index = hex.startIndex
        while index < hex.endIndex {
            let next = hex.index(index, offsetBy: 2)
            bytes.append(UInt8(hex[index..<next], radix: 16)!)
            index = next
        }
        return try Changes.bincodeDeserialize(input: bytes)
    }

    /// Every example of the core, applied as the shell applies the changes, ends in the rows of
    /// the core. The examples that ran are all those of the file.
    func testTheShellAppliesEveryExampleOfTheCoreAsTheCoreDoes() throws {
        let file = try JSONDecoder().decode(File.self, from: Data(contentsOf: examplesFile()))
        var ran: [String] = []

        for example in file.native.rows {
            let before = [Row]().applying(try changes(example.before).rows)
            let after = [Row]().applying(try changes(example.after).rows)

            XCTAssertEqual(before.applying(try changes(example.changes).rows), after, example.id)
            ran.append(example.id)
        }

        XCTAssertFalse(ran.isEmpty)
        XCTAssertEqual(ran, file.native.rows.map(\.id))
    }
}
