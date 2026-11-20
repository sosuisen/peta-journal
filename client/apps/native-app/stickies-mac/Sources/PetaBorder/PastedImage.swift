import AppKit
import Foundation

/// The file that the image on the clipboard is written to. The core uploads it to the journal
/// server and then deletes it (ADR 071).
public enum PastedImage {
    /// Writes the image on `pasteboard` to a file in a folder of its own, and gives the file.
    /// `nil` when the pasteboard holds no image.
    public static func write(from pasteboard: NSPasteboard) -> URL? {
        guard let png = pasteboard.data(forType: .png) ?? tiffAsPng(pasteboard) else { return nil }
        // A folder of its own: an image of the same name may be pasted before the first is saved
        let folder = FileManager.default.temporaryDirectory
            .appendingPathComponent("peta-pasted", isDirectory: true)
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        let file = folder.appendingPathComponent("image.png")
        do {
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            try png.write(to: file)
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
