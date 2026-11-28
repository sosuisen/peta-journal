//! The marks of the atlas as SVG: the icon of the tags of a card, the blob behind it, the face
//! of a callout and the color of a group. picturesque-marks draws them all, so a card looks
//! like the tag icons of the same tags.

use picturesque_marks::{
    background_svg, signature_svg, BackgroundStyle, Color, ColorName, ComposeLayout,
    SignatureStyle, MAX_COMPOSED_TAGS,
};

/// The color of the character of a callout
const CALLOUT_COLOR: Color = Color::rgb(0x00, 0x00, 0x00);
/// The width of the lines of the character of a callout, in pixels of the SVG as drawn
const CALLOUT_STROKE_WIDTH: &str = "3.5";

/// The character of a callout as an SVG of `size` pixels square: the face picturesque-marks
/// draws for the text as its signature, in black, its lines thinner.
pub fn callout_face_svg(text: &str, size: i32) -> String {
    // picturesque-marks has no option for the width of the lines of a face; it writes 5
    signature_svg(text, SignatureStyle::Face, CALLOUT_COLOR, size, size).replace(
        r#"stroke-width="5""#,
        &format!(r#"stroke-width="{CALLOUT_STROKE_WIDTH}""#),
    )
}

/// The color of a group as `#RRGGBB`, one of the six of picturesque-marks, the colors of the
/// tag icons. A hash of the id of the group picks it, so a group keeps its color on every
/// machine.
pub fn group_color(group_id: &str) -> String {
    ColorName::from_index(fnv1a(group_id) as u32)
        .color()
        .to_hex()
}

/// The icon of the tags as an SVG of `size` pixels square, as picturesque-marks draws it for
/// a tag. Only the first tags fit in one icon.
pub fn tag_icon_svg(tags: &[String], size: i32) -> String {
    let tags = &tags[..tags.len().min(MAX_COMPOSED_TAGS)];
    background_svg(
        tags,
        ComposeLayout::default(),
        BackgroundStyle::default(),
        size,
        size,
    )
}

/// The light blue of the blob behind the icon of a card.
const BLOB_COLOR: Color = Color::rgb(0xCF, 0xE3, 0xFF);
/// The light orange of the blob of a link card, which tells it from the cards of the directory.
const LINK_BLOB_COLOR: Color = Color::rgb(0xFF, 0xD8, 0xA8);
/// The line around the blob of a published page: red, so it stands out, in pixels of the
/// card whatever the SVG is stretched to.
const BLOB_LINE: &str =
    r##" stroke="#E53935" stroke-width="5" vector-effect="non-scaling-stroke""##;
/// The space kept around an outlined blob, as a share of its size, so its line is not cut off
/// at the edges of the SVG
const BLOB_LINE_MARGIN: f64 = 0.09;

/// The blob of a tag as an SVG of `size` pixels square: the outline picturesque-marks draws
/// for the tag name, filled in light blue. The blob fills the whole SVG, which the card
/// stretches to its own size. An outlined blob has a line around it and keeps a thin margin
/// for that line.
pub fn tag_blob_svg(tag: &str, size: i32, outlined: bool) -> String {
    blob_svg(tag, BLOB_COLOR, size, outlined)
}

/// The blob of a link card: drawn from the page it links to as a tag blob is, in light orange.
pub fn link_blob_svg(target: &str, size: i32) -> String {
    blob_svg(target, LINK_BLOB_COLOR, size, false)
}

/// The light gray of the blob of a card to a page that is not there
const MISSING_BLOB_COLOR: Color = Color::rgb(0xDD, 0xDD, 0xDD);

/// The blob of a card to a page that is not there: drawn from its target, in light gray.
pub fn missing_blob_svg(target: &str, size: i32) -> String {
    blob_svg(target, MISSING_BLOB_COLOR, size, false)
}

/// The light yellow of the blob of a plain card, whose first line is no link
const PLAIN_BLOB_COLOR: Color = Color::rgb(0xFF, 0xF1, 0xA8);

/// The blob of a plain card: drawn from its label as a tag blob is, in light yellow.
pub fn plain_blob_svg(label: &str, size: i32) -> String {
    blob_svg(label, PLAIN_BLOB_COLOR, size, false)
}

/// The white of the blob of a card whose label is hidden
const HIDDEN_BLOB_COLOR: Color = Color::rgb(0xFF, 0xFF, 0xFF);

/// A blob as `blob_svg` drew it, filled white: the blob of a card whose label is hidden. The
/// shape stays, so the card is still told apart from its neighbors.
pub fn hidden_blob_svg(blob: &str) -> String {
    // The fill of the path is the first in the SVG
    let Some(start) = blob.find(" fill=\"").map(|i| i + " fill=\"".len()) else {
        return blob.to_string();
    };
    let Some(end) = blob[start..].find('"').map(|i| start + i) else {
        return blob.to_string();
    };
    format!(
        "{}{}{}",
        &blob[..start],
        HIDDEN_BLOB_COLOR.to_hex(),
        &blob[end..]
    )
}

fn blob_svg(name: &str, color: Color, size: i32, outlined: bool) -> String {
    let svg = signature_svg(name, SignatureStyle::Blob, color, size, size);
    // The fill of the path is the first in the SVG
    let svg = if outlined {
        svg.replacen(" fill=\"", &format!("{BLOB_LINE} fill=\""), 1)
    } else {
        svg
    };
    // picturesque-marks leaves a margin around the blob, which would keep it off the edges of
    // the card. The view box is cut to the points of its path instead. The control points of a
    // curve lie a little outside it, so a thin margin stays.
    let Some((mut min_x, mut min_y, mut max_x, mut max_y)) = path_bounds(&svg) else {
        return svg;
    };
    if outlined {
        let margin_x = (max_x - min_x) * BLOB_LINE_MARGIN;
        let margin_y = (max_y - min_y) * BLOB_LINE_MARGIN;
        (min_x, max_x, min_y, max_y) = (
            min_x - margin_x,
            max_x + margin_x,
            min_y - margin_y,
            max_y + margin_y,
        );
    }
    let Some(start) = svg.find("viewBox=\"") else {
        return svg;
    };
    let Some(end) = svg[start + 9..].find('"').map(|i| start + 9 + i) else {
        return svg;
    };
    format!(
        r#"{}viewBox="{} {} {} {}" preserveAspectRatio="none"{}"#,
        &svg[..start],
        min_x,
        min_y,
        max_x - min_x,
        max_y - min_y,
        &svg[end + 1..]
    )
}

/// The smallest box around the points of the first path in the SVG.
fn path_bounds(svg: &str) -> Option<(f64, f64, f64, f64)> {
    let start = svg.find(" d=\"")? + 4;
    let end = start + svg[start..].find('"')?;
    let numbers: Vec<f64> = svg[start..end]
        .split(|c: char| c.is_ascii_whitespace() || c == ',' || c.is_ascii_alphabetic())
        .filter_map(|n| n.parse().ok())
        .collect();
    let (xs, ys): (Vec<f64>, Vec<f64>) = numbers
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[x, y]| (*x, *y))
        .unzip();
    let min = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (!xs.is_empty()).then(|| (min(&xs), min(&ys), max(&xs), max(&ys)))
}

fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // グループのidから、6色のどれかを選ぶ。同じグループは同じ色になる
    #[test]
    fn the_id_of_a_group_picks_one_of_the_six_colors() {
        let six = [
            "#F9A2A2", "#B5B5FF", "#94D094", "#E1A6E1", "#5FD3D3", "#C3C372",
        ];
        for id in [
            "g",
            "2026-09-29-10-00-00-AAAAAAA",
            "2026-09-29-10-00-01-AAAAAAB",
        ] {
            assert!(six.contains(&group_color(id).as_str()), "{id}");
            assert_eq!(group_color(id), group_color(id));
        }
    }

    // アイコンは指定の大きさのSVGで、同じタグからは同じアイコンになる
    #[test]
    fn an_icon_is_an_svg_of_the_size_asked() {
        let tags: Vec<String> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|t| t.to_string())
            .collect();
        let svg = tag_icon_svg(&tags, 24);
        assert!(svg.starts_with("<svg"), "{svg}");
        assert!(svg.contains(r#"width="24""#), "{svg}");
        assert_eq!(svg, tag_icon_svg(&tags, 24));
    }

    // blobはタグから描く水色のSVGで、タグが違えば形も違う
    #[test]
    fn a_blob_is_a_light_blue_svg_drawn_from_the_tag() {
        let svg = tag_blob_svg("DailyReport", 100, false);
        assert!(svg.starts_with("<svg"), "{svg}");
        assert!(svg.contains(r##"fill="#CFE3FF""##), "{svg}");
        assert_eq!(svg, tag_blob_svg("DailyReport", 100, false));
        assert_ne!(svg, tag_blob_svg("JavaFX", 100, false));
    }

    // リンクカードのblobは薄いオレンジで、同じ名前のカードと同じ形である
    #[test]
    fn the_blob_of_a_link_card_is_light_orange() {
        let svg = link_blob_svg("x/Target", 100);
        assert!(svg.contains(r##"fill="#FFD8A8""##), "{svg}");
        assert!(!svg.contains(r##"fill="#CFE3FF""##), "{svg}");
        assert_eq!(
            svg.replace("#FFD8A8", "#CFE3FF"),
            tag_blob_svg("x/Target", 100, false)
        );
    }

    // ノーマルカードのblobはリンクカードと同じ薄いオレンジ、ないページへのカードのblobは薄い灰色で、同じ名前のカードと同じ形である
    #[test]
    fn the_blob_of_a_plain_card_is_the_light_orange_of_a_link_card_and_of_a_missing_page_light_gray() {
        let plain = plain_blob_svg("x/Target", 100);
        let missing = missing_blob_svg("x/Target", 100);
        let shape = tag_blob_svg("x/Target", 100, false);
        assert_eq!(plain, link_blob_svg("x/Target", 100));
        assert!(missing.contains(r##"fill="#DDDDDD""##), "{missing}");
        assert_eq!(
            [
                plain.replace("#FFD8A8", "#CFE3FF"),
                missing.replace("#DDDDDD", "#CFE3FF")
            ],
            [shape.clone(), shape]
        );
    }

    // blobはSVGの端まで広がる
    #[test]
    fn a_blob_fills_its_svg_to_the_edges() {
        let svg = tag_blob_svg("DailyReport", 100, false);
        assert!(svg.contains(r#"preserveAspectRatio="none""#), "{svg}");
        assert!(!svg.contains(r#"viewBox="-50 -50 100 100""#), "{svg}");
    }

    // 公開済みのページのblobには、まわりに線がある
    #[test]
    fn the_blob_of_a_published_page_has_a_line_around_it() {
        let plain = tag_blob_svg("DailyReport", 100, false);
        let outlined = tag_blob_svg("DailyReport", 100, true);
        assert!(!plain.contains("stroke="), "{plain}");
        assert!(outlined.contains(r##"<path d="##), "{outlined}");
        assert!(
            outlined.contains(r#"vector-effect="non-scaling-stroke""#),
            "{outlined}"
        );
        assert!(
            outlined.contains(r#"preserveAspectRatio="none""#),
            "{outlined}"
        );
    }

    // コールアウトのキャラクターは、テキストの顔で、線が細い
    #[test]
    fn the_character_of_a_callout_is_a_face_of_its_text() {
        let svg = callout_face_svg("Hello", 64);
        assert!(svg.starts_with("<svg"), "{svg}");
        assert!(svg.contains("<path"), "{svg}");
        assert!(svg.contains(r#"stroke-width="3.5""#), "{svg}");
        assert!(!svg.contains(r#"stroke-width="5""#), "{svg}");
    }
}
