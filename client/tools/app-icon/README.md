# app-icon

Makes the icons of the apps from an SVG: that of a Windows app, of an iOS app and of a macOS
app. The apps get their icons from it through a script in their folders:

| App | Script | Icon |
|---|---|---|
| `stickies-win` | `icon.ps1`, `icon.sh` | `PetaStickies/PetaStickies.ico` |
| `outliner-ios` | `icon.sh` | `App/Assets.xcassets/AppIcon.appiconset/icon-1024.png` |
| `stickies-mac` | `icon.sh` | `App/Assets.xcassets/AppIcon.appiconset` |

```
cd peta-journal/client
cargo run --release -p app-icon -- windows <svg> <ico>
cargo run --release -p app-icon -- ios <svg> <png>
cargo run --release -p app-icon -- macos <svg> <appiconset>
```

## Why Rust

The apps are built with Rust anyway. A tool such as resvg or ImageMagick would have to be
installed only to make the icons. The crate uses resvg, ico and png as libraries, so `cargo`
brings them.

## How an icon is made

Every icon starts the same way. The SVG is read. Its drawing need not be in the middle of its
page: the empty edges are left out. The outline of the drawing, with its strokes, says where
they are. Then the drawing is drawn into a square, in the middle, as large as fits.

A text in the SVG is not drawn: the crate is built without fonts. The drawing of the project
has no text.

### Windows

1. For each size, the drawing is drawn into a clear square. It fills the whole square: a
   tall drawing reaches from top to bottom.
2. The squares go into one `.ico`, from the largest to the smallest.

The sizes are 256, 128, 96, 64, 48, 40, 32, 24, 20 and 16 pixels. Windows picks the one that
fits the place and the scale of the screen.

Each size is drawn from the SVG itself, not made smaller from a large image. A small image
that is made from a large one is blurred.

The `ico` crate stores each image as PNG, as the edges of the drawing are partly clear. PNG
keeps the file small: the icon of the stickies is about 8KB. Windows reads icons with PNG
images.

### iOS

1. The drawing is drawn into a white square of 1024 pixels. It fits into 640 pixels in the
   middle: the white around it is the margin that the icons of iOS have.
2. The square is written as a PNG without an alpha channel. iOS takes no icon that has one,
   also when every pixel is opaque.

Xcode makes every other size from this one image.

The icon looks as that of the Android shell: the notebook in the middle of a white square.

### macOS

1. For each size, a white plate with rounded corners is drawn into a clear square. On a side
   of 1024 pixels the plate is 824 pixels, 100 in from each edge, as in the template of Apple
   for the icons of macOS. Its corners have a radius of 185.4 pixels.
2. The drawing is drawn on the plate, in the middle. It fits into 515 pixels, five eighths of
   the plate, as on iOS.
3. Each square is written as a PNG with alpha: the corners and the edge around the plate are
   clear.
4. `Contents.json` names the images: each size of 16, 32, 128, 256 and 512 points, at one and
   at two pixels to the point. The sizes are 16, 32, 64, 128, 256, 512 and 1024 pixels.

macOS, unlike iOS, does not round the corners of an icon: the image has to have them. Each
size is drawn from the SVG itself, as for Windows.

## Tests

`cargo test -p app-icon` checks the icons of a small SVG. For Windows: the sizes, the place
of the drawing in the square, and the clear square around it. For iOS: the size, the PNG
without alpha, and the place of the drawing in the white square. For macOS: the sizes, the
PNG with alpha, the plate and the clear edge, the place of the drawing, and the names in
`Contents.json`.
