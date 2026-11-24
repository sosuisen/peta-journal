import Foundation
import ImageIO
import PetaShared
import UniformTypeIdentifiers

/// The file that a pasted image is written to. The core saves it among the assets and then
/// deletes it (ADR 071): the image of the clipboard is only data, which Rust does not read.
public enum PastedImage {
    /// Writes `content` into a folder of its own in the temporary folder, and gives the file.
    /// `name` is the name the image came with, and `type` its type, such as `public.png`. The
    /// core takes the name for the text of the link, and the end of the name for that of the
    /// saved file. A HEIC image is written as a PNG or a JPEG: a browser may not show HEIC. Throws when
    /// the file cannot be written.
    public static func write(_ content: Data, named name: String?, type: String?) throws -> URL {
        let (content, name, type) = shownInBrowsers(content, named: name, type: type)
        // A folder of its own: an image of the same name may be pasted before the first is saved
        let folder = FileManager.default.temporaryDirectory
            .appendingPathComponent("pasted", isDirectory: true)
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let file = folder.appendingPathComponent(fileName(name, type))
        try content.write(to: file)
        return file
    }

    /// The image as a browser shows it: a HEIC, as a sticker of the keyboard and Photos give,
    /// becomes a PNG or a JPEG, under the name without its extension. Any other image stays as
    /// it came, and so does a HEIC that cannot be read.
    private static func shownInBrowsers(_ content: Data, named name: String?, type: String?) -> (
        Data, String?, String?
    ) {
        guard let type, let kind = UTType(type), let source = CGImageSourceCreateWithData(content as CFData, nil),
            let (converted, shown) = BrowserImage.fromHeic(source, of: kind)
        else { return (content, name, type) }
        return (converted, name.map { ($0 as NSString).deletingPathExtension }, shown.identifier)
    }

    /// The name of the file: the name the image came with, or "image", and the extension of its
    /// type when the name has none.
    private static func fileName(_ name: String?, _ type: String?) -> String {
        let named = name.flatMap { $0.isEmpty ? nil : $0 } ?? "image"
        guard (named as NSString).pathExtension.isEmpty, let type, let ext = UTType(type)?.preferredFilenameExtension
        else { return named }
        return "\(named).\(ext)"
    }
}
