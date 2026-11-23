import Foundation
import ImageIO
import UniformTypeIdentifiers

/// An image as a browser shows it. A pasted image goes among the assets, and the web shells
/// show it too: a browser may not show HEIC, as a photo of a phone or a sticker is (ADR 071).
public enum BrowserImage {
    /// The HEIC image in `source`, of the type `kind`: as a PNG when it has transparency, and
    /// as a JPEG when not. `nil` for an image of any other type, which a browser shows as it is,
    /// and for a HEIC that cannot be read.
    public static func fromHeic(_ source: CGImageSource, of kind: UTType) -> (content: Data, type: UTType)? {
        // HEIC is not a kind of HEIF to the system: both are asked for
        guard kind.conforms(to: .heic) || kind.conforms(to: .heif),
            let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
        else { return nil }
        // A photo as a PNG would be many times larger
        let shown: UTType = [.none, .noneSkipFirst, .noneSkipLast].contains(image.alphaInfo) ? .jpeg : .png
        let converted = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(converted, shown.identifier as CFString, 1, nil)
        else { return nil }
        CGImageDestinationAddImage(destination, image, nil)
        guard CGImageDestinationFinalize(destination) else { return nil }
        return (converted as Data, shown)
    }
}
