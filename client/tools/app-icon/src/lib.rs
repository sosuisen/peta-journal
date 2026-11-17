//! Makes the icons of the apps from an SVG: that of a Windows app and that of an iOS app. The
//! drawing is put in the middle of a square, and its empty edges are left out. Only Rust is
//! needed: the apps are built with it anyway.

use resvg::tiny_skia::{Color, Pixmap, Transform};
use resvg::usvg::{Options, Rect, Tree};
use std::fmt;

/// The sizes in an icon of Windows. Windows picks the one that fits the place and the scale of
/// the screen.
pub const WINDOWS_SIZES: [u32; 10] = [256, 128, 96, 64, 48, 40, 32, 24, 20, 16];

/// The side of the icon of iOS. Xcode makes every other size from it.
pub const IOS_SIDE: u32 = 1024;

/// How large the drawing is in the icon of iOS: the white around it is the margin that the
/// icons of iOS have.
pub const IOS_DRAWING: u32 = 640;

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

/// The `.ico` of the SVG, with every size of [`WINDOWS_SIZES`]. The square is clear around
/// the drawing.
pub fn windows_icon(svg: &[u8]) -> Result<Vec<u8>, Error> {
    let (tree, drawn) = read(svg)?;
    let mut icon = ico::IconDir::new(ico::ResourceType::Icon);
    for size in WINDOWS_SIZES {
        // Each size is drawn from the SVG itself: a small one made from a large image is blurred
        let pixmap = square(&tree, drawn, size, size, None);
        // tiny-skia keeps the colors multiplied by the alpha; an icon keeps them as they are
        let rgba = pixmap
            .pixels()
            .iter()
            .flat_map(|pixel| {
                let color = pixel.demultiply();
                [color.red(), color.green(), color.blue(), color.alpha()]
            })
            .collect();
        let image = ico::IconImage::from_rgba_data(size, size, rgba);
        let entry = ico::IconDirEntry::encode(&image).map_err(Error::Icon)?;
        icon.add_entry(entry);
    }
    let mut bytes = Vec::new();
    icon.write(&mut bytes).map_err(Error::Icon)?;
    Ok(bytes)
}

/// The PNG of the icon of iOS: the drawing in the middle of a white square of [`IOS_SIDE`],
/// as large as fits into [`IOS_DRAWING`]. The PNG has no alpha channel: iOS takes no icon that
/// has one, also when every pixel is opaque.
pub fn ios_icon(svg: &[u8]) -> Result<Vec<u8>, Error> {
    let (tree, drawn) = read(svg)?;
    let pixmap = square(&tree, drawn, IOS_SIDE, IOS_DRAWING, Some(Color::WHITE));
    // Every pixel is opaque on white, so its colors are those that are kept
    let rgb: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| [pixel.red(), pixel.green(), pixel.blue()])
        .collect();
    let mut bytes = Vec::new();
    let mut png = png::Encoder::new(&mut bytes, IOS_SIDE, IOS_SIDE);
    png.set_color(png::ColorType::Rgb);
    png.set_depth(png::BitDepth::Eight);
    let mut writer = png.write_header().map_err(|e| Error::Icon(e.into()))?;
    writer
        .write_image_data(&rgb)
        .map_err(|e| Error::Icon(e.into()))?;
    writer.finish().map_err(|e| Error::Icon(e.into()))?;
    Ok(bytes)
}

/// The SVG and where it draws, with its strokes.
fn read(svg: &[u8]) -> Result<(Tree, Rect), Error> {
    let tree = Tree::from_data(svg, &Options::default()).map_err(Error::Svg)?;
    let drawn = tree.root().abs_stroke_bounding_box().ok_or(Error::Empty)?;
    Ok((tree, drawn))
}

/// A square of `side` pixels with the drawing in its middle, as large as fits into `fit`
/// pixels. The square is clear, or of the color `ground`.
fn square(tree: &Tree, drawn: Rect, side: u32, fit: u32, ground: Option<Color>) -> Pixmap {
    let (side_f, fit_f) = (side as f32, fit as f32);
    let scale = fit_f / drawn.width().max(drawn.height());
    let left = (side_f - drawn.width() * scale) / 2.0 - drawn.x() * scale;
    let top = (side_f - drawn.height() * scale) / 2.0 - drawn.y() * scale;
    let mut pixmap = Pixmap::new(side, side).expect("an icon is never empty");
    if let Some(color) = ground {
        pixmap.fill(color);
    }
    let place = Transform::from_row(scale, 0.0, 0.0, scale, left, top);
    resvg::render(tree, place, &mut pixmap.as_mut());
    pixmap
}

#[cfg(test)]
mod tests;
