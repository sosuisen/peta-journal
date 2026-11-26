import Foundation
import PetaTypes
import XCTest

/// The examples of the core of the values that cross the border of a native shell (ADR 090 D2).
/// They are in `client/tools/shell-check/common/border-native.json`, which the core writes:
/// every variant of `Changes` and `Event`, as their bytes in hexadecimal.
final class BorderValuesTests: XCTestCase {
    /// One example: a value of the border, as its bytes in hexadecimal.
    private struct Example: Decodable {
        let id: String
        let bytes: String
    }

    private struct File: Decodable {
        let changes: [Example]
        let events: [Example]
    }

    /// The file of the examples, found from this file in the repository.
    private func examplesFile() throws -> File {
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<6 {
            url.deleteLastPathComponent()
        }
        let data = try Data(contentsOf: url.appendingPathComponent("tools/shell-check/common/border-native.json"))
        return try JSONDecoder().decode(File.self, from: data)
    }

    /// The bytes in the hexadecimal `hex`.
    private func bytes(_ hex: String) -> [UInt8] {
        var bytes: [UInt8] = []
        var index = hex.startIndex
        while index < hex.endIndex {
            let next = hex.index(index, offsetBy: 2)
            bytes.append(UInt8(hex[index..<next], radix: 16)!)
            index = next
        }
        return bytes
    }

    /// Reads each example as the shell reads the border, writes it back as the shell writes it,
    /// and expects the bytes it began with. Gives the ids of the examples it ran.
    private func readAndWriteBack(_ examples: [Example], _ readAndWrite: ([UInt8]) throws -> [UInt8]) throws -> [String] {
        var ran: [String] = []
        for example in examples {
            let read = bytes(example.bytes)

            let written = try readAndWrite(read)

            XCTAssertEqual(written, read, example.id)
            ran.append(example.id)
        }
        return ran
    }

    /// Every example of `Changes`, read and written back by the generated types, is the bytes it
    /// was. The examples that ran are all those of the file.
    func testTheShellReadsAndWritesBackEveryExampleOfTheChangesAsItWas() throws {
        let file = try examplesFile()

        let ran = try readAndWriteBack(file.changes) { try Changes.bincodeDeserialize(input: $0).bincodeSerialize() }

        XCTAssertFalse(ran.isEmpty)
        XCTAssertEqual(ran, file.changes.map(\.id))
    }

    /// Every example of `Event`, read and written back by the generated types, is the bytes it
    /// was. The examples that ran are all those of the file.
    func testTheShellReadsAndWritesBackEveryExampleOfTheEventsAsItWas() throws {
        let file = try examplesFile()

        let ran = try readAndWriteBack(file.events) { try Event.bincodeDeserialize(input: $0).bincodeSerialize() }

        XCTAssertFalse(ran.isEmpty)
        XCTAssertEqual(ran, file.events.map(\.id))
    }
}
