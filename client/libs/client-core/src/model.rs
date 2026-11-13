//! The data the server sends and receives, in the same JSON shape as the Angular models.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// An item as the server returns it (`presentation/dto.rs` in journal/journal-server).
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub page_id: String,
    #[serde(default)]
    pub dir_path: String,
    #[serde(default)]
    pub name: String,
    /// "journal" or "doc", from the parent page.
    #[serde(rename = "type", default)]
    pub item_type: String,
    #[serde(default)]
    pub created_date: i64,
    #[serde(default)]
    pub modified_date: i64,
    #[serde(default)]
    pub idx: String,
    pub level: i32,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub types: Vec<String>,
    /// Property values are strings or lists of strings.
    #[serde(default)]
    pub properties: BTreeMap<String, PropertyValue>,
    #[serde(default)]
    pub transclude_item_id: Option<String>,
}

/// The value of a property. The server has only these two shapes.
///
/// It is written in two ways. JSON has it as the server does: a string, or an array of
/// strings. A format that does not describe itself, like the bincode of the native border
/// (ADR 024), cannot tell the two apart by looking, so there it is an enum with a tag.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyValue {
    Text(String),
    List(Vec<String>),
}

impl PropertyValue {
    /// The text, or None for a list.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            PropertyValue::Text(text) => Some(text),
            PropertyValue::List(_) => None,
        }
    }

    /// The list, or None for a text.
    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            PropertyValue::Text(_) => None,
            PropertyValue::List(list) => Some(list),
        }
    }
}

// Short ways to write a value in a test. The app only makes values from owned strings.
#[cfg(test)]
impl From<&str> for PropertyValue {
    fn from(text: &str) -> Self {
        PropertyValue::Text(text.to_string())
    }
}

#[cfg(test)]
impl<const N: usize> From<[&str; N]> for PropertyValue {
    fn from(list: [&str; N]) -> Self {
        PropertyValue::List(list.iter().map(|text| text.to_string()).collect())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "PropertyValue", untagged)]
enum PropertyValueAsJson {
    Text(String),
    List(Vec<String>),
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "PropertyValue", rename = "PropertyValue")]
enum PropertyValueWithTag {
    Text(String),
    List(Vec<String>),
}

impl Serialize for PropertyValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            PropertyValueAsJson::serialize(self, serializer)
        } else {
            PropertyValueWithTag::serialize(self, serializer)
        }
    }
}

impl<'de> Deserialize<'de> for PropertyValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            PropertyValueAsJson::deserialize(deserializer)
        } else {
            PropertyValueWithTag::deserialize(deserializer)
        }
    }
}

/// Items are shared between the state, the view model and undo snapshots, so they are
/// reference counted: a change makes a new item instead of touching the shared one.
pub type ItemRef = Arc<Item>;

/// The task state of an item, with the times and the plan that go with it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TaskState {
    pub task: bool,
    pub begin: Option<String>,
    pub end: Option<String>,
    pub plan: Option<String>,
}

impl Item {
    pub fn is_task(&self) -> bool {
        self.types.iter().any(|t| t == "task")
    }

    pub fn task_state(&self) -> TaskState {
        let text_of = |key| self.prop_str(key).map(str::to_string);
        TaskState {
            task: self.is_task(),
            begin: text_of("begin"),
            end: text_of("end"),
            plan: text_of("plan"),
        }
    }

    pub fn has_type(&self, name: &str) -> bool {
        self.types.iter().any(|t| t == name)
    }

    /// A string property, or None when unset or not a string.
    pub fn prop_str(&self, key: &str) -> Option<&str> {
        self.properties.get(key).and_then(PropertyValue::as_text)
    }

    /// A list property, or an empty list when unset or not a list.
    pub fn prop_list(&self, key: &str) -> Vec<String> {
        self.properties
            .get(key)
            .and_then(PropertyValue::as_list)
            .map(<[String]>::to_vec)
            .unwrap_or_default()
    }

    /// The hashtags of the item, as kept in its tags property.
    pub fn tags(&self) -> Vec<String> {
        self.prop_list("tags")
    }

    /// The zoom of the item as stored in its properties; 1 when unset.
    pub fn zoom(&self) -> f64 {
        self.prop_str("zoom")
            .and_then(|z| z.parse::<f64>().ok())
            .filter(|z| *z != 0.0)
            .unwrap_or(1.0)
    }

    /// True for an item of a daily journal page (named YYYY-MM-DD).
    pub fn is_daily_journal_item(&self) -> bool {
        self.item_type == "journal"
            && crate::util::filename::classify_journal_file_name(&self.name)
                == crate::util::filename::JournalType::Daily
    }

    /// The heading level of a body that starts with `#`..`#####` followed by a space; 0 otherwise.
    pub fn heading_level_of_body(&self) -> u8 {
        let trimmed = self.body.trim_start();
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=5).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            hashes as u8
        } else {
            0
        }
    }
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub id: String,
    #[serde(default)]
    pub parent_id: String,
    #[serde(default)]
    pub dir_path: String,
    pub name: String,
    /// "journal" or "doc".
    #[serde(rename = "type", default)]
    pub page_type: String,
    #[serde(default)]
    pub created_date: i64,
    #[serde(default)]
    pub modified_date: i64,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Property values are strings or lists of strings; empty in page lists.
    #[serde(default)]
    pub properties: BTreeMap<String, PropertyValue>,
}

impl Page {
    pub fn is_journal(&self) -> bool {
        self.page_type == "journal"
    }

    /// `dirPath` without its trailing slash plus the name, the form used in URLs.
    pub fn canonical_path(&self) -> String {
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

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub page_id: String,
    pub page_title: String,
    /// Empty for page hits.
    #[serde(default)]
    pub item_id: String,
    #[serde(default)]
    pub snippet: String,
    #[serde(default)]
    pub score: f64,
}

/// Where an item goes: first on the page, last, or after another item.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemPosition {
    First,
    Last,
    After(String),
}

impl ItemPosition {
    pub fn after(id: impl Into<String>) -> ItemPosition {
        ItemPosition::After(id.into())
    }

    /// The form the server takes: `first`, `last` or `after:<id>`.
    pub fn to_api(&self) -> String {
        match self {
            ItemPosition::First => "first".to_string(),
            ItemPosition::Last => "last".to_string(),
            ItemPosition::After(id) => format!("after:{id}"),
        }
    }

    pub fn parse(s: &str) -> Option<ItemPosition> {
        match s {
            "first" => Some(ItemPosition::First),
            "last" => Some(ItemPosition::Last),
            _ => s
                .strip_prefix("after:")
                .map(|id| ItemPosition::After(id.to_string())),
        }
    }
}

/// The form of a format that does not describe itself, like the bincode of the native border
/// (ADR 024). The types of a native shell are generated from the shape of the Rust type, so
/// the bytes have to follow that shape too.
#[derive(Serialize, Deserialize)]
#[serde(remote = "ItemPosition", rename = "ItemPosition")]
enum ItemPositionWithTag {
    First,
    Last,
    After(String),
}

impl Serialize for ItemPosition {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_api())
        } else {
            ItemPositionWithTag::serialize(self, serializer)
        }
    }
}

impl<'de> Deserialize<'de> for ItemPosition {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if !deserializer.is_human_readable() {
            return ItemPositionWithTag::deserialize(deserializer);
        }
        let s = String::deserialize(deserializer)?;
        ItemPosition::parse(&s).ok_or_else(|| D::Error::custom(format!("bad position: {s}")))
    }
}

/// One operation of a batch. The server applies the operations of a batch in their order, all
/// of them or none. The names are the ones the server reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum BatchOp {
    /// Makes an item with the id it names, as the undo of a deletion makes the item again.
    #[serde(rename_all = "camelCase")]
    Create {
        id: String,
        page_id: String,
        position: ItemPosition,
        level: i32,
        body: String,
    },
    /// Writes the fields given. A task state is written whole, with its times.
    Update {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        level: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        collapsed: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task: Option<TaskState>,
    },
    /// Moves items, in their order, to a position on a page, as the undo of a move does.
    #[serde(rename_all = "camelCase")]
    Move {
        ids: Vec<String>,
        page_id: String,
        position: ItemPosition,
    },
    /// Deletes an item. The server deletes the items of a batch last, as one group.
    Delete { id: String },
}

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

/// A file the server saved.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Uploaded {
    /// The name of the file on the server. The server makes it up. See ADR-022.
    pub file_name: String,
    /// The name the file came with.
    pub original_name: String,
}

/// What a paste came to on the server.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pasted {
    /// The item pasted into, then the items created below it, in their order on the page.
    pub items: Vec<Item>,
    pub cursor: PasteCursor,
}

/// What a batch came to on the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Batched {
    /// The items the batch wrote, as they are now.
    pub items: Vec<Item>,
    /// The empty items the server made on the pages the batch left without an item.
    pub created: Vec<Item>,
}

/// What sending an item to the stash came to on the server.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stashed {
    /// The `# Stash` heading the items went under.
    pub heading: Item,
    /// True when the heading was made for this.
    pub created: bool,
    /// The moved items, as they are now.
    pub items: Vec<Item>,
}

/// What a split came to on the server.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Split {
    /// The item that was split, with the text it keeps.
    pub item: Item,
    /// The item made beside it.
    pub created: Item,
}

/// Where the cursor goes after a paste: right after the pasted text.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteCursor {
    pub item_id: String,
    pub position: usize,
}

/// A property definition (`GET /api/config/property-definitions`, the `definitions` list).
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDefinition {
    pub key: String,
    #[serde(default)]
    pub value_type: String,
    /// Written by the server itself; shown read-only, cannot be defined again.
    #[serde(default)]
    pub system: bool,
    /// May be set on a page.
    #[serde(default)]
    pub page: bool,
}

/// The types a user may give a new key, in the order the settings offer them.
pub const USER_VALUE_TYPES: [&str; 6] =
    ["text", "number", "checkbox", "date", "datetime", "bounds"];

/// The server's settings (`GET /api/config`).
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    #[serde(default)]
    pub markdown_dir: String,
    #[serde(default)]
    pub week_numbering: String,
    #[serde(default)]
    pub logical_outdent: bool,
    /// Directory of the tag pages relative to the pages root, `tags/` style; empty for the root.
    #[serde(default)]
    pub tags_dir: String,
    /// The page the Home button opens, as the target of a wiki link; empty for today.
    #[serde(default)]
    pub home_link: String,
}

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

/// `GET /api/journals/before` and `/weekly/before`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalBefore {
    pub found: bool,
    #[serde(default)]
    pub page: Option<Page>,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default)]
    pub transcluded_trees: Option<TranscludedTrees>,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkEntry {
    pub source_page: Page,
    /// None when the source page links by the tag in its tags property.
    #[serde(default)]
    pub source_item: Option<Item>,
}

/// `POST /api/items/export`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeExport {
    pub tree_markdown: String,
    pub flat_markdown: String,
}

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

impl DeferredKey {
    /// True for a key that types a character; other keys are shortcuts or navigation.
    pub fn is_typed_character(&self) -> bool {
        self.key.chars().count() == 1 && !self.ctrl_key && !self.alt_key && !self.meta_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // JSON の文字列は、文字列のプロパティ値として読み取られる。
    #[test]
    fn reads_a_json_string_as_a_text_value() {
        let value: PropertyValue = serde_json::from_str(r#""1.5""#).unwrap();
        assert_eq!(value, PropertyValue::Text("1.5".into()));
    }

    // プロパティ値は、サーバーと同じ形の JSON に書かれる。文字列は文字列に、リストは配列になる。
    #[test]
    fn writes_json_in_the_shape_of_the_server() {
        let text = PropertyValue::Text("1.5".into());
        let list = PropertyValue::List(vec!["x".into(), "y".into()]);
        assert_eq!(serde_json::to_string(&text).unwrap(), r#""1.5""#);
        assert_eq!(serde_json::to_string(&list).unwrap(), r#"["x","y"]"#);
    }

    // JSON の文字列の配列は、リストのプロパティ値として読み取られる。
    #[test]
    fn reads_a_json_array_of_strings_as_a_list_value() {
        let value: PropertyValue = serde_json::from_str(r#"["x","y"]"#).unwrap();
        assert_eq!(value, PropertyValue::List(vec!["x".into(), "y".into()]));
    }

    // 文字列でも文字列の配列でもない JSON は、プロパティ値として読み取れない。サーバーはこの 2 つの形しか返さない。
    #[test]
    fn refuses_json_of_any_other_shape() {
        for json in ["1.5", "true", "null", "[1]", r#"{"a":"b"}"#] {
            assert!(
                serde_json::from_str::<PropertyValue>(json).is_err(),
                "{json}"
            );
        }
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

    // ItemPosition は first・last・after:<id> の文字列を解析して同じ文字列に戻し、after は "after:x" という JSON 文字列にシリアライズされる。
    #[test]
    fn position_round_trips() {
        for s in ["first", "last", "after:2026-01-01-00-00-00-AAAAAAA"] {
            assert_eq!(ItemPosition::parse(s).unwrap().to_api(), s);
        }
        assert_eq!(
            serde_json::to_string(&ItemPosition::after("x")).unwrap(),
            "\"after:x\""
        );
    }
}
