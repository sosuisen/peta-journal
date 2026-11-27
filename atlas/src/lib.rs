//! The atlas of a directory of PetaJournal, kept as the outline of its `index_atlas` page.
//! The server writes that outline; the frontend and other programs, such as a static site,
//! read it and draw it with `atlas.js`.
//!
//! - The crate root reads the outline as the server wrote it: what each item is and which
//!   group a card is in. The server reads it so until it has moved every atlas to the plain
//!   form (ADR 053).
//! - [`plain`] reads the outline of an atlas that is a plain page, as the clients draw it.
//! - [`marks`] draws the icons, blobs and faces as SVG (feature `marks`).
//! - [`scene`] writes the cards and callouts as `atlas.js` reads them (feature `marks`).
//! - [`measure`] measures the width of a card from its label in a font (feature `measure`).

#[cfg(feature = "marks")]
pub mod marks;
#[cfg(feature = "measure")]
pub mod measure;
pub mod plain;
#[cfg(feature = "marks")]
pub mod scene;

/// Where a card or a callout lies: `x,y,width,height`.
pub const BOUNDS_KEY: &str = "atlas_bounds";
/// The id of the page a page card stands for.
pub const PAGE_KEY: &str = "atlas_page";
/// What an item that is no card is: `group`, `callouts` or `callout`.
pub const TYPE_KEY: &str = "atlas_type";
/// The side of a group its title lies along, or the side of a callout its bubble is on.
pub const SIDE_KEY: &str = "atlas_side";

pub const GROUP_TYPE: &str = "group";
pub const CALLOUTS_TYPE: &str = "callouts";
pub const CALLOUT_TYPE: &str = "callout";

/// The sides a title or a bubble can take, the first by default.
pub const SIDES: [&str; 4] = ["top", "bottom", "left", "right"];

/// The height of a card at scale 1, which it takes when it is first placed.
pub const CARD_HEIGHT: f64 = 40.0;
/// The stored width that lets the label set the width of a card. Any other width is the
/// width of the card. A card first placed takes this one.
pub const AUTO_WIDTH: f64 = 0.0;
/// The name of the page that holds the atlas of its directory. The journal keeps the name
/// for the atlas, and the clients show the page of that name as an atlas.
pub const ATLAS_NAME: &str = "index_atlas";

/// The smallest a card can be scaled to. A larger card has no limit.
pub const MIN_CARD_SCALE: f64 = 0.5;

/// The place and size of a card or a callout. `x` and `y` are its center; `y` grows
/// downward, as on the screen.
#[cfg_attr(feature = "marks", derive(serde::Serialize))]
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The text is not four finite numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotBounds(String);

impl std::fmt::Display for NotBounds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bounds must be four numbers x,y,width,height: {}",
            self.0
        )
    }
}

impl std::error::Error for NotBounds {}

/// Reads `x,y,width,height`, with or without spaces around the numbers. The journal and its
/// clients both read the value of the `bounds` property here.
impl std::str::FromStr for Bounds {
    type Err = NotBounds;

    fn from_str(text: &str) -> Result<Bounds, NotBounds> {
        let numbers = text
            .split(',')
            .map(|n| n.trim().parse::<f64>().ok().filter(|n| n.is_finite()))
            .collect::<Option<Vec<f64>>>();
        match numbers.as_deref() {
            Some(&[x, y, width, height]) => Ok(Bounds {
                x,
                y,
                width,
                height,
            }),
            _ => Err(NotBounds(text.to_string())),
        }
    }
}

/// Writes the value without spaces, each number as a number property of the journal shows
/// it, so the front matter and the metadata line of an item hold it as one plain value.
impl std::fmt::Display for Bounds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [x, y, width, height] = [self.x, self.y, self.width, self.height].map(number);
        write!(f, "{x},{y},{width},{height}")
    }
}

/// A number as a number property of the journal shows it (`format_number` of `journal-core`,
/// which this crate cannot use): an integer without a fraction, and a number too large to be
/// written so with `.0` behind it.
fn number(n: f64) -> String {
    if n == n.floor() && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    let text = format!("{n}");
    if text.contains('.') {
        text
    } else {
        format!("{text}.0")
    }
}

impl Bounds {
    /// Reads a bounds value. A value that is not four numbers counts as no value, so the card
    /// can be placed again.
    pub fn parse(value: &str) -> Option<Bounds> {
        value.parse().ok()
    }

    /// How much the card is scaled, read from the height: the width may be left to the label.
    pub fn scale(&self) -> f64 {
        self.height / CARD_HEIGHT
    }
}

/// An item of an atlas page as the reader sees it.
pub trait AtlasItem {
    /// The level of the item in the outline; the top level is 1.
    fn level(&self) -> i32;
    fn body(&self) -> &str;
    /// A scalar property of the item.
    fn prop(&self, key: &str) -> Option<&str>;
}

#[cfg(feature = "api-types")]
impl AtlasItem for journal_api_types::Item {
    fn level(&self) -> i32 {
        self.level
    }

    fn body(&self) -> &str {
        &self.body
    }

    fn prop(&self, key: &str) -> Option<&str> {
        self.properties
            .get(key)
            .and_then(journal_api_types::PropertyValue::as_text)
    }
}

/// A side as stored: one of [`SIDES`], or the first.
pub fn side_or_default(side: &str) -> &'static str {
    SIDES
        .iter()
        .find(|s| **s == side)
        .copied()
        .unwrap_or(SIDES[0])
}

/// The side of an item: of the title of a group, or of the bubble of a callout.
pub fn side_of<T: AtlasItem>(item: &T) -> &'static str {
    side_or_default(item.prop(SIDE_KEY).unwrap_or(""))
}

/// The page a body links to when the body is a single wiki link and nothing else.
pub fn link_target(body: &str) -> Option<&str> {
    let inner = body.trim().strip_prefix("[[")?.strip_suffix("]]")?;
    let inner = inner.trim();
    (!inner.is_empty() && !inner.contains("[[") && !inner.contains("]]")).then_some(inner)
}

/// The body of a card that links to the page at `path`.
pub fn link_body(path: &str) -> String {
    format!("[[{path}]]")
}

/// The name a card shows for the page at `path`: the last part of the path. An index page is
/// named by its folder with a slash, and the index of the root `(root)/`.
pub fn card_label(path: &str) -> String {
    let path = path.strip_suffix(".md").unwrap_or(path);
    let (dir, name) = match path.rfind('/') {
        Some(slash) => (&path[..=slash], &path[slash + 1..]),
        None => ("", path),
    };
    if name != "index" {
        return name.to_string();
    }
    match dir.trim_end_matches('/').rsplit('/').next() {
        Some(last) if !last.is_empty() => format!("{last}/"),
        _ => "(root)/".to_string(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::BTreeMap;

    pub(crate) struct Item {
        level: i32,
        body: String,
        props: BTreeMap<String, String>,
    }

    impl AtlasItem for Item {
        fn level(&self) -> i32 {
            self.level
        }
        fn body(&self) -> &str {
            &self.body
        }
        fn prop(&self, key: &str) -> Option<&str> {
            self.props.get(key).map(String::as_str)
        }
    }

    pub(crate) fn item(level: i32, body: &str, props: &[(&str, &str)]) -> Item {
        Item {
            level,
            body: body.to_string(),
            props: props
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    // サーバーの答えの項目は、文字列のプロパティだけをアトラスの値として読む。
    #[cfg(feature = "api-types")]
    #[test]
    fn an_item_of_the_answer_gives_its_text_properties_to_the_atlas() {
        let answer = journal_api_types::Item {
            level: 2,
            body: "card".into(),
            properties: BTreeMap::from([
                ("side".into(), "left".into()),
                ("tags".into(), ["x"].into()),
            ]),
            ..Default::default()
        };

        assert_eq!(
            (
                answer.level(),
                answer.body(),
                answer.prop("side"),
                answer.prop("tags")
            ),
            (2, "card", Some("left"), None)
        );
    }

    // 4つの数の値はbounds、それ以外はboundsでない
    #[test]
    fn bounds_are_four_numbers() {
        assert_eq!(
            Bounds::parse("1, -2.5,0,40"),
            Some(Bounds {
                x: 1.0,
                y: -2.5,
                width: 0.0,
                height: 40.0
            })
        );
        assert_eq!(Bounds::parse("1,2,3"), None);
        assert_eq!(Bounds::parse("1,2,x,4"), None);
        assert_eq!(Bounds::parse("1,2,inf,4"), None);
    }

    // 値は、空白なしで、数のプロパティと同じ書き方で書かれる。大きすぎる整数には「.0」が付く
    #[test]
    fn bounds_are_written_without_spaces_as_number_properties_are() {
        let written = |text: &str| text.parse::<Bounds>().unwrap().to_string();
        assert_eq!(written(" 10, 20.5,160 ,-56 "), "10,20.5,160,-56");
        assert_eq!(written("999999999999999,0,0,0"), "999999999999999,0,0,0");
        assert_eq!(
            written("1e15,-1e15,0,0"),
            "1000000000000000.0,-1000000000000000.0,0,0"
        );
    }

    // boundsでない値は、その値を添えた理由で断られる
    #[test]
    fn a_value_that_is_not_bounds_is_refused_with_the_value() {
        assert_eq!(
            "1,2,3".parse::<Bounds>().unwrap_err().to_string(),
            "Bounds must be four numbers x,y,width,height: 1,2,3"
        );
    }

    // 倍率は高さから読む
    #[test]
    fn scale_is_read_from_the_height() {
        assert_eq!(Bounds::parse("0,0,0,60").unwrap().scale(), 1.5);
    }

    // 辺は4つのどれかで、ほかは上になる
    #[test]
    fn a_side_is_one_of_four_or_top() {
        assert_eq!(side_or_default("left"), "left");
        assert_eq!(side_or_default("middle"), "top");
        assert_eq!(side_or_default(""), "top");
    }

    // 1つのwiki linkだけの本文は、そのリンク先を指す
    #[test]
    fn a_single_wiki_link_names_its_target() {
        assert_eq!(link_target(" [[ d/p ]] "), Some("d/p"));
        assert_eq!(link_target("[[a]] and [[b]]"), None);
        assert_eq!(link_target("[[]]"), None);
        assert_eq!(link_target("d/p"), None);
    }

    // カードのラベルはパスの最後で、indexはフォルダ名にスラッシュを付ける
    #[test]
    fn a_card_label_is_the_last_part_of_the_path() {
        assert_eq!(card_label("d/e/page"), "page");
        assert_eq!(card_label("d/e/page.md"), "page");
        assert_eq!(card_label("d/e/index"), "e/");
        assert_eq!(card_label("index"), "(root)/");
    }
}
