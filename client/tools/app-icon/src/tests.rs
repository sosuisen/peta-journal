use super::*;

/// A tall red bar in the upper left of its page: the page is larger than the drawing.
const TALL_BAR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
  <rect x="10" y="20" width="20" height="50" fill="red"/>
</svg>"#;

fn images(svg: &str) -> Vec<ico::IconImage> {
    let icon = windows_icon(svg.as_bytes()).expect("the icon is made");
    let icon = ico::IconDir::read(std::io::Cursor::new(icon)).expect("the icon is read");
    let entries = icon.entries().iter();
    entries
        .map(|entry| entry.decode().expect("the image is read"))
        .collect()
}

/// The first and the last column and row that are drawn on, from the alpha of the pixels.
fn drawn_on(image: &ico::IconImage) -> (u32, u32, u32, u32) {
    let (width, height) = (image.width(), image.height());
    let alpha = |x: u32, y: u32| image.rgba_data()[((y * width + x) * 4 + 3) as usize];
    let columns: Vec<u32> = (0..width)
        .filter(|&x| (0..height).any(|y| alpha(x, y) > 0))
        .collect();
    let rows: Vec<u32> = (0..height)
        .filter(|&y| (0..width).any(|x| alpha(x, y) > 0))
        .collect();
    (
        columns[0],
        *columns.last().unwrap(),
        rows[0],
        *rows.last().unwrap(),
    )
}

// アイコンには、Windowsが使う10の大きさが、大きい順に入る。
#[test]
fn the_icon_has_every_size_that_windows_uses() {
    let sizes: Vec<(u32, u32)> = images(TALL_BAR)
        .iter()
        .map(|image| (image.width(), image.height()))
        .collect();

    let squares: Vec<(u32, u32)> = WINDOWS_SIZES.iter().map(|&size| (size, size)).collect();
    assert_eq!(sizes, squares);
}

// 絵は余白を除いて正方形の真ん中に置かれ、縦長の絵は上下いっぱいに描かれる。
#[test]
fn the_drawing_fills_the_middle_of_the_square_without_its_empty_edges() {
    let largest = &images(TALL_BAR)[0];

    let (left, right, top, bottom) = drawn_on(largest);

    // The bar is 20 by 50: in 256 pixels it is 102.4 wide, from 76.8 to 179.2
    assert_eq!((top, bottom), (0, 255));
    assert!((76..=77).contains(&left), "left {left}");
    assert!((178..=179).contains(&right), "right {right}");
}

// 絵の外は透明で、絵の色は元のままである。
#[test]
fn the_square_is_clear_around_the_drawing_and_the_drawing_keeps_its_color() {
    let largest = &images(TALL_BAR)[0];
    let pixel = |x: u32, y: u32| {
        let at = ((y * largest.width() + x) * 4) as usize;
        largest.rgba_data()[at..at + 4].to_vec()
    };

    assert_eq!(pixel(0, 0)[3], 0);
    assert_eq!(pixel(128, 128), [255, 0, 0, 255]);
}

// 何も描かないSVGからは、アイコンを作らない。
#[test]
fn an_svg_that_draws_nothing_gives_no_icon() {
    let empty = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"/>"#;

    let made = windows_icon(empty.as_bytes());

    assert!(matches!(made, Err(Error::Empty)));
}

/// The PNG of the icon of iOS as it was written: its header, and its pixels as RGB.
fn ios(svg: &str) -> (png::OutputInfo, Vec<u8>) {
    let icon = ios_icon(svg.as_bytes()).expect("the icon is made");
    let decoder = png::Decoder::new(std::io::Cursor::new(icon));
    let mut reader = decoder.read_info().expect("the PNG is read");
    let mut pixels = vec![0; reader.output_buffer_size().expect("the PNG has a size")];
    let info = reader.next_frame(&mut pixels).expect("the image is read");
    (info, pixels)
}

// iOSのアイコンは、1024pxの正方形で、アルファチャンネルのないPNGである。iOSは、アルファチャンネルのあるアイコンを受け付けない。
#[test]
fn the_ios_icon_is_a_square_of_1024_without_alpha() {
    let (info, _) = ios(TALL_BAR);

    assert_eq!((info.width, info.height), (1024, 1024));
    assert_eq!(info.color_type, png::ColorType::Rgb);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
}

// iOSのアイコンでは、絵が余白を除いて640pxに収まり、白い正方形の真ん中に置かれる。
#[test]
fn the_ios_icon_has_the_drawing_in_640_in_the_middle_of_white() {
    let (_, pixels) = ios(TALL_BAR);
    let pixel = |x: usize, y: usize| pixels[(y * 1024 + x) * 3..(y * 1024 + x) * 3 + 3].to_vec();
    let inked = |x: usize, y: usize| pixel(x, y) != [255, 255, 255];
    let columns: Vec<usize> = (0..1024)
        .filter(|&x| (0..1024).any(|y| inked(x, y)))
        .collect();
    let rows: Vec<usize> = (0..1024)
        .filter(|&y| (0..1024).any(|x| inked(x, y)))
        .collect();

    // The bar is 20 by 50: in 640 pixels it is 256 wide, from 384 to 640, and from 192 to 832
    assert_eq!((rows[0], *rows.last().unwrap()), (192, 831));
    assert_eq!((columns[0], *columns.last().unwrap()), (384, 639));
    assert_eq!(pixel(0, 0), [255, 255, 255]);
    assert_eq!(pixel(512, 512), [255, 0, 0]);
}

/// The images of the icon set of macOS by their file names, each with its header and its
/// pixels as RGBA.
fn macos(svg: &str) -> Vec<(String, png::OutputInfo, Vec<u8>)> {
    let files = macos_icon_set(svg.as_bytes()).expect("the icon set is made");
    files
        .into_iter()
        .filter(|(name, _)| name.ends_with(".png"))
        .map(|(name, bytes)| {
            let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
            let mut reader = decoder.read_info().expect("the PNG is read");
            let mut pixels = vec![0; reader.output_buffer_size().expect("the PNG has a size")];
            let info = reader.next_frame(&mut pixels).expect("the image is read");
            (name, info, pixels)
        })
        .collect()
}

// macOSのアイコンセットには、16から1024pxまでの7つの画像が、アルファチャンネルのあるPNGで入る。
#[test]
fn the_macos_icon_set_has_seven_sizes_with_alpha() {
    let images = macos(TALL_BAR);

    let sizes: Vec<(String, u32, u32)> = images
        .iter()
        .map(|(name, info, _)| (name.clone(), info.width, info.height))
        .collect();
    let expected: Vec<(String, u32, u32)> = [16, 32, 64, 128, 256, 512, 1024]
        .iter()
        .map(|&side| (format!("icon-{side}.png"), side, side))
        .collect();
    assert_eq!(sizes, expected);
    assert!(images
        .iter()
        .all(|(_, info, _)| info.color_type == png::ColorType::Rgba));
}

// macOSのアイコンでは、角の丸い白い板が824pxで真ん中にあり、その外は透明で、絵は板の真ん中に515pxで収まる。
#[test]
fn the_macos_icon_has_the_drawing_on_a_white_plate_in_a_clear_square() {
    let images = macos(TALL_BAR);
    let (_, _, pixels) = images.last().expect("the largest image");
    let pixel = |x: usize, y: usize| pixels[(y * 1024 + x) * 4..(y * 1024 + x) * 4 + 4].to_vec();
    let red = |x: usize, y: usize| pixel(x, y) == [255, 0, 0, 255];
    let columns: Vec<usize> = (0..1024)
        .filter(|&x| (0..1024).any(|y| red(x, y)))
        .collect();
    let rows: Vec<usize> = (0..1024)
        .filter(|&y| (0..1024).any(|x| red(x, y)))
        .collect();

    // Outside the plate, which is 100 in from each edge
    assert_eq!(pixel(50, 512)[3], 0);
    // The rounded corner of the plate leaves the corner of its square clear
    assert_eq!(pixel(102, 102)[3], 0);
    // On the plate, beside the drawing
    assert_eq!(pixel(150, 512), [255, 255, 255, 255]);
    // The bar is 20 by 50: in 515 pixels it is 206 wide, from 409 to 615, and from 254.5 to
    // 769.5, where the half pixels at its ends are not wholly red
    assert_eq!((rows[0], *rows.last().unwrap()), (255, 768));
    assert_eq!((columns[0], *columns.last().unwrap()), (409, 614));
}

// アイコンセットのContents.jsonは、16から512ポイントの各大きさを、1倍と2倍の画像で名指す。
#[test]
fn the_contents_of_the_icon_set_name_each_size_at_1x_and_2x() {
    let files = macos_icon_set(TALL_BAR.as_bytes()).expect("the icon set is made");
    let (_, contents) = files
        .iter()
        .find(|(name, _)| name == "Contents.json")
        .expect("the set has its Contents.json");
    let contents = String::from_utf8(contents.clone()).expect("the JSON is text");

    let named: Vec<&str> = contents
        .lines()
        .filter_map(|line| line.trim().strip_prefix("\"filename\" : \""))
        .map(|rest| rest.trim_end_matches("\","))
        .collect();
    assert_eq!(
        named,
        [
            "icon-16.png",
            "icon-32.png",
            "icon-32.png",
            "icon-64.png",
            "icon-128.png",
            "icon-256.png",
            "icon-256.png",
            "icon-512.png",
            "icon-512.png",
            "icon-1024.png"
        ]
    );
    assert!(contents.contains("\"size\" : \"512x512\""));
    assert!(contents.contains("\"scale\" : \"2x\""));
    assert!(contents.contains("\"idiom\" : \"mac\""));
}

// 何も描かないSVGからは、macOSのアイコンセットを作らない。
#[test]
fn an_svg_that_draws_nothing_gives_no_macos_icon_set() {
    let made =
        macos_icon_set(br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>"#);

    assert!(matches!(made, Err(Error::Empty)));
}
