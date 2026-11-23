import AppKit
import Foundation
import ImageIO
import UniformTypeIdentifiers

/// The file that the image on the clipboard is written to. The core uploads it to the journal
/// server and then deletes it (ADR 071).
public enum PastedImage {
    /// Writes the image on `pasteboard` to a file in a folder of its own, and gives the file.
    /// `nil` when the pasteboard holds no image, or text or another file with its image.
    public static func write(from pasteboard: NSPasteboard) -> URL? {
        if let image = imageFile(pasteboard) {
            // A HEIC, as a photo of a phone, is written as a PNG or a JPEG: a browser may not show it
            if let (content, ext) = shownInBrowsers(image) {
                let name = image.deletingPathExtension().appendingPathExtension(ext).lastPathComponent
                return inFolderOfItsOwn(named: name) { try content.write(to: $0) }
            }
            // A copy: the core deletes the file it was given, and the original is the user's
            return inFolderOfItsOwn(named: image.lastPathComponent) {
                try FileManager.default.copyItem(at: image, to: $0)
            }
        }
        // An app such as Word adds a picture of the text it copied: the text is what is meant.
        // The Finder adds the icon of a file that is not an image, which is not meant either
        guard pasteboard.string(forType: .string) == nil, pasteboard.availableType(from: [.fileURL]) == nil,
            let png = pasteboard.data(forType: .png) ?? tiffAsPng(pasteboard)
        else { return nil }
        return inFolderOfItsOwn(named: "image.png") { try png.write(to: $0) }
    }

    /// The image file that the Finder copied, if any.
    private static func imageFile(_ pasteboard: NSPasteboard) -> URL? {
        let options: [NSPasteboard.ReadingOptionKey: Any] = [
            .urlReadingFileURLsOnly: true, .urlReadingContentsConformToTypes: ["public.image"],
        ]
        return (pasteboard.readObjects(forClasses: [NSURL.self], options: options) as? [URL])?.first
    }

    /// The HEIC image in `file` as a PNG when it has transparency, and a JPEG when not, with
    /// the extension of that type. `nil` for any other image, and for a HEIC that cannot be read.
    private static func shownInBrowsers(_ file: URL) -> (Data, String)? {
        // HEIC is not a kind of HEIF to the system: both are asked for
        guard let kind = UTType(filenameExtension: file.pathExtension),
            kind.conforms(to: .heic) || kind.conforms(to: .heif),
            let source = CGImageSourceCreateWithURL(file as CFURL, nil),
            let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
        else { return nil }
        // A photo as a PNG would be many times larger
        let shown: UTType = [.none, .noneSkipFirst, .noneSkipLast].contains(image.alphaInfo) ? .jpeg : .png
        let converted = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(converted, shown.identifier as CFString, 1, nil)
        else { return nil }
        CGImageDestinationAddImage(destination, image, nil)
        guard CGImageDestinationFinalize(destination), let ext = shown.preferredFilenameExtension else { return nil }
        return (converted as Data, ext)
    }

    /// Makes the file `name` in a folder of its own, and gives it. `nil` when it cannot be made.
    private static func inFolderOfItsOwn(named name: String, _ make: (URL) throws -> Void) -> URL? {
        // A folder of its own: an image of the same name may be pasted before the first is saved
        let folder = FileManager.default.temporaryDirectory
            .appendingPathComponent("peta-pasted", isDirectory: true)
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        let file = folder.appendingPathComponent(name)
        do {
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            try make(file)
        } catch {
            return nil
        }
        return file
    }

    /// The image of a TIFF as a PNG. A screenshot and many apps give only a TIFF, which a
    /// browser does not show.
    private static func tiffAsPng(_ pasteboard: NSPasteboard) -> Data? {
        guard let tiff = pasteboard.data(forType: .tiff) else { return nil }
        return NSBitmapImageRep(data: tiff)?.representation(using: .png, properties: [:])
    }
}
