import Foundation
import XCTest

/// A journal server for a test, with its files in a folder of its own. It needs the program:
/// the tests that use it are skipped when `PETA_E2E_SERVER` does not name it.
final class JournalServer {
    /// The client that `send` changes things as.
    private static let anotherClient = UUID().uuidString.lowercased()
    let address: String
    private let process = Process()
    private let root: URL

    /// Starts the server on a port that is free, and waits until it answers. `functions` are
    /// the extensions of the server: the `main.js` of each, by its name.
    init(functions: [String: String] = [:]) throws {
        guard let program = ProcessInfo.processInfo.environment["PETA_E2E_SERVER"] else {
            throw XCTSkip("PETA_E2E_SERVER does not name the journal-server program")
        }
        let port = try Self.freePort()
        address = "http://127.0.0.1:\(port)"
        root = FileManager.default.temporaryDirectory
            .appendingPathComponent("peta-swiftui-test-\(UUID().uuidString)")
        let config = root.appendingPathComponent("config")
        let markdown = root.appendingPathComponent("markdown")
        try FileManager.default.createDirectory(at: config, withIntermediateDirectories: true)
        // No markdown folder is named: a throw-away server has `markdown/` under its folder
        let settings = ["weekNumbering": "iso"]
        try JSONSerialization.data(withJSONObject: settings)
            .write(to: config.appendingPathComponent("config.json"))
        for (name, mainJs) in functions {
            let folder = config.appendingPathComponent("functions").appendingPathComponent(name)
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            try JSONSerialization.data(withJSONObject: ["name": name])
                .write(to: folder.appendingPathComponent("config.json"))
            try mainJs.write(to: folder.appendingPathComponent("main.js"), atomically: true, encoding: .utf8)
        }

        process.executableURL = URL(fileURLWithPath: program)
        process.arguments = [
            "--port", String(port),
            // Everything the server writes is under this folder. Without it the server
            // would write into the notes of whoever runs the test (ADR 002)
            "--throwaway-dir", root.path,
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
        try await send("POST", path, json)
    }

    /// Changes something on the server as another client does: the web frontend or a tool.
    /// Throws when the server does not take it.
    func send(_ method: String, _ path: String, _ json: [String: Any]) async throws {
        var request = URLRequest(url: URL(string: address + path)!)
        request.httpMethod = method
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        // A change names its client (ADR 015). This one is none of the shells of the test
        request.setValue(Self.anotherClient, forHTTPHeaderField: "X-Client-Id")
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

    /// A port that nothing listens on now. Why not a port of each test: when a run of the tests
    /// is killed, as the mutation tests do, its server goes on, and the next run would talk to
    /// that server
    private static func freePort() throws -> Int {
        let socket = Darwin.socket(AF_INET, SOCK_STREAM, 0)
        guard socket >= 0 else { throw ServerError.noFreePort }
        defer { close(socket) }
        var address = sockaddr_in()
        address.sin_family = sa_family_t(AF_INET)
        address.sin_addr.s_addr = inet_addr("127.0.0.1")
        // Port 0: the system picks a free one
        address.sin_port = 0
        var size = socklen_t(MemoryLayout<sockaddr_in>.size)
        let found = withUnsafeMutablePointer(to: &address) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                bind(socket, $0, size) == 0 && getsockname(socket, $0, &size) == 0
            }
        }
        guard found else { throw ServerError.noFreePort }
        return Int(UInt16(bigEndian: address.sin_port))
    }

    enum ServerError: Error {
        case noFreePort
        case neverCameUp
        case writesElsewhere(String)
        case refused(String)
    }
}
