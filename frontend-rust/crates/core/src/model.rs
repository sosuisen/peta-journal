//! The data the server sends and receives, in the same JSON shape as the Angular models.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// An item as the server returns it (`presentation/dto.rs` in journal-server-rust).
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
    pub ast: String,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub types: Vec<String>,
    /// Property values are strings or lists of strings.
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
    #[serde(default)]
    pub transclude_item_id: Option<String>,
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
        self.properties.get(key).and_then(|v| v.as_str())
    }

    /// A list property, or an empty list when unset or not a list.
    pub fn prop_list(&self, key: &str) -> Vec<String> {
        match self.properties.get(key) {
            Some(Value::Array(values)) => values
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        }
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
    pub properties: BTreeMap<String, Value>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexSortKey {
    #[default]
    Name,
    Modified,
    /// The `published` property; a page without one falls back to `modified`.
    Published,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// The sort of one directory's index list. The shell keeps one per directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IndexSort {
    pub key: IndexSortKey,
    pub order: SortOrder,
}

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

impl Serialize for ItemPosition {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_api())
    }
}

impl<'de> Deserialize<'de> for ItemPosition {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ItemPosition::parse(&s).ok_or_else(|| D::Error::custom(format!("bad position: {s}")))
    }
}

/// One item of Flat Markdown as split by the server. `id` is None when the metadata line has none.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ParsedFlatItem {
    pub level: i32,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
    #[serde(default)]
    pub body: String,
}

/// A property definition (`GET /api/config/property-definitions`, the `definitions` list).
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
    #[serde(default)]
    pub ast: String,
    pub flat_markdown: String,
}

/// A key press kept for an editor that was not yet on screen when it was pressed.
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
