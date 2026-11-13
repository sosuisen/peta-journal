import Foundation

/// The Markdown folder of the app: a copy of the pages and their images in the folder in the
/// cloud (ADR 030).
///
/// The cloud does not tell `Notes.md` from `notes.md`, and the file system of a phone does. So
/// a path of the cloud is looked up here without regard to case, and a file never lies here
/// under two spellings.
public final class MarkdownFolder: @unchecked Sendable {
    public static let markdown = ".md"
    /// The folder of the images of the pages, with the `/` after it.
    public static let assets = "assets/"

    private let root: URL
    private let temp: URL
    private let files = FileManager.default
    private let placing = NSLock()

    /// `root` is the folder the journal core takes the files in from. `temp` is where a file
    /// is written before it is whole. It is not under `root`: the core must never take in
    /// half a file.
    public init(root: URL, temp: URL) {
        self.root = root.resolvingSymlinksInPath()
        self.temp = temp
    }

    /// True for a file that the app copies from the cloud: a page, or an image of a page.
    /// `path` is in lower case. The cloud holds more, such as the settings of other apps.
    public static func keeps(_ path: String) -> Bool {
        path.hasSuffix(markdown) || path.hasPrefix(assets)
    }

    private func find(_ path: String) -> URL? {
        findWithoutCase(root: root, path: path)
    }

    /// The path of a file under the root, as it is spelled here.
    private func spelled(_ file: URL) -> String {
        String(file.resolvingSymlinksInPath().path.dropFirst(root.path.count + 1))
    }

    /// Every file under `top`, and `top` itself when it is a file.
    private func filesUnder(_ top: URL) -> [URL] {
        if !isFolder(top) { return [top] }
        let all = files.enumerator(at: top, includingPropertiesForKeys: [.isRegularFileKey])
        return (all?.allObjects as? [URL] ?? []).filter { !isFolder($0) }
    }

    private func isFolder(_ place: URL) -> Bool {
        var folder: ObjCBool = false
        return files.fileExists(atPath: place.path, isDirectory: &folder) && folder.boolValue
    }

    /// Deletes the folders under `top` that hold no file any more, and `top` when it is one.
    private func removeEmptyFolders(from top: URL) {
        guard isFolder(top) else { return }
        for child in (try? files.contentsOfDirectory(at: top, includingPropertiesForKeys: nil)) ?? [] {
            removeEmptyFolders(from: child)
        }
        if (try? files.contentsOfDirectory(atPath: top.path))?.isEmpty == true {
            try? files.removeItem(at: top)
        }
    }

    /// Deletes the file, or the folder with all that is in it, but the files that `keep`
    /// wants: it gets the path of each file as it is spelled here. Nothing there is fine.
    public func remove(_ path: String, keep: (String) -> Bool) {
        guard let found = find(path) else { return }
        for file in filesUnder(found) where !keep(spelled(file)) {
            try? files.removeItem(at: file)
        }
        // A folder that still holds a kept file does not go, and that is right
        removeEmptyFolders(from: found)
    }

    /// The hash of the content of the file, as `hash` makes it: each cloud has its own way.
    /// `nil` when it is not here.
    public func contentHash(_ path: String, by hash: (URL) -> String?) -> String? {
        guard let file = find(path), !isFolder(file) else { return nil }
        return hash(file)
    }

    /// The paths of the Markdown files that are here, as they are spelled here, in order.
    public func pages() -> [String] {
        filesUnder(root)
            .filter { $0.lastPathComponent.lowercased().hasSuffix(MarkdownFolder.markdown) }
            .map(spelled)
            .sorted()
    }

    /// The file at a path that `pages` gave.
    public func file(_ path: String) -> URL {
        root.appendingPathComponent(path)
    }

    /// Puts a file at the path, with the spelling of the path. `write` gets the place to
    /// write its content to. The file appears whole or not at all.
    public func put(_ path: String, write: (URL) throws -> Void) throws {
        try files.createDirectory(at: temp, withIntermediateDirectories: true)
        let part = temp.appendingPathComponent("download-\(UUID().uuidString).part")
        defer { try? files.removeItem(at: part) }
        try write(part)
        try place(part, at: path)
    }

    /// One file at a time: two downloads may need the same new folder.
    private func place(_ part: URL, at path: String) throws {
        placing.lock()
        defer { placing.unlock() }
        var names = path.split(separator: "/").map(String.init)
        let name = names.removeLast()
        var dir = root
        for folder in names {
            if let there = child(of: dir, named: folder) {
                dir = there
            } else {
                dir = dir.appendingPathComponent(folder)
                try files.createDirectory(at: dir, withIntermediateDirectories: false)
            }
        }
        // The same file under another spelling, as after it was renamed in the cloud
        if let old = child(of: dir, named: name) { try files.removeItem(at: old) }
        try files.moveItem(at: part, to: dir.appendingPathComponent(name))
    }

    /// Deletes every file that the app copies and whose path is not in `paths`, which are in
    /// lower case. It is for a list of all that the cloud holds: what is not in it has gone.
    /// A file that `keep` wants stays: it gets the path as it is spelled here.
    public func keepOnly(_ paths: Set<String>, keep: (String) -> Bool) {
        for file in filesUnder(root) {
            let path = spelled(file)
            let key = path.lowercased()
            if MarkdownFolder.keeps(key), !paths.contains(key), !keep(path) {
                try? files.removeItem(at: file)
            }
        }
    }
}

/// The file or folder at the path under `root`, however its name is spelled there. The path of
/// the cloud, and the path that a page writes for its image, may differ from it in case.
public func findWithoutCase(root: URL, path: String) -> URL? {
    path.split(separator: "/").reduce(root as URL?) { dir, name in
        dir.flatMap { child(of: $0, named: String(name)) }
    }
}

private func child(of dir: URL, named name: String) -> URL? {
    let names = (try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []
    return names.first { $0.caseInsensitiveCompare(name) == .orderedSame }.map { dir.appendingPathComponent($0) }
}
