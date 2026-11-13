import Foundation
import XCTest

/// A journal server for a test, with its files in a folder of its own. It needs the program:
/// the tests that use it are skipped when `PETA_E2E_SERVER` does not name it.
final class JournalServer {
    let address: String
    private let process = Process()
    private let root: URL

    /// Starts the server and waits until it answers.
    init(port: Int) throws {
        guard let program = ProcessInfo.processInfo.environment["PETA_E2E_SERVER"] else {
            throw XCTSkip("PETA_E2E_SERVER does not name the journal-server program")
        }
        address = "http://127.0.0.1:\(port)"
        root = FileManager.default.temporaryDirectory
            .appendingPathComponent("peta-swiftui-test-\(UUID().uuidString)")
        let config = root.appendingPathComponent("config")
        let markdown = root.appendingPathComponent("markdown")
        try FileManager.default.createDirectory(at: config, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: markdown, withIntermediateDirectories: true)
        // Without this the server would write into the notes of whoever runs the test
        let settings = ["markdownDir": markdown.path, "weekNumbering": "iso"]
        try JSONSerialization.data(withJSONObject: settings)
            .write(to: config.appendingPathComponent("config.json"))

        process.executableURL = URL(fileURLWithPath: program)
        process.arguments = [
            "--port", String(port),
            "--data-dir", root.appendingPathComponent("data").path,
            "--index-dir", root.appendingPathComponent("index").path,
            "--config-dir", config.path,
        ]
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        try process.run()
        try waitUntilUp(markdown: markdown)
    }

    private func waitUntilUp(markdown: URL) throws {
        let deadline = Date().addingTimeInterval(20)
        while Date() < deadline {
            if let config = try? get("/api/config"),
                let json = try? JSONSerialization.jsonObject(with: config) as? [String: Any],
                let inUse = json["markdownDir"] as? String
            {
                let here = markdown.resolvingSymlinksInPath().path
                let there = URL(fileURLWithPath: inUse).resolvingSymlinksInPath().path
                guard there == here else {
                    stop()
                    throw ServerError.writesElsewhere(inUse)
                }
                return
            }
            Thread.sleep(forTimeInterval: 0.05)
        }
        stop()
        throw ServerError.neverCameUp
    }

    /// The body of the answer to a GET.
    func get(_ path: String) throws -> Data {
        try Data(contentsOf: URL(string: address + path)!)
    }

    /// Sends JSON with a POST and waits for the answer.
    func post(_ path: String, _ json: [String: Any]) async throws {
        var request = URLRequest(url: URL(string: address + path)!)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        // The server refuses a change that does not say which client made it
        request.setValue(UUID().uuidString.lowercased(), forHTTPHeaderField: "X-Client-Id")
        request.httpBody = try JSONSerialization.data(withJSONObject: json)
        let (_, answer) = try await URLSession.shared.data(for: request)
        let status = (answer as? HTTPURLResponse)?.statusCode ?? 0
        guard (200..<300).contains(status) else { throw ServerError.refused("\(path): \(status)") }
    }

    func stop() {
        if process.isRunning {
            process.terminate()
            process.waitUntilExit()
        }
        try? FileManager.default.removeItem(at: root)
    }

    enum ServerError: Error {
        case neverCameUp
        case writesElsewhere(String)
        case refused(String)
    }
}
