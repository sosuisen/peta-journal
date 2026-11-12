import CryptoKit
import Foundation
import PetaSync

/// A cloud of a test: the versions of its files, and the changes it tells of at each cursor.
final class FakeCloud: Cloud, @unchecked Sendable {
    private let lock = NSLock()
    /// The content of each version of a file, by how the cloud finds the version.
    private var contents: [String: Data] = [:]
    /// The newest version of each file, by its path in lower case.
    private var newest: [String: CloudFile] = [:]

    /// What `changes` answers, by the cursor it is asked with. "" stands for no cursor.
    var answers: [String: Changes] = [:]
    var askedWith: [String?] = []
    var downloaded: [String] = []
    var uploaded: [String] = []
    var broken: String?
    /// The cloud cannot be reached at all.
    var failing = false

    /// Gives the file of the cloud a new version, and answers with the change that tells of it.
    @discardableResult
    func file(_ path: String, _ text: String) -> Change {
        .file(version(path, Data(text.utf8)))
    }

    private func version(_ path: String, _ content: Data) -> CloudFile {
        lock.lock()
        defer { lock.unlock() }
        let version = CloudFile(path: path, contentHash: hashOf(content), remote: "\(path)#\(contents.count)")
        contents[version.remote] = content
        newest[path.lowercased()] = version
        return version
    }

    /// Takes the file, or the folder with all that is in it, out of the cloud.
    @discardableResult
    func gone(_ path: String) -> Change {
        let key = path.lowercased()
        newest = newest.filter { $0.key != key && !$0.key.hasPrefix(key + "/") }
        return .gone(path: path)
    }

    /// The text of the newest version of the file, or `nil` when the cloud has no such file.
    func text(_ path: String) -> String? {
        newest[path.lowercased()].flatMap { contents[$0.remote] }.map { String(decoding: $0, as: UTF8.self) }
    }

    func changes(since cursor: String?) throws -> Changes {
        if failing { throw NetworkGone() }
        askedWith.append(cursor)
        return answers[cursor ?? ""]!
    }

    func download(_ file: CloudFile, to place: URL) throws {
        if file.path == broken { throw NetworkGone() }
        lock.lock()
        downloaded.append(file.path)
        let content = contents[file.remote]!
        lock.unlock()
        try content.write(to: place)
    }

    func upload(path: String, from file: URL, over: String?) throws -> CloudFile {
        if path == broken { throw NetworkGone() }
        if newest[path.lowercased()]?.remote != over { throw ChangedInCloud(path: path) }
        uploaded.append(path)
        return version(path, try Data(contentsOf: file))
    }
}

struct NetworkGone: Error {}

func sha256(_ bytes: Data) -> Data {
    Data(SHA256.hash(data: bytes))
}

func hex(_ bytes: Data) -> String {
    bytes.map { String(format: "%02x", $0) }.joined()
}

/// The content hash of Dropbox of a content of one block.
func hashOf(_ content: Data) -> String {
    hex(sha256(sha256(content)))
}

/// The folders of one test, deleted after it.
final class TestFolders {
    let dir: URL
    let root: URL

    init() throws {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("peta-sync-\(UUID().uuidString)")
        root = dir.appendingPathComponent("markdown")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    }

    deinit {
        try? FileManager.default.removeItem(at: dir)
    }

    var folder: MarkdownFolder { MarkdownFolder(root: root, temp: dir.appendingPathComponent("temp")) }
    var record: CloudRecord { CloudRecord(file: dir.appendingPathComponent("record")) }
    var cursor: URL { dir.appendingPathComponent("cursor") }

    /// The files under the Markdown folder, by their path, with their text.
    func here() -> [String: String] {
        var found: [String: String] = [:]
        let all = FileManager.default.enumerator(at: root, includingPropertiesForKeys: [.isRegularFileKey])
        let prefix = root.resolvingSymlinksInPath().path + "/"
        for case let file as URL in all ?? NSEnumerator() {
            guard (try? file.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true else { continue }
            let path = String(file.resolvingSymlinksInPath().path.dropFirst(prefix.count))
            found[path] = try? String(contentsOf: file, encoding: .utf8)
        }
        return found
    }

    /// Writes the file of a page, as the core does after an edit in the app.
    func write(_ path: String, _ text: String) throws {
        let file = root.appendingPathComponent(path)
        try FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        try Data(text.utf8).write(to: file)
    }
}
