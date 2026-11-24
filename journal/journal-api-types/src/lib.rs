//! The bodies of the requests to the journal, and the messages of its event stream. The
//! journal reads the requests with these types and the frontend core writes them with the same
//! types. The messages go the other way. So the two cannot name a field differently.
//!
//! The crate has no dependency but serde: the frontend core is also built for wasm, where the
//! journal core, with its database, cannot go.
//!
//! The doc comment of a field is also what an MCP client reads about it: the journal server
//! takes some of these types as the arguments of its MCP tools (the `schemars` feature).
//!
//! The types are as loose as the journal reads them. A field the journal can do without is an
//! `Option` or has a default, and the journal says what is wrong with a value. A field that is
//! not set is not written, so a request says only what its sender set.

/// The header that carries the ID a client names itself with (ADR-015). The journal refuses
/// a change without it. Lower case, as HTTP/2 and the header names of the server write it.
pub const CLIENT_ID_HEADER: &str = "x-client-id";

pub mod routes;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

fn is_false(value: &bool) -> bool {
    !*value
}

// ---- pages ----

/// The page to look for, without making it: a journal by its name, or else a document page
/// by its title and folder. Going back to a page that sync deleted looks for it this way.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct FindPageQuery {
    /// Name of a journal (e.g. '2026-11-20' or '2026-W47')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal: Option<String>,
    /// Title of a document page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Directory path of a document page (e.g. 'projects/'). Empty or left out for the root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct GetOrCreateDocInput {
    /// Title of the document (used as file name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Directory path (e.g. 'projects/'). Empty or left out for the root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct UpdatePageInput {
    /// New title. Left out to keep the current one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// New directory path. Left out to keep the current one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir_path: Option<String>,
    /// Deletes another page that has the new name, instead of refusing the update.
    #[serde(skip_serializing_if = "is_false")]
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct DemotePageInput {
    /// Deletes another page that has the new name, instead of refusing the demotion.
    #[serde(skip_serializing_if = "is_false")]
    pub overwrite: bool,
}

/// The value of a property of a page or an item.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PropertyInput {
    /// Value type: 'text', 'number', 'checkbox', 'date', 'datetime', or 'bounds' (four numbers
    /// 'x,y,width,height'). No type means `text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    /// Property value as a string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

// ---- journals ----

/// The items that move to the journal of a day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct MoveToJournalInput {
    /// IDs of the items to move, in the order of the page
    pub item_ids: Vec<String>,
    /// Position in the journal: 'first', 'last', or 'after:<id>'
    pub position: ItemPosition,
    /// The text the editor of the first item has now, when it is not saved yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

// ---- items ----

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CreateItemInput {
    /// ID of the page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_id: Option<String>,
    /// Position: 'first', 'last', or 'after:<id>'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<ItemPosition>,
    /// Indentation level (1-based)
    pub level: i32,
    /// Markdown body text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// The ID the client made for the item. The journal makes one when it is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct UpdateItemInput {
    /// New markdown body. Left out to keep the current one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// New indentation level (1-based). Left out to keep the current one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    /// New collapsed state. Left out to keep the current one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct MoveItemsInput {
    /// IDs of the items to move
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_ids: Option<Vec<String>>,
    /// ID of the target page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_page_id: Option<String>,
    /// Position: 'first', 'last', or 'after:<id>'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<ItemPosition>,
    /// Number to add to the level of each item. Left out or 0 for no change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_delta: Option<i32>,
    /// The body the editor of the first item has now, when it is not saved yet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// The items an operation is about.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ItemIdsInput {
    /// IDs of the items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct BatchInput {
    /// The operations, done in this order in one transaction
    pub ops: Vec<BatchOpInput>,
}

/// An item as the server answers it, with the fields the clients read (ADR 082). The answer of
/// the server has more fields, which a client passes over.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub page_id: String,
    pub dir_path: String,
    pub name: String,
    /// "journal" or "doc", from the parent page.
    #[serde(rename = "type")]
    pub item_type: String,
    pub created_date: i64,
    pub modified_date: i64,
    pub idx: String,
    pub level: i32,
    pub body: String,
    pub collapsed: bool,
    pub types: Vec<String>,
    /// Property values are strings or lists of strings.
    pub properties: BTreeMap<String, PropertyValue>,
    pub transclude_item_id: Option<String>,
}

/// A page as the server answers it, with the fields the clients read (ADR 082). The answer of
/// the server has more fields, which a client passes over.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub id: String,
    pub parent_id: String,
    pub dir_path: String,
    pub name: String,
    /// "journal" or "doc".
    #[serde(rename = "type")]
    pub page_type: String,
    pub created_date: i64,
    pub modified_date: i64,
    /// Property values are strings or lists of strings; empty in page lists.
    pub properties: BTreeMap<String, PropertyValue>,
}

/// A page or an item that a search found. The server and the clients share it (ADR 082).
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub page_id: String,
    pub page_title: String,
    /// Empty for page hits.
    pub item_id: String,
    pub snippet: String,
    pub score: f32,
}

/// The folder the person chose in the dialog of the server. It does not cross the native border.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FolderChoice {
    /// True when a folder was chosen.
    pub selected: bool,
    /// The chosen folder, or an empty string.
    pub path: String,
}

/// A file the server saved.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Uploaded {
    /// The name of the file on the server. The server makes it up. See ADR-022.
    pub file_name: String,
    /// The name the file came with.
    pub original_name: String,
}

/// Where the cursor goes after a paste: right after the pasted text.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteCursor {
    pub item_id: String,
    pub position: usize,
}

/// A property definition (`GET /api/config/property-definitions`, the `definitions` list).
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDefinition {
    pub key: String,
    pub value_type: String,
    /// Written by the server itself; shown read-only, cannot be defined again.
    pub system: bool,
    /// May be set on a page.
    pub page: bool,
}

/// The server's settings (`GET /api/config`), with the fields the clients read (ADR 082). The
/// answer of the server has more fields, which a client passes over.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub markdown_dir: String,
    pub week_numbering: String,
    pub logical_outdent: bool,
    /// Directory of the tag pages relative to the pages root, `tags/` style; empty for the root.
    pub tags_dir: String,
    /// The page the Home button opens, as the target of a wiki link; empty for today.
    pub home_link: String,
}

/// `POST /api/items/export`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeExport {
    pub tree_markdown: String,
    pub flat_markdown: String,
}

// The answers below hold items or pages. The server writes them with its own item and page,
// which have more fields; a client reads them with `Item` and `Page`, the default (ADR 082).

/// What a paste came to on the server.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pasted<I = Item> {
    /// The item pasted into, then the items created below it, in their order on the page.
    pub items: Vec<I>,
    pub cursor: PasteCursor,
}

/// What a batch came to on the server. It does not cross the native border.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Batched<I = Item> {
    /// The items the batch wrote, as they are now.
    pub items: Vec<I>,
    /// The empty items the server made on the pages the batch left without an item.
    pub created: Vec<I>,
}

/// What sending an item to the stash came to on the server.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stashed<I = Item> {
    /// The `# Stash` heading the items went under.
    pub heading: I,
    /// True when the heading was made for this.
    pub created: bool,
    /// The moved items, as they are now.
    pub items: Vec<I>,
}

/// What a split came to on the server.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Split<I = Item> {
    /// The item that was split, with the text it keeps.
    pub item: I,
    /// The item made beside it.
    pub created: I,
}

/// `GET /api/journals/before` and `/weekly/before`. It does not cross the native border.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalBefore<P = Page, I = Item> {
    pub found: bool,
    pub page: Option<P>,
    pub items: Vec<I>,
    /// The trees of the items that the transclusions show, by the id of their source.
    pub transcluded_trees: BTreeMap<String, Vec<I>>,
}

/// `GET /api/pages/{id}`: a page with its items.
// facet needs `I: 'static` for the map of trees; every item type is.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageWithItems<P = Page, I: 'static = Item> {
    pub page: P,
    pub items: Vec<I>,
    /// The trees of the items that the transclusions show, by the id of their source.
    pub transcluded_trees: BTreeMap<String, Vec<I>>,
}

/// A journal that was found or made, with its items.
// facet needs `I: 'static` for the map of trees; every item type is.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalPage<P = Page, I: 'static = Item> {
    pub page: P,
    pub items: Vec<I>,
    /// True when the journal was made for this.
    pub created: bool,
    /// The trees of the items that the transclusions show, by the id of their source.
    pub transcluded_trees: BTreeMap<String, Vec<I>>,
}

/// A document page that was found or made. It does not cross the native border.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocPage<P = Page, I = Item> {
    pub page: P,
    pub items: Vec<I>,
    /// True when the page was made for this.
    pub created: bool,
}

/// The journal that items moved to, and the items as they are now. It does not cross the
/// native border.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedToJournal<P = Page, I = Item> {
    pub page: P,
    /// True when the journal was made for this.
    pub created: bool,
    pub items: Vec<I>,
}

/// A page that links to another, and the item that holds the link.
#[cfg_attr(feature = "facet", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkEntry<P = Page, I = Item> {
    pub source_page: P,
    /// None when the source page links by the tag in its tags property.
    pub source_item: Option<I>,
}

/// The value of a property. The server has only these two shapes.
///
/// It is written in two ways. JSON has it as the server does: a string, or an array of
/// strings. A format that does not describe itself, like the bincode of the native border
/// (ADR 024), cannot tell the two apart by looking, so there it is an enum with a tag.
#[cfg_attr(feature = "facet", derive(facet::Facet), repr(C))]
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

/// A short way to write a text value.
impl From<&str> for PropertyValue {
    fn from(text: &str) -> Self {
        PropertyValue::Text(text.to_string())
    }
}

/// A short way to write a list value.
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

/// Written by hand: a derive would read the attributes of the type, but JSON has the form of
/// `PropertyValueAsJson`.
#[cfg(feature = "schemars")]
impl schemars::JsonSchema for PropertyValue {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "PropertyValue".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "description": "The value of a property: a text, or a list of texts.",
            "anyOf": [
                { "type": "string" },
                { "type": "array", "items": { "type": "string" } }
            ]
        })
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

/// Where an item goes on a page: first, last, or after another item. JSON has it as the text
/// `first`, `last` or `after:<id>` (ADR 081).
///
/// A format that does not describe itself, like the bincode of the native border (ADR 024),
/// has it as an enum with a tag instead.
#[cfg_attr(feature = "facet", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemPosition {
    First,
    Last,
    After(String),
}

/// The form of a format that does not describe itself. The types of a native shell are
/// generated from the shape of the Rust type, so the bytes have to follow that shape too.
#[derive(Serialize, Deserialize)]
#[serde(remote = "ItemPosition", rename = "ItemPosition")]
enum ItemPositionWithTag {
    First,
    Last,
    After(String),
}

impl ItemPosition {
    pub fn after(id: impl Into<String>) -> ItemPosition {
        ItemPosition::After(id.into())
    }

    fn parse(text: &str) -> Option<ItemPosition> {
        match text {
            "first" => Some(ItemPosition::First),
            "last" => Some(ItemPosition::Last),
            _ => text
                .strip_prefix("after:")
                .map(|id| ItemPosition::After(id.to_string())),
        }
    }
}

/// The text JSON has: `first`, `last` or `after:<id>`.
impl std::fmt::Display for ItemPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemPosition::First => f.write_str("first"),
            ItemPosition::Last => f.write_str("last"),
            ItemPosition::After(id) => write!(f, "after:{id}"),
        }
    }
}

impl Serialize for ItemPosition {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.collect_str(self)
        } else {
            ItemPositionWithTag::serialize(self, serializer)
        }
    }
}

/// Written by hand: a derive would show MCP an enum, but the tools take the text.
#[cfg(feature = "schemars")]
impl schemars::JsonSchema for ItemPosition {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ItemPosition".into()
    }

    fn inline_schema() -> bool {
        true
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "Position: 'first', 'last', or 'after:<id>'"
        })
    }
}

impl<'de> Deserialize<'de> for ItemPosition {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if !deserializer.is_human_readable() {
            return ItemPositionWithTag::deserialize(deserializer);
        }
        let text = String::deserialize(deserializer)?;
        ItemPosition::parse(&text).ok_or_else(|| D::Error::custom(format!("bad position: {text}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
/// One operation of a batch: 'create', 'update', 'move' or 'delete', named by `op`.
pub enum BatchOpInput {
    #[serde(rename_all = "camelCase")]
    Create {
        id: String,
        page_id: String,
        position: ItemPosition,
        level: i32,
        #[serde(default)]
        body: String,
        /// ID of the source item the item shows, as the undo of a deletion makes a
        /// transclusion again
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transclude_item_id: Option<String>,
        /// The properties of the item, as the undo of a deletion makes them again. The tags
        /// are left out: they come from the hashtags of the body
        #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
        properties: std::collections::BTreeMap<String, PropertyValue>,
    },
    Update {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        level: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        collapsed: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task: Option<TaskStateInput>,
    },
    #[serde(rename_all = "camelCase")]
    Move {
        ids: Vec<String>,
        page_id: String,
        position: ItemPosition,
    },
    Delete {
        id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct TaskStateInput {
    /// Whether the item is a task
    pub task: bool,
    /// The time the task was started, as the value of the 'begin' property. Left out for none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub begin: Option<String>,
    /// The time the task was completed, as the value of the 'end' property. Left out for none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    /// The day the task is planned for, as the value of the 'plan' property. Left out for none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct MergeItemsInput {
    /// ID of the item that goes away. Its text is added to the other item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    /// ID of the item that takes the text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub into_id: Option<String>,
    /// The body an editor has now for the item that goes away, when it is not saved yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// The same for the item that takes the text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub into_body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PasteInput {
    /// The body of the item as it is now
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Where the text goes in the body, counted in Unicode code points
    pub cursor: usize,
    /// The text to paste
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// `auto`, `flat` or `raw`. No mode means `auto`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SplitInput {
    /// The body of the item as it is now
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Where the body is split, counted in Unicode code points
    pub cursor: usize,
    /// The transclusion that shows the item, for an item of another page shown below it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct StashInput {
    /// The text the editor of the item has now, when it is not saved yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ShiftLevelsInput {
    /// IDs of the items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_ids: Option<Vec<String>>,
    /// Number to add to the level of each item: 1 to indent, -1 to outdent
    pub delta: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct TransclusionInput {
    /// ID of the source item to transclude
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_item_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct TreeMarkdownInput {
    /// Tree markdown (an indented list)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_markdown: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct FlatMarkdownInput {
    /// Flat Markdown: items separated by '<!-- @@ -->' marker lines
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

// ---- atlas ----

/// A link card: the page or the address it shows, and where it is.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AtlasLinkInput {
    /// What the link card shows: the name of a page, or a URL
    #[serde(skip_serializing_if = "String::is_empty")]
    pub target: String,
    /// Where the card is, as four numbers 'x,y,width,height'
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bounds: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AtlasGroupInput {
    /// Each a card by its item id, or a page card by the id of its page
    pub cards: Vec<String>,
    /// The card or the group the cards are put together with
    #[serde(skip_serializing_if = "String::is_empty")]
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AtlasTitleInput {
    /// The side of the group its title is on: 'top', 'bottom', 'left' or 'right'
    #[serde(skip_serializing_if = "String::is_empty")]
    pub side: String,
    /// The title of the group
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AtlasCalloutInput {
    /// Where the callout is, as four numbers 'x,y,width,height'
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bounds: String,
    /// The side of the callout its bubble is on: 'top', 'bottom', 'left' or 'right'
    #[serde(skip_serializing_if = "String::is_empty")]
    pub side: String,
    /// The text of the callout
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
}

// ---- settings ----

/// The settings to change. A setting not given stays as it is.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct UpdateConfigInput {
    /// Markdown export directory path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown_dir: Option<String>,
    /// Week numbering system: 'us' or 'iso'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub week_numbering: Option<String>,
    /// Logical outdent: move children with parent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_outdent: Option<bool>,
    /// Directory of the tag pages relative to the pages root, e.g. 'tags/' (empty for the root)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_dir: Option<String>,
    /// The page the Home button opens, as the target of a wiki link, e.g. 'today' (empty for today's journal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AddPropertyDefinitionInput {
    /// The key of the property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Value type: 'text', 'number', 'checkbox', 'date', 'datetime', or 'bounds'. Left out, the name of the key gives it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    /// Whether the key may be set on a page; true unless given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<bool>,
}

// ---- what the journal server does for its clients ----

/// The answer of the user to a question of an extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerConfirmInput {
    pub ok: bool,
}

// ---- the event stream ----

/// What the journal tells its clients on the event stream (`/api/events`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    /// A page changed, so a client that shows it reads it again.
    #[serde(rename_all = "camelCase")]
    PagesUpdated {
        page_id: String,
        source: ChangeSource,
        /// The client that made the change, for a change of a client (ADR-015).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_id: Option<String>,
    },
    /// An extension asks the user. The answer goes back with the ID of the question.
    #[serde(rename_all = "camelCase")]
    Confirm { request_id: String, message: String },
    /// The property definitions changed, so the clients read them again (ADR 046).
    PropertyDefinitionsUpdated,
}

/// Where the change of a page came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeSource {
    /// A client, through the API.
    App,
    /// An MCP tool.
    Mcp,
    /// A file of the markdown folder.
    File,
    /// An extension, which wrote items after the answer of a tag add.
    Extension,
    /// The journal itself, which wrote an atlas.
    Server,
    /// A source that a newer journal names and this client does not know.
    #[serde(other)]
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ページの変更の知らせは、変更したクライアントがいるときだけ、そのIDを書く。
    #[test]
    fn a_change_of_a_page_names_its_client_only_when_there_is_one() {
        let by_client = ClientMessage::PagesUpdated {
            page_id: "p".into(),
            source: ChangeSource::App,
            client_id: Some("c".into()),
        };
        let by_file = ClientMessage::PagesUpdated {
            page_id: "p".into(),
            source: ChangeSource::File,
            client_id: None,
        };

        assert_eq!(
            serde_json::to_value(&by_client).unwrap(),
            json!({ "type": "pagesUpdated", "pageId": "p", "source": "app", "clientId": "c" })
        );
        assert_eq!(
            serde_json::to_value(&by_file).unwrap(),
            json!({ "type": "pagesUpdated", "pageId": "p", "source": "file" })
        );
    }

    // 知らない出どころの変更も、ページの変更として読まれる。
    #[test]
    fn a_change_from_a_source_that_is_not_known_is_still_read() {
        let read: ClientMessage = serde_json::from_value(
            json!({ "type": "pagesUpdated", "pageId": "p", "source": "later" }),
        )
        .unwrap();

        assert_eq!(
            read,
            ClientMessage::PagesUpdated {
                page_id: "p".into(),
                source: ChangeSource::Other,
                client_id: None,
            }
        );
    }

    // 質問と、定義の変更の知らせは、書いた形のまま読み戻される。
    #[test]
    fn a_question_and_a_change_of_the_definitions_are_read_back_as_written() {
        let question = ClientMessage::Confirm {
            request_id: "r".into(),
            message: "Go on?".into(),
        };

        let written = serde_json::to_value(&question).unwrap();

        assert_eq!(
            written,
            json!({ "type": "confirm", "requestId": "r", "message": "Go on?" })
        );
        assert_eq!(
            serde_json::from_value::<ClientMessage>(written).unwrap(),
            question
        );
        assert_eq!(
            serde_json::to_value(ClientMessage::PropertyDefinitionsUpdated).unwrap(),
            json!({ "type": "propertyDefinitionsUpdated" })
        );
    }

    // 設定していない欄は、要求に書かれない。
    #[test]
    fn a_field_that_is_not_set_is_not_written() {
        let update = UpdatePageInput {
            title: Some("New".into()),
            ..Default::default()
        };
        let link = AtlasLinkInput {
            target: "Page".into(),
            ..Default::default()
        };

        assert_eq!(
            serde_json::to_value(update).unwrap(),
            json!({ "title": "New" })
        );
        assert_eq!(
            serde_json::to_value(link).unwrap(),
            json!({ "target": "Page" })
        );
    }

    // 上書きの指定は、指定したときだけ要求に書かれる。
    #[test]
    fn the_overwrite_is_written_only_when_it_is_asked_for() {
        let asked = DemotePageInput { overwrite: true };
        let not_asked = DemotePageInput { overwrite: false };

        assert_eq!(
            serde_json::to_value(asked).unwrap(),
            json!({ "overwrite": true })
        );
        assert_eq!(serde_json::to_value(not_asked).unwrap(), json!({}));
    }

    // 書いた要求は、同じ値に読み戻される。
    #[test]
    fn a_written_request_is_read_back_as_it_was() {
        let batch = BatchInput {
            ops: vec![
                BatchOpInput::Create {
                    id: "i".into(),
                    page_id: "p".into(),
                    position: ItemPosition::Last,
                    level: 1,
                    body: "text".into(),
                    transclude_item_id: None,
                    properties: Default::default(),
                },
                BatchOpInput::Update {
                    id: "i".into(),
                    body: None,
                    level: Some(2),
                    collapsed: None,
                    task: Some(TaskStateInput {
                        task: true,
                        ..Default::default()
                    }),
                },
            ],
        };

        let written = serde_json::to_value(&batch).unwrap();

        assert_eq!(
            written,
            json!({ "ops": [
                { "op": "create", "id": "i", "pageId": "p", "position": "last", "level": 1,
                  "body": "text" },
                { "op": "update", "id": "i", "level": 2, "task": { "task": true } },
            ] })
        );
        assert_eq!(
            serde_json::from_value::<BatchInput>(written).unwrap(),
            batch
        );
    }

    // 位置は、JSONでは"first"、"last"、"after:<id>"の文字列として書かれ、同じ位置に読み戻される。
    #[test]
    fn a_position_is_written_in_json_as_its_text_and_read_back() {
        let positions = [
            (ItemPosition::First, "first"),
            (ItemPosition::Last, "last"),
            (ItemPosition::after("x"), "after:x"),
        ];

        for (position, text) in positions {
            let written = serde_json::to_value(&position).unwrap();
            assert_eq!(written, json!(text));
            assert_eq!(
                serde_json::from_value::<ItemPosition>(written).unwrap(),
                position
            );
        }
    }

    // MCPのスキーマでは、位置は、書き方を説明した文字列である。要求のスキーマの中でも、その場で文字列として書かれる。
    #[cfg(feature = "schemars")]
    #[test]
    fn a_position_is_a_text_in_the_schema_of_mcp() {
        let schema = serde_json::to_value(schemars::schema_for!(ItemPosition)).unwrap();
        let request = serde_json::to_value(schemars::schema_for!(MoveToJournalInput)).unwrap();

        assert_eq!(schema["type"], json!("string"));
        assert!(
            schema["description"]
                .as_str()
                .is_some_and(|text| text.contains("'after:<id>'")),
            "{schema}"
        );
        assert_eq!(
            request["properties"]["position"]["type"],
            json!("string"),
            "{request}"
        );
    }

    // プロパティの値は、JSONでは文字列か文字列の配列として書かれ、同じ値に読み戻される。
    #[test]
    fn a_property_value_is_written_in_json_as_a_text_or_a_list_and_read_back() {
        let values = [
            (PropertyValue::Text("x".into()), json!("x")),
            (
                PropertyValue::List(vec!["a".into(), "b".into()]),
                json!(["a", "b"]),
            ),
        ];

        for (value, written) in values {
            assert_eq!(serde_json::to_value(&value).unwrap(), written);
            assert_eq!(
                serde_json::from_value::<PropertyValue>(written).unwrap(),
                value
            );
        }
    }

    // 文字列でも文字列の配列でもないJSONは、プロパティの値として読めない。サーバーはこの2つの形しか返さない。
    #[test]
    fn a_property_value_refuses_json_of_any_other_shape() {
        for json in ["1.5", "true", "null", "[1]", r#"{"a":"b"}"#] {
            assert!(
                serde_json::from_str::<PropertyValue>(json).is_err(),
                "{json}"
            );
        }
    }

    // プロパティの値は、文字列としては文字列の値だけを、リストとしてはリストの値だけを返す。
    #[test]
    fn a_property_value_gives_its_text_or_its_list_by_its_shape() {
        let text = PropertyValue::from("x");
        let list = PropertyValue::from(["a", "b"]);

        assert_eq!((text.as_text(), text.as_list()), (Some("x"), None));
        assert_eq!(
            (list.as_text(), list.as_list()),
            (None, Some(&["a".to_string(), "b".to_string()][..]))
        );
    }

    // MCPのスキーマでは、一括の要求のプロパティの値は、文字列か文字列の配列である。
    #[cfg(feature = "schemars")]
    #[test]
    fn a_property_value_is_a_text_or_a_list_in_the_schema_of_mcp() {
        let request = serde_json::to_value(schemars::schema_for!(BatchInput)).unwrap();

        assert_eq!(
            request["$defs"]["PropertyValue"]["anyOf"],
            json!([
                { "type": "string" },
                { "type": "array", "items": { "type": "string" } }
            ]),
            "{request}"
        );
    }

    // 項目は、サーバーの答えのJSONから読める。サーバーだけの欄は読み飛ばす。
    #[test]
    fn an_item_is_read_from_the_answer_of_the_server_past_the_fields_of_the_server() {
        let answer = json!({
            "id": "i", "pageId": "p", "parentItemId": null, "dirPath": "d/", "name": "n",
            "type": "doc", "createdDate": 1, "modifiedDate": 2, "idx": "a0", "level": 2,
            "body": "text", "ast": "[]", "collapsed": true, "types": ["task"],
            "properties": { "plan": "2026-11-24", "tags": ["x"] },
            "transcludeItemId": "s", "url": "/u"
        });

        let item: Item = serde_json::from_value(answer).unwrap();

        assert_eq!(
            item,
            Item {
                id: "i".into(),
                page_id: "p".into(),
                dir_path: "d/".into(),
                name: "n".into(),
                item_type: "doc".into(),
                created_date: 1,
                modified_date: 2,
                idx: "a0".into(),
                level: 2,
                body: "text".into(),
                collapsed: true,
                types: vec!["task".into()],
                properties: BTreeMap::from([
                    ("plan".into(), PropertyValue::Text("2026-11-24".into())),
                    ("tags".into(), PropertyValue::List(vec!["x".into()])),
                ]),
                transclude_item_id: Some("s".into()),
            }
        );
    }

    // ページは、サーバーの答えのJSONから読める。サーバーだけの欄は読み飛ばす。
    #[test]
    fn a_page_is_read_from_the_answer_of_the_server_past_the_fields_of_the_server() {
        let answer = json!({
            "id": "p", "parentId": "q", "dirPath": "d/", "name": "n", "type": "journal",
            "createdDate": 1, "modifiedDate": 2, "url": "/u",
            "properties": { "plan": "2026-11-24", "tags": ["x"] }
        });

        let page: Page = serde_json::from_value(answer).unwrap();

        assert_eq!(
            page,
            Page {
                id: "p".into(),
                parent_id: "q".into(),
                dir_path: "d/".into(),
                name: "n".into(),
                page_type: "journal".into(),
                created_date: 1,
                modified_date: 2,
                properties: BTreeMap::from([
                    ("plan".into(), PropertyValue::from("2026-11-24")),
                    ("tags".into(), PropertyValue::from(["x"])),
                ]),
            }
        );
    }

    /// An item as the server answers it, with the fields only the server has.
    fn item_answer(id: &str) -> serde_json::Value {
        json!({
            "id": id, "pageId": "p", "parentItemId": null, "dirPath": "", "name": "n",
            "type": "doc", "createdDate": 0, "modifiedDate": 0, "idx": "a0", "level": 1,
            "body": id, "ast": "[]", "collapsed": false, "types": [], "properties": {},
            "transcludeItemId": null, "url": "/u"
        })
    }

    /// The item that `item_answer` reads as.
    fn item(id: &str) -> Item {
        serde_json::from_value(item_answer(id)).unwrap()
    }

    /// A page as the server answers it, with the fields only the server has.
    fn page_answer() -> serde_json::Value {
        json!({
            "id": "p", "parentId": "", "dirPath": "", "name": "n", "type": "doc",
            "createdDate": 0, "modifiedDate": 0, "url": "/u", "properties": {}
        })
    }

    // 項目やページを持つ答えの型は、サーバーの答えのJSONから読める。中の項目とページのサーバーだけの欄は読み飛ばす。
    #[test]
    fn the_answers_with_items_or_pages_are_read_past_the_fields_of_the_server() {
        let page: Page = serde_json::from_value(page_answer()).unwrap();

        let split: Split = serde_json::from_value(json!({
            "item": item_answer("a"), "created": item_answer("b")
        }))
        .unwrap();
        assert_eq!(
            split,
            Split {
                item: item("a"),
                created: item("b"),
            }
        );

        let stashed: Stashed = serde_json::from_value(json!({
            "heading": item_answer("h"), "created": true, "items": [item_answer("a")]
        }))
        .unwrap();
        assert_eq!(
            stashed,
            Stashed {
                heading: item("h"),
                created: true,
                items: vec![item("a")],
            }
        );

        let pasted: Pasted = serde_json::from_value(json!({
            "items": [item_answer("a")], "cursor": { "itemId": "a", "position": 1 }
        }))
        .unwrap();
        assert_eq!(
            pasted,
            Pasted {
                items: vec![item("a")],
                cursor: PasteCursor {
                    item_id: "a".into(),
                    position: 1,
                },
            }
        );

        let batched: Batched = serde_json::from_value(json!({
            "items": [item_answer("a")], "created": [item_answer("b")]
        }))
        .unwrap();
        assert_eq!(
            batched,
            Batched {
                items: vec![item("a")],
                created: vec![item("b")],
            }
        );

        let entry: BacklinkEntry = serde_json::from_value(json!({
            "sourcePage": page_answer(), "sourceItem": item_answer("a")
        }))
        .unwrap();
        assert_eq!(
            entry,
            BacklinkEntry {
                source_page: page.clone(),
                source_item: Some(item("a")),
            }
        );

        let before: JournalBefore = serde_json::from_value(json!({
            "found": true, "page": page_answer(), "items": [item_answer("a")],
            "transcludedTrees": { "s": [item_answer("s")] }
        }))
        .unwrap();
        assert_eq!(
            before,
            JournalBefore {
                found: true,
                page: Some(page),
                items: vec![item("a")],
                transcluded_trees: BTreeMap::from([("s".into(), vec![item("s")])]),
            }
        );
    }

    // ドキュメントの答えとジャーナルへの移動の答えは、サーバーの答えのJSONから読める。
    #[test]
    fn the_answers_of_a_doc_and_of_a_move_to_a_journal_are_read_from_the_server() {
        let page: Page = serde_json::from_value(page_answer()).unwrap();

        let doc: DocPage = serde_json::from_value(json!({
            "page": page_answer(), "items": [item_answer("a")], "created": true
        }))
        .unwrap();
        assert_eq!(
            doc,
            DocPage {
                page: page.clone(),
                items: vec![item("a")],
                created: true,
            }
        );

        let moved: MovedToJournal = serde_json::from_value(json!({
            "page": page_answer(), "created": false, "items": [item_answer("a")]
        }))
        .unwrap();
        assert_eq!(
            moved,
            MovedToJournal {
                page,
                created: false,
                items: vec![item("a")],
            }
        );
    }

    // ページの答えとジャーナルの答えは、サーバーの答えのJSONから読める。
    #[test]
    fn the_answers_of_a_page_and_of_a_journal_are_read_from_the_server() {
        let page: Page = serde_json::from_value(page_answer()).unwrap();
        let trees = BTreeMap::from([("s".to_string(), vec![item("s")])]);

        let loaded: PageWithItems = serde_json::from_value(json!({
            "page": page_answer(), "items": [item_answer("a")],
            "transcludedTrees": { "s": [item_answer("s")] }
        }))
        .unwrap();
        assert_eq!(
            loaded,
            PageWithItems {
                page: page.clone(),
                items: vec![item("a")],
                transcluded_trees: trees.clone(),
            }
        );

        let journal: JournalPage = serde_json::from_value(json!({
            "page": page_answer(), "items": [item_answer("a")], "created": true,
            "transcludedTrees": { "s": [item_answer("s")] }
        }))
        .unwrap();
        assert_eq!(
            journal,
            JournalPage {
                page,
                items: vec![item("a")],
                created: true,
                transcluded_trees: trees,
            }
        );
    }

    // 欄がサーバーと同じ答えの型は、サーバーの答えのJSONに書かれ、同じ値に読み戻される。
    #[test]
    fn the_answers_with_the_fields_of_the_server_are_written_in_its_json_and_read_back() {
        fn round_trip<T>(value: T, written: serde_json::Value)
        where
            T: Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
        {
            assert_eq!(serde_json::to_value(&value).unwrap(), written);
            assert_eq!(serde_json::from_value::<T>(written).unwrap(), value);
        }

        round_trip(
            PropertyDefinition {
                key: "plan".into(),
                value_type: "date".into(),
                system: true,
                page: false,
            },
            json!({ "key": "plan", "valueType": "date", "system": true, "page": false }),
        );
        round_trip(
            TreeExport {
                tree_markdown: "- a".into(),
                flat_markdown: "a".into(),
            },
            json!({ "treeMarkdown": "- a", "flatMarkdown": "a" }),
        );
        round_trip(
            PasteCursor {
                item_id: "i".into(),
                position: 3,
            },
            json!({ "itemId": "i", "position": 3 }),
        );
        round_trip(
            Uploaded {
                file_name: "f.png".into(),
                original_name: "o.png".into(),
            },
            json!({ "fileName": "f.png", "originalName": "o.png" }),
        );
        round_trip(
            FolderChoice {
                selected: true,
                path: "C:/notes".into(),
            },
            json!({ "selected": true, "path": "C:/notes" }),
        );
    }

    // サーバーの設定は、サーバーの答えのJSONから読める。サーバーだけの欄は読み飛ばす。
    #[test]
    fn the_config_is_read_from_the_answer_of_the_server_past_the_fields_of_the_server() {
        let answer = json!({
            "markdownDir": "m", "weekNumbering": "iso", "logicalOutdent": true,
            "tagsDir": "tags/", "homeLink": "home", "wikilinkHref": "/w", "hashtagHref": "/h",
            "pagesBase": "/p", "wikilinkFragmentPattern": "x"
        });

        let config: ServerConfig = serde_json::from_value(answer).unwrap();

        assert_eq!(
            config,
            ServerConfig {
                markdown_dir: "m".into(),
                week_numbering: "iso".into(),
                logical_outdent: true,
                tags_dir: "tags/".into(),
                home_link: "home".into(),
            }
        );
    }

    // 検索の結果は、サーバーと同じ形のJSONに書かれ、同じ値に読み戻される。
    #[test]
    fn a_search_result_is_written_in_the_json_of_the_server_and_read_back() {
        let result = SearchResult {
            page_id: "p".into(),
            page_title: "t".into(),
            item_id: "i".into(),
            snippet: "s".into(),
            score: 1.5,
        };
        let written = json!({
            "pageId": "p", "pageTitle": "t", "itemId": "i", "snippet": "s", "score": 1.5
        });

        assert_eq!(serde_json::to_value(&result).unwrap(), written);
        assert_eq!(
            serde_json::from_value::<SearchResult>(written).unwrap(),
            result
        );
    }

    // 決まりに合わない文字列は、位置として読めない。
    #[test]
    fn a_text_that_is_no_position_is_not_read() {
        for text in ["nowhere", "", "after", "First"] {
            assert!(
                serde_json::from_value::<ItemPosition>(json!(text)).is_err(),
                "{text}"
            );
        }
    }
}
