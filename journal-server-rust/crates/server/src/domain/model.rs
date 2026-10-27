//! Domain records. Dates are Unix epoch milliseconds, as stored in SQLite.

use hmtid::Hmtid;

/// A page: a journal (`type = "journal"`) or a document (`type = "doc"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub id: Hmtid,
    pub parent_id: Option<Hmtid>,
    pub dir_path: String,
    pub name: String,
    pub page_type: String,
    pub created_date: i64,
    pub modified_date: i64,
}

impl Page {
    /// The full path of the page, `dir_path + name`, used as its title.
    pub fn title(&self) -> String {
        format!("{}{}", self.dir_path, self.name)
    }

    pub fn is_index(&self) -> bool {
        self.name == "index"
    }

    pub fn is_atlas(&self) -> bool {
        self.page_type == "doc" && self.name == ATLAS_NAME
    }
}

/// The reserved file name of the atlas page of a directory. The atlas page is opened from
/// its index page, so it is left out of the page-name search.
pub const ATLAS_NAME: &str = "index_atlas";

/// True when a page title, `dir_path + name`, is the title of an atlas page.
pub fn is_atlas_title(title: &str) -> bool {
    title == ATLAS_NAME || title.ends_with(&format!("/{ATLAS_NAME}"))
}

/// One outline item of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: Hmtid,
    pub page_id: Hmtid,
    /// Parent item ID (None for top-level items).
    pub parent_item_id: Option<Hmtid>,
    pub dir_path: String,
    pub name: String,
    /// "journal" or "doc" (from the parent page).
    pub item_type: String,
    pub created_date: i64,
    pub modified_date: i64,
    /// Fractional index string for ordering.
    pub idx: String,
    /// Indentation level (1-based, 1 = top level).
    pub level: i32,
    /// Markdown source text.
    pub body: String,
    /// Parsed AST JSON.
    pub ast: String,
    pub collapsed: bool,
    /// Item types (e.g. "heading", "code", "task").
    pub types: Vec<String>,
    /// Source item ID for transclusion (None if not a transclusion).
    pub transclude_item_id: Option<Hmtid>,
}

impl Item {
    pub fn is_task(&self) -> bool {
        self.types.iter().any(|t| t == ItemPatch::TASK_TYPE)
    }
}

/// A partial update of one item. Only the fields that are set are written.
#[derive(Debug, Clone)]
pub struct ItemPatch {
    /// New markdown body, or None to keep the current body.
    pub body: Option<String>,
    /// Parsed AST of the new body; required when body is set.
    pub ast: Option<String>,
    /// Types found in the new body (heading, code, blockquote); required when body is set.
    /// They replace every current type except `task`.
    pub body_types: Option<Vec<String>>,
    /// Some(true) to add the `task` type, Some(false) to remove it, None to keep it.
    pub task: Option<bool>,
    /// New indentation level, or None to keep the current level.
    pub level: Option<i32>,
    /// New collapsed state, or None to keep the current state.
    pub collapsed: Option<bool>,
    /// New modified date of the item.
    pub modified_date: i64,
}

impl ItemPatch {
    pub const TASK_TYPE: &'static str = "task";

    /// Builds a patch, checking that a body comes with its AST and its types.
    pub fn new(
        body: Option<String>,
        ast: Option<String>,
        body_types: Option<Vec<String>>,
        task: Option<bool>,
        level: Option<i32>,
        collapsed: Option<bool>,
        modified_date: i64,
    ) -> Result<ItemPatch, String> {
        if body.is_some() {
            if ast.is_none() {
                return Err("ast must not be null when body is set".to_string());
            }
            if body_types.is_none() {
                return Err("bodyTypes must not be null when body is set".to_string());
            }
        }
        if let Some(l) = level {
            if l < 1 {
                return Err(format!("level must be 1 or more: {l}"));
            }
        }
        Ok(ItemPatch {
            body,
            ast,
            body_types,
            task,
            level,
            collapsed,
            modified_date,
        })
    }

    pub fn changes_body(&self) -> bool {
        self.body.is_some()
    }
}

/// The new place of one item in a move.
#[derive(Debug, Clone)]
pub struct ItemPlacement {
    pub item_id: Hmtid,
    pub idx: String,
}

/// Where a new or moved item goes on a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemPosition {
    First,
    Last,
    After(Hmtid),
}

impl ItemPosition {
    /// Parses `first`, `last` or `after:<id>`.
    pub fn parse(s: &str) -> Result<ItemPosition, String> {
        if s.is_empty() {
            return Err("position must not be null or empty".to_string());
        }
        match s {
            "first" => Ok(ItemPosition::First),
            "last" => Ok(ItemPosition::Last),
            _ => {
                if let Some(rest) = s.strip_prefix("after:") {
                    Hmtid::parse(rest)
                        .map(ItemPosition::After)
                        .map_err(|e| e.to_string())
                } else {
                    Err(format!("Unknown position: {s}"))
                }
            }
        }
    }
}

/// A page and the item in it that links to another page.
#[derive(Debug, Clone)]
pub struct BacklinkEntry {
    pub source_page: Page,
    pub source_item: Option<Item>,
}

/// One hit of a page or full-text search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub page_id: Hmtid,
    pub page_title: String,
    /// Empty for page hits.
    pub item_id: String,
    pub snippet: String,
    pub score: f32,
}

impl SearchResult {
    /// A result for a page, with the page's full path as title and snippet.
    pub fn from_page(page: &Page) -> SearchResult {
        let title = page.title();
        SearchResult {
            page_id: page.id.clone(),
            page_title: title.clone(),
            item_id: String::new(),
            snippet: title,
            score: 1.0,
        }
    }
}
