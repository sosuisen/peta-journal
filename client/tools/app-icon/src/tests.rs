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
