import Foundation

/// What is kept of one sticky between two runs of the app.
public struct StickySettings: Codable, Equatable, Identifiable {
    public var id: String
    /// What the sticky shows, as the URL that opens it. `nil` when it shows no page.
    public var url: String?
    /// The frame of the window, as AppKit counts: from the bottom left of the main screen.
    public var x: Double
    public var y: Double
    public var width: Double
    public var height: Double
    /// The color of the paper, as `#RRGGBB`.
    public var color: String
    /// The window stays above the windows of other apps.
    public var alwaysOnTop: Bool

    public static let yellow = "#FFFDD0"

    public init(
        id: String, url: String?, x: Double, y: Double, width: Double, height: Double,
        color: String = StickySettings.yellow, alwaysOnTop: Bool = false
    ) {
        self.id = id
        self.url = url
        self.x = x
        self.y = y
        self.width = width
        self.height = height
        self.color = color
        self.alwaysOnTop = alwaysOnTop
    }

    /// A file of an older version of the app lacks what was added since.
    public init(from decoder: Decoder) throws {
        let kept = try decoder.container(keyedBy: CodingKeys.self)
        id = try kept.decode(String.self, forKey: .id)
        url = try kept.decodeIfPresent(String.self, forKey: .url)
        x = try kept.decode(Double.self, forKey: .x)
        y = try kept.decode(Double.self, forKey: .y)
        width = try kept.decode(Double.self, forKey: .width)
        height = try kept.decode(Double.self, forKey: .height)
        color = try kept.decodeIfPresent(String.self, forKey: .color) ?? StickySettings.yellow
        alwaysOnTop = try kept.decodeIfPresent(Bool.self, forKey: .alwaysOnTop) ?? false
    }
}

/// The stickies as a file keeps them. Every change is written at once: the app may be ended
/// at any time.
public final class StickyStore {
    public private(set) var stickies: [StickySettings]
    private let file: URL

    /// The file of the app, in the folder of the user for the data of apps.
    public static var standardFile: URL {
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return support.appendingPathComponent("PetaJournal").appendingPathComponent("stickies.json")
    }

    /// Reads the file. A file that is not there, or cannot be read, gives no stickies.
    public init(file: URL) {
        self.file = file
        let kept = try? JSONDecoder().decode([StickySettings].self, from: Data(contentsOf: file))
        stickies = kept ?? []
    }

    /// Adds the sticky, or changes the one with its id.
    public func put(_ sticky: StickySettings) {
        if let place = stickies.firstIndex(where: { $0.id == sticky.id }) {
            if stickies[place] == sticky { return }
            stickies[place] = sticky
        } else {
            stickies.append(sticky)
        }
        write()
    }

    public func remove(_ id: String) {
        stickies.removeAll { $0.id == id }
        write()
    }

    /// A failure is not told: the stickies on screen go on, and the next change writes again.
    private func write() {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        try? FileManager.default.createDirectory(
            at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        try? encoder.encode(stickies).write(to: file, options: .atomic)
    }
}
