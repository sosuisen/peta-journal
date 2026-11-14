import AppKit
import Combine
import PetaCore

/// The images of the assets folder, read from the journal server, which serves the folder at
/// `/assets/`. An image is asked for when a text is made, and the text is made again when the
/// image has come: reading it while the text is made would stop the window.
@MainActor
public final class AssetImages: ObservableObject {
    /// Counts up whenever an image has come.
    @Published public private(set) var arrived = 0
    private let server: String
    private let fetch: (URL, @escaping @Sendable (Data?) -> Void) -> Void
    /// The images that came. An image that cannot be read is kept as `nil`: it is not asked
    /// for again.
    private var images: [String: NSImage?] = [:]

    /// `server` is the address of the journal server. `fetch` reads the bytes of an address
    /// and hands them over on any thread, or `nil` when they cannot be read.
    public init(
        server: String,
        fetch: @escaping (URL, @escaping @Sendable (Data?) -> Void) -> Void = { url, done in
            URLSession.shared.dataTask(with: url) { data, response, _ in
                done((response as? HTTPURLResponse)?.statusCode == 200 ? data : nil)
            }.resume()
        }
    ) {
        self.server = server
        self.fetch = fetch
    }

    /// The image of the file at `path` under the Markdown folder, such as `assets/picture.png`,
    /// when it has come. The first time it is asked for, it is read.
    public func image(_ path: String) -> NSImage? {
        if let known = images[path] { return known }
        images[path] = .some(nil)
        guard let url = address(of: path) else { return nil }
        fetch(url) { data in
            DispatchQueue.main.async {
                MainActor.assumeIsolated {
                    self.images[path] = data.flatMap(NSImage.init(data:))
                    self.arrived += 1
                }
            }
        }
        return nil
    }

    /// Where the journal server serves the file at `path` under the Markdown folder.
    public func address(of path: String) -> URL? {
        URL(string: assetAddress(appUrl: server, path: path))
    }
}
