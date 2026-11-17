# app-icon

Makes the icon of a Windows app from an SVG. The Windows stickies (`stickies-win`) get their
icon from it, through `icon.ps1` in their folder.

```
cd peta-journal/client
cargo run --release -p app-icon -- <svg> <ico>
```

## Why Rust

The apps are built with Rust anyway. A tool such as resvg or ImageMagick would have to be
installed only to make the icon. The crate uses resvg and ico as libraries, so `cargo` brings
them.

## How the icon is made

1. The SVG is read. Its drawing need not be in the middle of its page: the empty edges are
   left out. The outline of the drawing, with its strokes, says where they are.
2. For each size, the drawing is drawn into a clear square, as large as fits, in the middle.
   A tall drawing fills the square from top to bottom.
3. The squares go into one `.ico`, from the largest to the smallest.

The sizes are 256, 128, 96, 64, 48, 40, 32, 24, 20 and 16 pixels. Windows picks the one that
fits the place and the scale of the screen.

Each size is drawn from the SVG itself, not made smaller from a large image. A small image
that is made from a large one is blurred.

The `ico` crate stores each image as PNG, as the edges of the drawing are partly clear. PNG
keeps the file small: the icon of the stickies is about 8KB. Windows reads icons with PNG
images.

A text in the SVG is not drawn: the crate is built without fonts. The drawing of the project
has no text.

## Tests

`cargo test -p app-icon` checks the icon of a small SVG: its sizes, the place of the drawing
in the square, and the clear square around it.
