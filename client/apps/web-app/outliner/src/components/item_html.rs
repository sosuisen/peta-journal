//! The HTML of an item, as the web shell shows it (ADR 026).
//!
//! `ast_to_html` draws every kind of node, and it is not changed for this shell. It leaves out
//! the size of an image, because each app uses it in its own way. So the size travels in the
//! fragment of the image URL, and `item_view` reads it back from the `<img>`.

use std::collections::BTreeMap;
use std::num::NonZeroU64;

use flatmarkdown_ast::{Node, PropValue};
use flatmarkdown_ast2html::ast_to_html;

/// The HTML of an item, from the AST that the core gives with its row (ADR 041).
pub fn item_html(ast: &Node) -> String {
    let mut root = ast.clone();
    keep_image_sizes(&mut root);
    ast_to_html(&root)
}

/// Puts the size of every image under `node` into the fragment of its URL.
fn keep_image_sizes(node: &mut Node) {
    if let Node::Embed { url, props, .. } = node {
        url.push_str(&ImageSize::of_props(props).fragment());
    }
    for child in node.children_mut().into_iter().flatten() {
        keep_image_sizes(child);
    }
}

/// The size that the body asks for an image, in pixels. A side that is not given follows the
/// aspect ratio of the image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageSize {
    pub width: Option<NonZeroU64>,
    pub height: Option<NonZeroU64>,
}

impl ImageSize {
    /// The size in the `w` and `h` props of an image. The parser writes every prop with a value
    /// as text. A side that is not a number above zero is not given.
    fn of_props(props: &BTreeMap<String, PropValue>) -> Self {
        let side = |key: &str| match props.get(key) {
            Some(PropValue::Text(text)) => pixels(text),
            _ => None,
        };
        Self {
            width: side("w"),
            height: side("h"),
        }
    }

    /// The size in the fragment of an image URL, as `fragment` writes it.
    pub fn of_src(src: &str) -> Self {
        let mut size = Self::default();
        let Some((_, fragment)) = src.rsplit_once('#') else {
            return size;
        };
        for pair in fragment.split('&') {
            match pair.split_once('=') {
                Some(("w", value)) => size.width = pixels(value),
                Some(("h", value)) => size.height = pixels(value),
                _ => {}
            }
        }
        size
    }

    /// The fragment that carries this size, with its `#`. Empty when no side is given.
    fn fragment(self) -> String {
        let sides: Vec<String> = [("w", self.width), ("h", self.height)]
            .into_iter()
            .filter_map(|(key, side)| side.map(|pixels| format!("{key}={pixels}")))
            .collect();
        if sides.is_empty() {
            String::new()
        } else {
            format!("#{}", sides.join("&"))
        }
    }
}

fn pixels(text: &str) -> Option<NonZeroU64> {
    text.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ast_of(body: &str) -> Node {
        client_core::asts::parse(body)
    }

    fn size(width: u64, height: u64) -> ImageSize {
        ImageSize {
            width: NonZeroU64::new(width),
            height: NonZeroU64::new(height),
        }
    }

    // 項目のASTから、HTMLができる。
    #[test]
    fn the_ast_of_an_item_becomes_html() {
        assert_eq!(
            item_html(&ast_of("Some **bold**")),
            "<p>Some <strong>bold</strong></p>\n"
        );
    }

    // 本文が空の項目は、何も表示しない。
    #[test]
    fn an_item_without_a_body_shows_nothing() {
        assert_eq!(item_html(&ast_of("")), "");
    }

    // 画像の大きさは、HTMLの画像のURLから読み戻せる。
    #[test]
    fn the_size_of_an_image_is_read_back_from_its_url_in_the_html() {
        let html = item_html(&ast_of("![alt](img.png#w=300&h=240)"));

        let src = html.split('"').nth(1).unwrap().replace("&amp;", "&");
        assert!(src.starts_with("img.png"), "{src}");
        assert_eq!(ImageSize::of_src(&src), size(300, 240));
    }

    // リストの中など、深いところにある画像も、大きさを保つ。
    #[test]
    fn an_image_deep_in_the_item_keeps_its_size() {
        let html = item_html(&ast_of("- ![alt](img.png#w=300)"));

        assert!(
            html.contains(&format!("src=\"img.png{}\"", size(300, 0).fragment())),
            "{html}"
        );
    }

    // 大きさのない画像のURLは、そのままである。
    #[test]
    fn the_url_of_an_image_without_a_size_stays_as_it_is() {
        assert_eq!(
            item_html(&ast_of("![alt](img.png)")),
            "<p><img src=\"img.png\" alt=\"alt\" /></p>\n"
        );
    }

    // 大きさは、URLのフラグメントに w と h として書かれる。与えられた辺だけが書かれる。
    #[test]
    fn a_size_is_written_as_w_and_h_in_the_fragment() {
        assert_eq!(size(300, 240).fragment(), "#w=300&h=240");
        assert_eq!(size(300, 0).fragment(), "#w=300");
        assert_eq!(size(0, 240).fragment(), "#h=240");
        assert_eq!(size(0, 0).fragment(), "");
    }

    // 書いたフラグメントからは、同じ大きさが読める。
    #[test]
    fn a_size_is_read_back_from_the_fragment_it_wrote() {
        for written in [size(300, 240), size(300, 0), size(0, 240), size(0, 0)] {
            let src = format!("img.png{}", written.fragment());
            assert_eq!(ImageSize::of_src(&src), written, "{src}");
        }
    }

    // 値のない辺、数値でない辺、0の辺は、与えられていないものとする。
    #[test]
    fn a_side_that_is_not_a_number_above_zero_is_not_given() {
        let props_of = |body: &str| match ast_of(body).children() {
            [Node::Paragraph { children }] => match &children[0] {
                Node::Embed { props, .. } => props.clone(),
                other => panic!("not an image: {other:?}"),
            },
            other => panic!("not a paragraph: {other:?}"),
        };

        assert_eq!(
            ImageSize::of_props(&props_of("![alt](img.png#w&h=big)")),
            size(0, 0)
        );
        assert_eq!(
            ImageSize::of_props(&props_of("![alt](img.png#w=0&h=240)")),
            size(0, 240)
        );
        assert_eq!(ImageSize::of_src("img.png#w=big&h=0"), size(0, 0));
        assert_eq!(ImageSize::of_src("img.png"), size(0, 0));
    }
}
