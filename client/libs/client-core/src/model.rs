//! The data the server sends and receives, in the same JSON shape as the Angular models.

pub use property_names::ValueType;
use property_names::{body_of, type_of};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

/// An item as the server returns it. The server and the client share the fields the client
/// reads (ADR 082); the client adds its methods through `ItemExt`.
pub use journal_api_types::Item;

/// The value of a property: a text, or a list of texts. The server and the client share it
/// (ADR 082).
pub use journal_api_types::PropertyValue;

/// Items are shared between the state, the view model and undo snapshots, so they are
/// reference counted: a change makes a new item instead of touching the shared one.
pub type ItemRef = Arc<Item>;

/// The task state of an item, with the times and the plan that go with it. It is the state a
/// batch writes, so the client keeps it in the type of the server (ADR 081).
pub use journal_api_types::TaskStateInput as TaskState;

/// What the client reads from an item. It is a trait because `Item` is a type of
/// `journal-api-types`, and a crate cannot add methods to a type of another crate (ADR 082).
pub trait ItemExt {
    fn is_task(&self) -> bool;
    fn task_state(&self) -> TaskState;
    fn has_type(&self, name: &str) -> bool;
    /// A string property, or None when unset or not a string.
    fn prop_str(&self, key: &str) -> Option<&str>;
    /// A list property, or an empty list when unset or not a list.
    fn prop_list(&self, key: &str) -> Vec<String>;
    /// The hashtags of the item, as kept in its tags property.
    fn tags(&self) -> Vec<String>;
    /// The zoom of the item as stored in its properties; 1 when unset.
    fn zoom(&self) -> f64;
    /// True for an item of a daily journal page (named YYYY-MM-DD).
    fn is_daily_journal_item(&self) -> bool;
    /// The heading level of a body that starts with `#`..`#####` followed by a space; 0 otherwise.
    fn heading_level_of_body(&self) -> u8;
}

impl ItemExt for Item {
    fn is_task(&self) -> bool {
        self.types.iter().any(|t| t == "task")
    }

    fn task_state(&self) -> TaskState {
        let text_of = |key| self.prop_str(key).map(str::to_string);
        TaskState {
            task: self.is_task(),
            begin: text_of("begin"),
            end: text_of("end"),
            plan: text_of("plan"),
        }
    }

    fn has_type(&self, name: &str) -> bool {
        self.types.iter().any(|t| t == name)
    }

    fn prop_str(&self, key: &str) -> Option<&str> {
        self.properties.get(key).and_then(PropertyValue::as_text)
    }

    fn prop_list(&self, key: &str) -> Vec<String> {
        self.properties
            .get(key)
            .and_then(PropertyValue::as_list)
            .map(<[String]>::to_vec)
            .unwrap_or_default()
    }

    fn tags(&self) -> Vec<String> {
        self.prop_list("tags")
    }

    fn zoom(&self) -> f64 {
        self.prop_str("zoom")
            .and_then(|z| z.parse::<f64>().ok())
            .filter(|z| *z != 0.0)
            .unwrap_or(1.0)
    }

    fn is_daily_journal_item(&self) -> bool {
        self.item_type == "journal"
            && crate::util::filename::classify_journal_file_name(&self.name)
                == crate::util::filename::JournalType::Daily
    }

    fn heading_level_of_body(&self) -> u8 {
        let trimmed = self.body.trim_start();
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=5).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            hashes as u8
        } else {
            0
        }
    }
}

/// A page as the server returns it. The server and the client share the fields the client
/// reads (ADR 082); the client adds its methods through `PageExt`.
pub use journal_api_types::Page;

/// What the client reads from a page. It is a trait because `Page` is a type of
/// `journal-api-types`, and a crate cannot add methods to a type of another crate (ADR 082).
pub trait PageExt {
    fn is_journal(&self) -> bool;
    /// `dirPath` without its trailing slash plus the name, the form used in URLs.
    fn canonical_path(&self) -> String;
}

impl PageExt for Page {
    fn is_journal(&self) -> bool {
        self.page_type == "journal"
    }

    fn canonical_path(&self) -> String {
        if self.dir_path.is_empty() {
            self.name.clone()
        } else {
            format!("{}/{}", self.dir_path.trim_end_matches('/'), self.name)
        }
    }
}

/// What the sidebar's index list is sorted by.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexSortKey {
    #[default]
    Name,
    Modified,
    /// The `published` property; a page without one falls back to `modified`.
    Published,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// The sort of one directory's index list. The shell keeps one per directory.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IndexSort {
    pub key: IndexSortKey,
    pub order: SortOrder,
}

/// A page or an item that a search found. The server and the client share it (ADR 082).
pub use journal_api_types::SearchResult;

/// Where an item goes: first on the page, last, or after another item. The server and the
/// client share the type (ADR 081).
pub use journal_api_types::ItemPosition;

/// One operation of a batch, as the server reads it (ADR 081).
pub use journal_api_types::BatchOpInput;

/// How the server reads a pasted text. The names are the ones the server reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PasteMode {
    /// Tree markdown is converted, and the text is split into items.
    Auto,
    /// The text is Flat Markdown, as this app copies it: it is split into items as it is.
    Flat,
    /// The text goes into the item as it is.
    Raw,
}

impl PasteMode {
    /// The name the server reads.
    pub fn as_api(&self) -> &'static str {
        match self {
            PasteMode::Auto => "auto",
            PasteMode::Flat => "flat",
            PasteMode::Raw => "raw",
        }
    }
}

/// A file the server saved. The server and the client share the type (ADR 082).
pub use journal_api_types::Uploaded;

/// What a paste came to on the server. The server and the client share the type (ADR 082).
pub use journal_api_types::Pasted;

/// What a batch came to on the server. The server and the client share the type (ADR 082).
pub use journal_api_types::Batched;

/// What sending an item to the stash came to on the server. The server and the client share the type (ADR 082).
pub use journal_api_types::Stashed;

/// What a split came to on the server. The server and the client share the type (ADR 082).
pub use journal_api_types::Split;

/// Where the cursor goes after a paste. The server and the client share the type (ADR 082).
pub use journal_api_types::PasteCursor;

/// A property definition. The server and the client share the type (ADR 082).
pub use journal_api_types::PropertyDefinition;

/// The name the screen shows for the key: the body, without the suffix of its type (ADR
/// 047). When another key of the definitions has the same body, the type is added, so that
/// the two can be told apart: `rating (number)`. A system key is shown whole.
pub fn shown_name(key: &str, definitions: &[PropertyDefinition]) -> String {
    let name = bare_name(key, definitions);
    let shared = definitions
        .iter()
        .any(|d| d.key != key && bare_name(&d.key, definitions) == name);
    if shared {
        format!("{name} ({})", type_of(key).as_str())
    } else {
        name.to_string()
    }
}

/// The name of the key without the suffix of its type (ADR 047). A system key is whole:
/// `atlas_bounds` is no `atlas` of type bounds.
pub fn bare_name<'a>(key: &'a str, definitions: &[PropertyDefinition]) -> &'a str {
    if definitions.iter().any(|d| d.system && d.key == key) {
        key
    } else {
        body_of(key)
    }
}

/// The type to send a list value of the key with. Only `tags` holds the tags; every other
/// list key ends with `_list` and holds a list of the user (ADR 046).
pub fn list_type_of(key: &str) -> &'static str {
    if key == "tags" {
        "tags"
    } else {
        "list"
    }
}

/// The type to send a value of the key with. A list stays a list, even when it has no
/// definition. Any other key without a definition is text.
pub fn value_type_of(key: &str, is_list: bool, definitions: &[PropertyDefinition]) -> String {
    if is_list {
        return list_type_of(key).to_string();
    }
    definitions
        .iter()
        .find(|d| d.key == key)
        .map_or_else(|| "text".to_string(), |d| d.value_type.clone())
}

/// The types a user may give a new key, in the order the settings offer them.
pub const USER_VALUE_TYPES: [ValueType; 7] = [
    ValueType::Text,
    ValueType::Number,
    ValueType::Checkbox,
    ValueType::Date,
    ValueType::Datetime,
    ValueType::Bounds,
    ValueType::List,
];

/// The server's settings. The server and the client share the fields the client reads
/// (ADR 082).
pub use journal_api_types::ServerConfig;

/// How weeks are numbered: US (Sunday start) or ISO (Monday start).
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekNumbering {
    #[default]
    Us,
    Iso,
}

impl WeekNumbering {
    pub fn parse(s: &str) -> WeekNumbering {
        if s == "iso" {
            WeekNumbering::Iso
        } else {
            WeekNumbering::Us
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            WeekNumbering::Us => "us",
            WeekNumbering::Iso => "iso",
        }
    }
}

pub type TranscludedTrees = HashMap<String, Vec<Item>>;

/// `GET /api/pages/{id}` and the journal endpoints: a page with its items.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageWithItems {
    pub page: Page,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default)]
    pub created: bool,
    #[serde(default)]
    pub transcluded_trees: Option<TranscludedTrees>,
}

/// `GET /api/journals/before` and `/weekly/before`. The server and the client share the type (ADR 082).
pub use journal_api_types::JournalBefore;

/// A page that links to another, and the item that holds the link. The server and the client share the type (ADR 082).
pub use journal_api_types::BacklinkEntry;

/// `POST /api/items/export`. The server and the client share the type (ADR 082).
pub use journal_api_types::TreeExport;

/// A key press kept for an editor that was not yet on screen when it was pressed.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DeferredKey {
    pub key: String,
    pub code: String,
    pub shift_key: bool,
    pub ctrl_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
}

/// What a held key does when it is played into the text of an editor, and only edits or moves
/// inside that text. Places count characters (ADR 043).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKey {
    /// The characters from `from` to `to` go.
    Delete { from: usize, to: usize },
    /// The cursor goes to the place.
    MoveTo(usize),
}

impl DeferredKey {
    /// What the key does in `text` with the selection from `start` to `end`, as the editor
    /// would do it. None for a key the keydown handler takes: a Backspace at the start, which
    /// joins the item above, a Delete at the end, which joins the item below, and any key that
    /// is not Backspace, Delete, an arrow to the side, Home or End. Shift and Alt make the
    /// key one of the handler too.
    pub fn text_key(&self, text: &str, start: usize, end: usize) -> Option<TextKey> {
        if self.shift_key || self.alt_key {
            return None;
        }
        let len = text.chars().count();
        let selected = start != end;
        match self.key.as_str() {
            "Backspace" if selected || start > 0 => Some(TextKey::Delete {
                from: if selected { start } else { start - 1 },
                to: end,
            }),
            "Delete" if selected || end < len => Some(TextKey::Delete {
                from: start,
                to: if selected { end } else { end + 1 },
            }),
            "ArrowLeft" => Some(TextKey::MoveTo(start.saturating_sub(1))),
            "ArrowRight" => Some(TextKey::MoveTo((end + 1).min(len))),
            "Home" => Some(TextKey::MoveTo(0)),
            "End" => Some(TextKey::MoveTo(len)),
            _ => None,
        }
    }

    /// True for a key that types a character; other keys are shortcuts or navigation. A typed
    /// letter can be several characters, as an emoji with a skin tone is (ADR 043).
    pub fn is_typed_character(&self) -> bool {
        self.key.graphemes(true).count() == 1 && !self.ctrl_key && !self.alt_key && !self.meta_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn deferred(key: &str) -> DeferredKey {
        DeferredKey {
            key: key.into(),
            ..DeferredKey::default()
        }
    }

    // 複数のコードポイントでできた1文字（肌の色付きの絵文字、異体字セレクタ付きの絵文字）も、入力した文字として再生される。
    // 名前のあるキー（Enter）は、入力した文字ではない（ADR 043）。
    #[test]
    fn a_letter_of_several_characters_is_a_typed_character_but_a_named_key_is_not() {
        assert!(deferred("a").is_typed_character());
        assert!(deferred("👍🏽").is_typed_character());
        assert!(deferred("❤\u{fe0f}").is_typed_character());
        assert!(!deferred("Enter").is_typed_character());
    }

    // Backspace と Delete は、カーソルの前と後の1文字か、選んだ文字を消す。位置は文字で数え、絵文字も1文字である。
    #[test]
    fn backspace_and_delete_take_one_character_or_the_selection() {
        let text = "a👍b";
        assert_eq!(
            deferred("Backspace").text_key(text, 2, 2),
            Some(TextKey::Delete { from: 1, to: 2 })
        );
        assert_eq!(
            deferred("Delete").text_key(text, 1, 1),
            Some(TextKey::Delete { from: 1, to: 2 })
        );
        assert_eq!(
            deferred("Backspace").text_key(text, 0, 3),
            Some(TextKey::Delete { from: 0, to: 3 })
        );
        assert_eq!(
            deferred("Delete").text_key(text, 3, 3),
            None,
            "the handler joins the item below"
        );
        assert_eq!(
            deferred("Backspace").text_key(text, 0, 0),
            None,
            "the handler joins the item above"
        );
    }

    // 横の矢印は1文字、Home と End は文の端へカーソルを動かす。右の矢印は文の終わりで止まる。
    #[test]
    fn the_side_arrows_home_and_end_move_the_cursor() {
        let text = "a👍b";
        assert_eq!(
            deferred("ArrowLeft").text_key(text, 2, 2),
            Some(TextKey::MoveTo(1))
        );
        assert_eq!(
            deferred("ArrowRight").text_key(text, 1, 1),
            Some(TextKey::MoveTo(2))
        );
        assert_eq!(
            deferred("ArrowRight").text_key(text, 3, 3),
            Some(TextKey::MoveTo(3))
        );
        assert_eq!(
            deferred("Home").text_key(text, 2, 2),
            Some(TextKey::MoveTo(0))
        );
        assert_eq!(
            deferred("End").text_key(text, 0, 0),
            Some(TextKey::MoveTo(3))
        );
    }

    // Shift か Alt を押したキーと、ほかのキーは、文の中では何もせず、キーのハンドラに任される。
    #[test]
    fn other_keys_are_left_to_the_key_handler() {
        let shifted = DeferredKey {
            key: "ArrowLeft".into(),
            shift_key: true,
            ..DeferredKey::default()
        };
        assert_eq!(shifted.text_key("ab", 1, 1), None);
        assert_eq!(deferred("Enter").text_key("ab", 1, 1), None);
    }

    // サーバーが返す JSON を Item にデシリアライズすると、camelCase のフィールドと properties のアクセサがサーバーの形どおりに読み取られる。
    #[test]
    fn item_json_matches_server_shape() {
        let json = r##"{"id":"a","pageId":"p","dirPath":"","name":"2026-02-10","type":"journal",
            "createdDate":1,"modifiedDate":2,"idx":"a0","level":1,"body":"# Hi","ast":"{}",
            "collapsed":false,"types":["heading","task"],"properties":{"tags":["x","y"],"zoom":"1.5","end":"2026-01-01T00:00:00Z"},
            "transcludeItemId":null}"##;
        let item: Item = serde_json::from_str(json).unwrap();
        assert_eq!(item.page_id, "p");
        assert_eq!(item.item_type, "journal");
        assert!(item.is_task());
        assert_eq!(item.tags(), vec!["x", "y"]);
        assert_eq!(item.zoom(), 1.5);
        assert_eq!(item.prop_str("end"), Some("2026-01-01T00:00:00Z"));
        assert!(item.is_daily_journal_item());
        assert_eq!(item.heading_level_of_body(), 1);
    }

    // プロパティ値は、ネイティブのシェルとの境界の形式（bincode）に書いて、同じ値に読み戻せる（ADR 024）。
    #[test]
    fn round_trips_through_the_format_of_the_native_border() {
        use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
        for value in [
            PropertyValue::Text("1.5".into()),
            PropertyValue::List(vec!["x".into(), "y".into()]),
        ] {
            let mut bytes = Vec::new();
            BincodeFfiFormat::serialize(&mut bytes, &value).unwrap();
            let back: PropertyValue = BincodeFfiFormat::deserialize(&bytes).unwrap();
            assert_eq!(back, value);
        }
    }

    // プロパティを持つ項目とページは、ネイティブのシェルとの境界の形式（bincode）に書いて、同じ値に読み戻せる（ADR 024）。
    #[test]
    fn items_and_pages_with_properties_round_trip_through_the_native_border() {
        use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
        let properties = BTreeMap::from([
            ("zoom".to_string(), PropertyValue::Text("1.5".into())),
            ("tags".to_string(), PropertyValue::List(vec!["x".into()])),
        ]);
        let item = Item {
            properties: properties.clone(),
            ..Default::default()
        };
        let page = Page {
            properties,
            ..Default::default()
        };
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &(&item, &page)).unwrap();
        let back: (Item, Page) = BincodeFfiFormat::deserialize(&bytes).unwrap();
        assert_eq!(back, (item, page));
    }

    // 文字列として読むプロパティがリストなら値はなく、リストとして読むプロパティが文字列なら空のリストになる。
    #[test]
    fn a_value_of_the_other_shape_reads_as_unset() {
        let item = Item {
            properties: BTreeMap::from([
                ("zoom".to_string(), PropertyValue::List(vec!["2".into()])),
                ("tags".to_string(), PropertyValue::Text("x".into())),
            ]),
            ..Default::default()
        };
        assert_eq!(item.prop_str("zoom"), None);
        assert_eq!(item.zoom(), 1.0);
        assert!(item.tags().is_empty());
    }
}
