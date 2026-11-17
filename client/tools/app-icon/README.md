# app-icon

Makes the icons of the apps from an SVG: that of a Windows app and that of an iOS app. The
apps get their icons from it through a script in their folders:

| App | Script | Icon |
|---|---|---|
| `stickies-win` | `icon.ps1`, `icon.sh` | `PetaStickies/PetaStickies.ico` |
| `outliner-ios` | `icon.sh` | `App/Assets.xcassets/AppIcon.appiconset/icon-1024.png` |

```
cd peta-journal/client
cargo run --release -p app-icon -- windows <svg> <ico>
cargo run --release -p app-icon -- ios <svg> <png>
```

## Why Rust

The apps are built with Rust anyway. A tool such as resvg or ImageMagick would have to be
installed only to make the icons. The crate uses resvg, ico and png as libraries, so `cargo`
brings them.

## How an icon is made

Both icons start the same way. The SVG is read. Its drawing need not be in the middle of its
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

## Tests

`cargo test -p app-icon` checks the icons of a small SVG. For Windows: the sizes, the place
of the drawing in the square, and the clear square around it. For iOS: the size, the PNG
without alpha, and the place of the drawing in the white square.
