import Foundation
import PetaCore

/// What is kept of one sticky between two runs of the app. The rules are in Rust, in
/// `client-stickies`, for the stickies of macOS and Windows.
public typealias StickySettings = PetaSticky

extension PetaSticky: Identifiable {
    /// The color of the paper of a new sticky, as `#RRGGBB`.
    public static let yellow = stickyYellow()
}

/// The stickies as a file keeps them. Every change is written at once: the app may be ended
/// at any time. `client-stickies` reads and writes the file.
public final class StickyStore {
    private let file: PetaStickyStore

    /// The file of the app, in the folder of the user for the data of apps.
    public static var standardFile: URL {
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return support.appendingPathComponent("PetaStickies").appendingPathComponent("stickies.json")
    }

    /// Reads the file. A file that is not there, or cannot be read, gives no stickies.
    public init(file: URL) {
        self.file = PetaStickyStore(file: file.path)
    }

    public var stickies: [StickySettings] {
        file.stickies()
    }

    /// Adds the sticky, or changes the one with its id.
    public func put(_ sticky: StickySettings) {
        file.put(sticky: sticky)
    }

    public func remove(_ id: String) {
        file.remove(id: id)
    }
}
