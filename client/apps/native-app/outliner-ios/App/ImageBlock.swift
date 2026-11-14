import ImageIO
import PetaBorder
import SwiftUI

/// What is known of the file of an image.
private enum Picture {
    /// Not read yet. Nothing is drawn, so that the alt does not flash before the image.
    case reading
    case missing
    /// `width` is the width of the image in its file, which `image` may be smaller than.
    case here(image: UIImage, width: CGFloat)
}

/// An image, as wide as the page asks, or else as wide as it is in its file. A pixel of the
/// page is one point, as in a browser. It is never wider than the room it has. An image whose
/// file is not here, or cannot be read, shows what stands for it.
struct ImageBlock: View {
    let image: RichImage
    let file: (String) -> URL?

    @State private var picture = Picture.reading
    @State private var large = false

    var body: some View {
        Group {
            switch picture {
            case .reading:
                // Not nothing at all: a view that is not there reads no file
                Color.clear.frame(width: 1, height: 1)
            case .missing:
                Text(image.alt.isEmpty ? image.path : image.alt)
            case .here(let read, let inFile):
                let shape = read.size.width / max(read.size.height, 1)
                // The side that the page does not ask for follows the shape of the image
                let width = image.width.map { CGFloat($0) } ?? image.height.map { CGFloat($0) * shape } ?? inFile
                let height = image.height.map { CGFloat($0) } ?? width / shape
                Image(uiImage: read)
                    .resizable()
                    // A page that asks for both sides may ask for another shape, as in a browser
                    .aspectRatio(width / max(height, 1), contentMode: .fit)
                    .frame(maxWidth: width, alignment: .leading)
                    .padding(.vertical, 4)
                    .accessibilityLabel(image.alt)
                    .accessibilityIdentifier("picture")
                    .onTapGesture { large = true }
                    .fullScreenCover(isPresented: $large) {
                        LargeImage(image: image, small: read, file: file) { large = false }
                    }
            }
        }
        .task(id: image.path) {
            let found = file(image.path)
            let room = await UIScreen.main.nativeBounds.width
            picture = await Task.detached { found.flatMap { read($0, roomPx: room) } ?? .missing }.value
        }
    }
}

/// How many times the width of the screen an image is read for its large view, at least.
private let largeWidths: CGFloat = 2

/// How far two fingers make the large view larger.
private let mostZoom: CGFloat = 8

/// An image over the whole screen, to read what is small in it. Two fingers make it larger,
/// and a finger moves it. A touch closes it.
///
/// The image is read again with more of its pixels. Until then, `small` is shown.
private struct LargeImage: View {
    let image: RichImage
    let small: UIImage
    let file: (String) -> URL?
    let onClose: () -> Void

    @State private var sharp: UIImage?
    @State private var zoom: CGFloat = 1
    @State private var zoomBefore: CGFloat = 1
    @State private var moved = CGSize.zero
    @State private var movedBefore = CGSize.zero

    var body: some View {
        GeometryReader { room in
            Image(uiImage: sharp ?? small)
                .resizable()
                .scaledToFit()
                .scaleEffect(zoom)
                .offset(moved)
                .frame(width: room.size.width, height: room.size.height)
                .contentShape(Rectangle())
                .onTapGesture(perform: onClose)
                .gesture(
                    MagnifyGesture()
                        .onChanged { fingers in
                            zoom = min(max(zoomBefore * fingers.magnification, 1), mostZoom)
                            moved = kept(moved, in: room.size)
                        }
                        .onEnded { _ in zoomBefore = zoom }
                        .simultaneously(
                            with: DragGesture()
                                .onChanged { finger in
                                    let next = CGSize(
                                        width: movedBefore.width + finger.translation.width,
                                        height: movedBefore.height + finger.translation.height)
                                    moved = kept(next, in: room.size)
                                }
                                .onEnded { _ in movedBefore = moved })
                )
                .accessibilityLabel(image.alt)
                .accessibilityIdentifier("large picture")
        }
        .background(Color.black)
        .ignoresSafeArea()
        .task {
            let found = file(image.path)
            let room = await UIScreen.main.nativeBounds.width * largeWidths
            if case .here(let read, _) = await Task.detached(operation: { found.flatMap { read($0, roomPx: room) } }).value {
                sharp = read
            }
        }
    }

    /// The image never leaves the screen: it moves only as far as it is larger.
    private func kept(_ place: CGSize, in room: CGSize) -> CGSize {
        let farX = room.width * (zoom - 1) / 2
        let farY = room.height * (zoom - 1) / 2
        return CGSize(width: min(max(place.width, -farX), farX), height: min(max(place.height, -farY), farY))
    }
}

/// Reads the image of the file. A large image is read smaller, down to no less than `roomPx`
/// wide: a photo at its full size takes the memory of many screens.
private func read(_ file: URL, roomPx: CGFloat) -> Picture? {
    guard let source = CGImageSourceCreateWithURL(file as CFURL, nil),
        let size = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
        let width = size[kCGImagePropertyPixelWidth] as? CGFloat,
        let height = size[kCGImagePropertyPixelHeight] as? CGFloat, width > 0, height > 0
    else { return nil }
    // A photo may lie on its side in its file, with a note of how to turn it
    let turned = (size[kCGImagePropertyOrientation] as? UInt32 ?? 1) > 4
    let shownWidth = turned ? height : width
    let smaller = min(1, roomPx / shownWidth)
    let options: [CFString: Any] = [
        kCGImageSourceCreateThumbnailFromImageAlways: true,
        kCGImageSourceCreateThumbnailWithTransform: true,
        kCGImageSourceThumbnailMaxPixelSize: max(width, height) * smaller,
    ]
    guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else { return nil }
    return .here(image: UIImage(cgImage: image), width: shownWidth)
}
