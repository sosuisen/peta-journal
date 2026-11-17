//! Makes the icon of a Windows app from an SVG. The drawing is put in the middle of a clear
//! square, and its empty edges are left out. Only Rust is needed: the apps are built with it
//! anyway.

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Rect, Tree};
use std::fmt;

/// The sizes in an icon of Windows. Windows picks the one that fits the place and the scale of
/// the screen.
pub const WINDOWS_SIZES: [u32; 10] = [256, 128, 96, 64, 48, 40, 32, 24, 20, 16];

#[derive(Debug)]
pub enum Error {
    /// The SVG cannot be read.
    Svg(resvg::usvg::Error),
    /// The SVG draws nothing.
    Empty,
    /// The icon cannot be written.
    Icon(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Svg(error) => write!(f, "the SVG cannot be read: {error}"),
            Error::Empty => write!(f, "the SVG draws nothing"),
            Error::Icon(error) => write!(f, "the icon cannot be written: {error}"),
        }
    }
}

impl std::error::Error for Error {}

/// The `.ico` of the SVG, with every size of [`WINDOWS_SIZES`].
pub fn windows_icon(svg: &[u8]) -> Result<Vec<u8>, Error> {
    let tree = Tree::from_data(svg, &Options::default()).map_err(Error::Svg)?;
    let drawn = tree.root().abs_stroke_bounding_box().ok_or(Error::Empty)?;
    let mut icon = ico::IconDir::new(ico::ResourceType::Icon);
    for size in WINDOWS_SIZES {
        // Each size is drawn from the SVG itself: a small one made from a large image is blurred
        let image = square(&tree, drawn, size);
        let entry = ico::IconDirEntry::encode(&image).map_err(Error::Icon)?;
        icon.add_entry(entry);
    }
    let mut bytes = Vec::new();
    icon.write(&mut bytes).map_err(Error::Icon)?;
    Ok(bytes)
}

/// The drawing in the middle of a clear square of `size` pixels, as large as fits.
fn square(tree: &Tree, drawn: Rect, size: u32) -> ico::IconImage {
    let side = size as f32;
    let scale = side / drawn.width().max(drawn.height());
    let left = (side - drawn.width() * scale) / 2.0 - drawn.x() * scale;
    let top = (side - drawn.height() * scale) / 2.0 - drawn.y() * scale;
    let mut pixmap = Pixmap::new(size, size).expect("an icon is never empty");
    let place = Transform::from_row(scale, 0.0, 0.0, scale, left, top);
    resvg::render(tree, place, &mut pixmap.as_mut());
    // tiny-skia keeps the colors multiplied by the alpha; an icon keeps them as they are
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    ico::IconImage::from_rgba_data(size, size, rgba)
}

#[cfg(test)]
mod tests;
