//! Makes the icons of the apps from an SVG: that of a Windows app, of an iOS app and of a macOS
//! app. The drawing is put in the middle of a square, and its empty edges are left out. Only
//! Rust is needed: the apps are built with it anyway.

use resvg::tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};
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

/// The sizes of the images of the icon of macOS, in pixels. The icon set names sizes from 16
/// to 512 points, each at one and two pixels to the point.
pub const MACOS_SIZES: [u32; 7] = [16, 32, 64, 128, 256, 512, 1024];

/// The white plate of the icon of macOS on a side of 1024, as the template of Apple has it: a
/// square of 824 with rounded corners, 100 in from each edge. Outside it the icon is clear.
pub const MACOS_PLATE: u32 = 824;
const MACOS_PLATE_RADIUS: f32 = 185.4;

/// How large the drawing is on the plate of 1024: as on iOS, five eighths of the plate.
pub const MACOS_DRAWING: u32 = 515;

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
    png_of(IOS_SIDE, png::ColorType::Rgb, &rgb)
}

/// The files of the icon set of macOS (`AppIcon.appiconset`): a PNG for each of
/// [`MACOS_SIZES`], and the `Contents.json` that names them. Each PNG has the drawing in the
/// middle of the white plate of [`MACOS_PLATE`], clear around it.
pub fn macos_icon_set(svg: &[u8]) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let (tree, drawn) = read(svg)?;
    let mut files = Vec::new();
    for size in MACOS_SIZES {
        // Each size is drawn from the SVG itself, as for Windows: a small one made from a
        // large image is blurred
        let pixmap = on_plate(&tree, drawn, size);
        files.push((macos_file(size), rgba_png(&pixmap)?));
    }
    files.push(("Contents.json".to_string(), macos_contents().into_bytes()));
    Ok(files)
}

fn macos_file(pixels: u32) -> String {
    format!("icon-{pixels}.png")
}

/// The `Contents.json` of the icon set: each size of 16 to 512 points at 1x and 2x.
fn macos_contents() -> String {
    let images: Vec<String> = [16, 32, 128, 256, 512]
        .iter()
        .flat_map(|points| {
            [1, 2].map(|scale| {
                format!(
                    "    {{\n      \"filename\" : \"{}\",\n      \"idiom\" : \"mac\",\n      \"scale\" : \"{scale}x\",\n      \"size\" : \"{points}x{points}\"\n    }}",
                    macos_file(points * scale)
                )
            })
        })
        .collect();
    format!(
        "{{\n  \"images\" : [\n{}\n  ],\n  \"info\" : {{\n    \"author\" : \"xcode\",\n    \"version\" : 1\n  }}\n}}\n",
        images.join(",\n")
    )
}

/// A clear square of `side` with the white plate of macOS, and the drawing in its middle.
fn on_plate(tree: &Tree, drawn: Rect, side: u32) -> Pixmap {
    let scale = side as f32 / 1024.0;
    let mut pixmap = Pixmap::new(side, side).expect("an icon is never empty");
    let edge = (1024 - MACOS_PLATE) as f32 / 2.0 * scale;
    let plate = rounded_square(edge, MACOS_PLATE as f32 * scale, MACOS_PLATE_RADIUS * scale);
    let mut white = Paint::default();
    white.set_color(Color::WHITE);
    white.anti_alias = true;
    pixmap.fill_path(
        &plate,
        &white,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    let fit = MACOS_DRAWING as f32 * scale;
    resvg::render(tree, place(drawn, side as f32, fit), &mut pixmap.as_mut());
    pixmap
}

/// A square from `at` to `at + side` on both axes, with corners of the radius `radius`.
fn rounded_square(at: f32, side: f32, radius: f32) -> resvg::tiny_skia::Path {
    // How far the control points of a cubic curve stand that draws a quarter of a circle
    let k = radius * 0.552_284_8;
    let (near, far) = (at, at + side);
    let mut path = PathBuilder::new();
    path.move_to(near + radius, near);
    path.line_to(far - radius, near);
    path.cubic_to(
        far - radius + k,
        near,
        far,
        near + radius - k,
        far,
        near + radius,
    );
    path.line_to(far, far - radius);
    path.cubic_to(
        far,
        far - radius + k,
        far - radius + k,
        far,
        far - radius,
        far,
    );
    path.line_to(near + radius, far);
    path.cubic_to(
        near + radius - k,
        far,
        near,
        far - radius + k,
        near,
        far - radius,
    );
    path.line_to(near, near + radius);
    path.cubic_to(
        near,
        near + radius - k,
        near + radius - k,
        near,
        near + radius,
        near,
    );
    path.close();
    path.finish()
        .expect("a square with rounded corners is a path")
}

/// The pixels as a PNG with alpha. tiny-skia keeps the colors multiplied by the alpha; a PNG
/// keeps them as they are.
fn rgba_png(pixmap: &Pixmap) -> Result<Vec<u8>, Error> {
    let rgba: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    png_of(pixmap.width(), png::ColorType::Rgba, &rgba)
}

fn png_of(side: u32, color: png::ColorType, data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    let mut png = png::Encoder::new(&mut bytes, side, side);
    png.set_color(color);
    png.set_depth(png::BitDepth::Eight);
    let mut writer = png.write_header().map_err(|e| Error::Icon(e.into()))?;
    writer
        .write_image_data(data)
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
    let mut pixmap = Pixmap::new(side, side).expect("an icon is never empty");
    if let Some(color) = ground {
        pixmap.fill(color);
    }
    resvg::render(
        tree,
        place(drawn, side as f32, fit as f32),
        &mut pixmap.as_mut(),
    );
    pixmap
}

/// Where the drawing goes in a square of `side`: in its middle, as large as fits into `fit`.
fn place(drawn: Rect, side: f32, fit: f32) -> Transform {
    let scale = fit / drawn.width().max(drawn.height());
    let left = (side - drawn.width() * scale) / 2.0 - drawn.x() * scale;
    let top = (side - drawn.height() * scale) / 2.0 - drawn.y() * scale;
    Transform::from_row(scale, 0.0, 0.0, scale, left, top)
}

#[cfg(test)]
mod tests;
