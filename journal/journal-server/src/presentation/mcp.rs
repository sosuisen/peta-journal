//! The MCP server: the tools of the journal, served over Streamable HTTP at `/mcp`.
//!
//! A tool takes the body of the REST route of its operation as its arguments, and answers with
//! the JSON of that route without the AST of the items (ADR 060, ADR 041). What the route takes in its path or its query is an
//! argument beside the body.

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::AppError;
use crate::presentation::date_format::DateFormat;
use crate::presentation::rest::sync;
use crate::state::{blocking, SharedState};
use journal_api_types::{
    AddPropertyDefinitionInput, AtlasCalloutInput, AtlasGroupInput, AtlasLinkInput,
    AtlasTitleInput, BatchInput, DemotePageInput, FlatMarkdownInput, GetOrCreateDocInput,
    MergeItemsInput, MoveToJournalInput, PasteInput, ShiftLevelsInput, SplitInput, StashInput,
    UpdatePageInput,
};
use journal_core::dto::ItemFields;
use journal_core::journal::{
    ChangedBy, CreateItemInput, ItemIdsInput, MoveItemsInput, PropertyInput, TransclusionInput,
    TreeMarkdownInput, UpdateConfigInput, UpdateItemInput,
};

/// The answer of an operation of the journal (ADR 029), as the JSON a tool gives back.
fn answer_json(answer: &impl serde::Serialize) -> Value {
    serde_json::to_value(answer).expect("an answer of the journal is plain data")
}

/// The field of an item that holds the AST of its body.
const AST_FIELD: &str = "ast";

/// The values of the properties are what the user wrote, so a key of a user with the name of
/// the AST field stays.
const PROPERTIES_FIELD: &str = "properties";

/// Takes the AST out of every item of an answer. A tool gives no AST: an AI reads the body,
/// and the AST would only make the answer long (ADR 041). This is the one difference from the
/// JSON of the REST route (ADR 060).
fn drop_asts(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(drop_asts),
        Value::Object(fields) => {
            fields.remove(AST_FIELD);
            fields
                .iter_mut()
                .filter(|(name, _)| *name != PROPERTIES_FIELD)
                .for_each(|(_, field)| drop_asts(field));
        }
        _ => {}
    }
}

/// The answer of a tool. Every tool answers through here.
fn ok(mut value: Value) -> Result<CallToolResult, McpError> {
    drop_asts(&mut value);
    Ok(CallToolResult::success(vec![Content::text(
        value.to_string(),
    )]))
}

fn mcp_error(e: AppError) -> McpError {
    match e {
        AppError::BadRequest(m) | AppError::NotFound(m) | AppError::Conflict(m) => {
            McpError::invalid_params(m, None)
        }
        AppError::Internal(err) => McpError::internal_error(err.to_string(), None),
    }
}

async fn run<T, F>(f: F) -> Result<T, McpError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    blocking(f).await.map_err(|refusal| mcp_error(refusal.0))
}

// ---- tool arguments ----

/// The arguments of a tool whose answer has pages or items, and with them the format of the
/// dates of the answer. The REST route takes the format as the query `dateFormat`.
#[derive(Deserialize, JsonSchema)]
pub struct Dated<T> {
    #[serde(flatten)]
    pub args: T,
    /// How the answer writes its dates (createdDate, modifiedDate): 'iso' for UTC ISO 8601
    /// text such as '2026-11-11T10:00:00Z'. Left out, or 'millis', for milliseconds since
    /// 1970-01-01 UTC as a number.
    #[serde(rename = "dateFormat", default)]
    pub dates: DateFormat,
}

/// The answer of an operation with its dates in the format the caller asked for.
fn dated(answer: &impl serde::Serialize, dates: DateFormat) -> Result<CallToolResult, McpError> {
    ok(dates.write(answer_json(answer)))
}

#[derive(Deserialize, JsonSchema)]
pub struct GetItemArgs {
    /// ID of the item
    #[serde(rename = "itemId")]
    pub item_id: String,
    /// Comma-separated field names to return (e.g. 'body,level'). Empty string for all fields.
    /// The AST of the body ('ast') is never returned.
    #[serde(default)]
    pub fields: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ItemIdArgs {
    /// ID of the item
    #[serde(rename = "itemId")]
    pub item_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct TypesArgs {
    /// Comma-separated types to filter by (e.g. 'task')
    pub types: String,
    /// Leaves out the items that have a value for this property key (e.g. 'end' for the
    /// tasks that are not completed). Left out for every item.
    #[serde(rename = "withoutProperty", default)]
    pub without_property: Option<String>,
    /// Comma-separated field names to return for each item (e.g. 'id,body'). Left out for
    /// all fields.
    #[serde(default)]
    pub fields: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct PageFieldsArgs {
    /// HMTID of the page
    #[serde(rename = "pageId")]
    pub page_id: String,
    /// Comma-separated field names to return for each item (e.g. 'id,level,body'). Left out
    /// for all fields.
    #[serde(default)]
    pub fields: Option<String>,
}

/// The arguments of a tool that changes one item: the ID of the item, and beside it the fields
/// of the body of the REST route.
#[derive(Deserialize, JsonSchema)]
pub struct OfItem<T> {
    /// ID of the item
    #[serde(rename = "itemId")]
    pub item_id: String,
    #[serde(flatten)]
    pub input: T,
}

/// The same for a tool that changes one page.
#[derive(Deserialize, JsonSchema)]
pub struct OfPage<T> {
    /// HMTID of the page
    #[serde(rename = "pageId")]
    pub page_id: String,
    #[serde(flatten)]
    pub input: T,
}

#[derive(Deserialize, JsonSchema)]
pub struct ItemPropertyArgs {
    /// ID of the item
    #[serde(rename = "itemId")]
    pub item_id: String,
    /// Property key (e.g. 'begin', 'end', 'due', 'plan')
    pub key: String,
    #[serde(flatten)]
    pub property: PropertyInput,
}

#[derive(Deserialize, JsonSchema)]
pub struct PageIdArgs {
    /// HMTID of the page
    #[serde(rename = "pageId")]
    pub page_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct SearchArgs {
    /// Search query string
    pub query: String,
    /// Maximum number of results (default 20)
    #[serde(rename = "maxResults", default)]
    pub max_results: i64,
}

impl SearchArgs {
    /// The number the caller named. Zero or less means that it named none.
    fn max_results(&self) -> Option<usize> {
        usize::try_from(self.max_results)
            .ok()
            .filter(|max| *max > 0)
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct JournalNameArgs {
    /// Journal file name (e.g. '2026-02-18', '2026-W08', '2026-02', '2026')
    #[serde(rename = "fileName")]
    pub file_name: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct BeforeDateArgs {
    /// Date in YYYY-MM-DD format (exclusive upper bound)
    #[serde(rename = "beforeDate")]
    pub before_date: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct RecentPagesArgs {
    /// Maximum number of results (default 10)
    #[serde(rename = "maxResults", default)]
    pub max_results: i64,
    /// Filter by page type: 'journal', 'doc', or empty for all
    #[serde(rename = "type", default)]
    pub page_type: String,
}

/// The arguments of a tool that changes one item of an atlas page: the atlas, the item, and
/// beside them the fields of the body of the REST route.
#[derive(Deserialize, JsonSchema)]
pub struct OfAtlasItem<T> {
    /// HMTID of the atlas page
    #[serde(rename = "pageId")]
    pub page_id: String,
    /// ID of the item of the atlas: a link card, a group or a callout
    #[serde(rename = "itemId")]
    pub item_id: String,
    #[serde(flatten)]
    pub input: T,
}

/// An item of an atlas page with nothing to say about it.
#[derive(Deserialize, JsonSchema, Default)]
pub struct Nothing {}

#[derive(Deserialize, JsonSchema)]
pub struct ItemKeyArgs {
    /// ID of the item
    #[serde(rename = "itemId")]
    pub item_id: String,
    /// Property key
    pub key: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct PagePropertyArgs {
    /// HMTID of the page
    #[serde(rename = "pageId")]
    pub page_id: String,
    /// Property key (e.g. 'tags', 'alias')
    pub key: String,
    #[serde(flatten)]
    pub property: PropertyInput,
}

#[derive(Deserialize, JsonSchema)]
pub struct PageKeyArgs {
    /// HMTID of the page
    #[serde(rename = "pageId")]
    pub page_id: String,
    /// Property key
    pub key: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct KeyArgs {
    /// Property key
    pub key: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct BeforeWeekArgs {
    /// Week in YYYY-Www format, e.g. '2026-W08' (exclusive upper bound)
    #[serde(rename = "beforeWeek")]
    pub before_week: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ToJournalArgs {
    /// Journal file name of a day (e.g. '2026-02-18')
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(flatten)]
    pub input: MoveToJournalInput,
}

#[derive(Deserialize, JsonSchema)]
pub struct TagArgs {
    /// The tag, without '#'
    pub tag: String,
    /// Maximum number of results. Left out for the default.
    #[serde(rename = "maxResults", default)]
    pub max_results: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct FilePathArgs {
    /// Absolute path of a file on the PC the server runs on (e.g. 'C:\Users\me\shot.png').
    /// The server reads the file itself, so its content is not sent. At most 50MB.
    #[serde(rename = "filePath")]
    pub file_path: String,
}

// ---- the server ----

#[derive(Clone)]
pub struct JournalMcp {
    state: SharedState,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl JournalMcp {
    pub fn new(state: SharedState) -> JournalMcp {
        JournalMcp {
            state,
            tool_router: Self::tool_router(),
        }
    }

    // ---- items ----

    #[tool(
        description = "Get a single item by its ID. Returns the item with its properties. Use 'fields' to return only specific fields (comma-separated, e.g. 'body,level,types')."
    )]
    async fn get_item(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<GetItemArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.get_item(&a.item_id)).await?;
        match ItemFields::parse(Some(&a.fields)) {
            Some(fields) => dated(&r.with_fields(&fields), dates),
            None => dated(&r, dates),
        }
    }

    #[tool(
        description = "Create a new item in a page. Use position to specify where: 'first' for page start, 'last' for page end, 'after:<id>' to insert after a specific item. In Flat Markdown, one block is one item: give each paragraph, heading or list an item of its own."
    )]
    async fn create_item(
        &self,
        Parameters(Dated { args: input, dates }): Parameters<Dated<CreateItemInput>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.create_item(input, &ChangedBy::Mcp)).await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Update an existing item's body, level, or collapsed state. A field that is left out stays as it is. In Flat Markdown, one block is one item: a body holds one paragraph, heading or list."
    )]
    async fn update_item(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfItem<UpdateItemInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .update_item(&a.item_id, a.input, &ChangedBy::Mcp)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(description = "Delete an item by its ID")]
    async fn delete_item(
        &self,
        Parameters(a): Parameters<ItemIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        run(move || state.journal.delete_item(&a.item_id, &ChangedBy::Mcp)).await?;
        ok(json!({ "deleted": true }))
    }

    #[tool(
        description = "Move items to a target page and position. Returns items with updated indices."
    )]
    async fn move_items(
        &self,
        Parameters(Dated { args: input, dates }): Parameters<Dated<MoveItemsInput>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.move_items(input, &ChangedBy::Mcp)).await?;
        dated(&r, dates)
    }

    #[tool(description = "Get all items with the specified types (e.g. all 'task' items)")]
    async fn get_items_by_type(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<TypesArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let types: Vec<String> = a.types.split(',').map(|s| s.to_string()).collect();
        let fields = a.fields.clone();
        let r = run(move || {
            state
                .journal
                .items_by_types(&types, a.without_property.as_deref())
        })
        .await?;
        match ItemFields::parse(fields.as_deref()) {
            Some(fields) => dated(&r.with_fields(&fields), dates),
            None => dated(&r, dates),
        }
    }

    #[tool(
        description = "Export items as hierarchical tree markdown. Returns tree markdown, flat markdown, and HTML representations."
    )]
    async fn export_tree_markdown(
        &self,
        Parameters(input): Parameters<ItemIdsInput>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.export_tree_markdown(input)).await?;
        ok(answer_json(&r))
    }

    #[tool(
        description = "Export an item and all its descendants (subtree) as tree markdown, flat markdown, and AST. Use this to get a complete subtree rooted at a specific item."
    )]
    async fn export_subtree_markdown(
        &self,
        Parameters(a): Parameters<ItemIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.export_subtree_markdown(Some(&a.item_id))).await?;
        ok(answer_json(&r))
    }

    #[tool(
        description = "Cycle the task state of an item: non-task → task, task → started (begin), started → completed (end), completed → non-task. An item on a daily journal that becomes a task gets the plan property set to that day."
    )]
    async fn cycle_task(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<ItemIdArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.cycle_task(&a.item_id, &ChangedBy::Mcp)).await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Set a transclusion on an item, making it mirror a source item's content."
    )]
    async fn set_transclusion(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfItem<TransclusionInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .set_transclusion(&a.item_id, a.input, &ChangedBy::Mcp)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(description = "Remove a transclusion from an item, clearing its body.")]
    async fn remove_transclusion(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<ItemIdArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .remove_transclusion(&a.item_id, &ChangedBy::Mcp)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Upsert a scalar property on an item (e.g. begin, end, due, plan, cancelled). The key must be defined: a key with no definition is refused, as its type is chosen only when the user defines it."
    )]
    async fn upsert_property(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<ItemPropertyArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .set_item_property(&a.item_id, &a.key, a.property, &ChangedBy::Mcp)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(description = "Convert tree markdown (indented list format) to flat markdown.")]
    async fn import_tree_markdown(
        &self,
        Parameters(input): Parameters<TreeMarkdownInput>,
    ) -> Result<CallToolResult, McpError> {
        ok(answer_json(&self.state.journal.import_tree_markdown(input)))
    }

    // ---- pages ----

    #[tool(description = "Get a page and its items by page ID")]
    async fn get_page_by_id(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<PageFieldsArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.get_page(&a.page_id)).await?;
        match ItemFields::parse(a.fields.as_deref()) {
            Some(fields) => dated(&r.with_fields(&fields), dates),
            None => dated(&r, dates),
        }
    }

    #[tool(
        description = "Get or create a document page by title. Returns the page and its items. If the page does not exist, it is created with an empty item."
    )]
    async fn get_or_create_doc(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<GetOrCreateDocInput>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .get_or_create_doc(a.title.as_deref(), a.dir_path.as_deref())
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Export a page's items as flat markdown, tree markdown, and AST. Includes item properties (begin, end, tags, etc.) in HTML comments."
    )]
    async fn export_page_markdown(
        &self,
        Parameters(a): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.export_page_markdown(Some(&a.page_id))).await?;
        ok(answer_json(&r))
    }

    #[tool(description = "Update a page's title or directory path")]
    async fn update_page(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfPage<UpdatePageInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state.journal.update_page(
                &a.page_id,
                a.input.title.as_deref(),
                a.input.dir_path.as_deref(),
                a.input.overwrite,
                &ChangedBy::Mcp,
            )
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(description = "Delete a page and all its items")]
    async fn delete_page(
        &self,
        Parameters(a): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        run(move || state.journal.delete_page(&a.page_id, &ChangedBy::Mcp)).await?;
        ok(json!({ "deleted": true }))
    }

    // ---- search ----

    #[tool(
        description = "Search pages by name. Index pages whose folder name matches come first with path prefix matches, then fuzzy/token search, then substring matching as fallback. The atlas pages (index_atlas) are not found by name."
    )]
    async fn search_page_names(
        &self,
        Parameters(a): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let answer =
            run(move || state.journal.search_page_names(&a.query, a.max_results())).await?;
        ok(answer_json(&answer))
    }

    #[tool(
        description = "Combined search supporting #tag, key=value properties, in:path page prefix, created:/modified: date filters, plain text, and OR/AND operators. Examples: '#report in:DailyReport/2026-03', 'status=done', 'created:2026-03', '#urgent OR #important'"
    )]
    async fn search_combined(
        &self,
        Parameters(a): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let answer = run(move || state.journal.search_combined(&a.query, a.max_results())).await?;
        ok(answer_json(&answer))
    }

    // ---- config ----

    #[tool(
        description = "Get server configuration (markdownDir, weekNumbering, logicalOutdent, tagsDir)"
    )]
    async fn get_config(&self) -> Result<CallToolResult, McpError> {
        ok(answer_json(&self.state.journal.config()))
    }

    #[tool(description = "Update server configuration")]
    async fn update_config(
        &self,
        Parameters(input): Parameters<UpdateConfigInput>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || Ok(state.journal.update_config(input))).await?;
        ok(answer_json(&r))
    }

    // ---- navigation ----

    #[tool(
        description = "Get pages and items that link to a given page (backlinks): by a wikilink, or by the tag of a tag page as a hashtag or in a page's tags. Each entry contains the source page and the item containing the link; the item is null when the source page links by its tags."
    )]
    async fn get_backlinks(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<PageIdArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.backlinks(&a.page_id)).await?;
        dated(&r, dates)
    }

    // ---- journals ----

    #[tool(
        description = "Get or create a journal page by file name. Supports daily (YYYY-MM-DD), weekly (YYYY-WNN), monthly (YYYY-MM), and yearly (YYYY) formats."
    )]
    async fn get_or_create_journal(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<JournalNameArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.get_or_create_journal(&a.file_name)).await?;
        dated(&r, dates)
    }

    #[tool(description = "Get the most recent daily journal before the specified date")]
    async fn get_most_recent_journal_before(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<BeforeDateArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.most_recent_journal_before(&a.before_date)).await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Get recently modified pages, ordered by modification date (newest first)"
    )]
    async fn get_recent_pages(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<RecentPagesArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .recent_pages(Some(a.max_results), Some(&a.page_type))
        })
        .await?;
        dated(&r, dates)
    }

    // ---- more of the items (the operations of an editor) ----

    #[tool(
        description = "Paste text into an item at a cursor. Text of several items (Flat Markdown or a Markdown list) becomes several items. Returns the items written and where the cursor is afterwards."
    )]
    async fn paste_text(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfItem<PasteInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.paste(&a.item_id, a.input, &ChangedBy::Mcp)).await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Split an item in two at a cursor, as Enter in an editor does. The text before the cursor stays, the rest goes to a new item below."
    )]
    async fn split_item(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfItem<SplitInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.split(&a.item_id, a.input, &ChangedBy::Mcp)).await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Merge an item into another, as Backspace at the start of an item does. The text of the item is added to the end of the other, and the item goes away."
    )]
    async fn merge_items(
        &self,
        Parameters(Dated { args: input, dates }): Parameters<Dated<MergeItemsInput>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.merge_items(input, &ChangedBy::Mcp)).await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Move an item with its children to the stash page. Returns the heading it went under and the items that moved."
    )]
    async fn stash_item(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfItem<StashInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.stash(&a.item_id, a.input, &ChangedBy::Mcp)).await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Apply several operations on items (create, update, move, delete) in one transaction: all of them are done, or none."
    )]
    async fn apply_batch(
        &self,
        Parameters(Dated { args: input, dates }): Parameters<Dated<BatchInput>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.apply_batch(input, &ChangedBy::Mcp)).await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Delete several items at once. A page left without an item gets an empty one, which is returned."
    )]
    async fn delete_items(
        &self,
        Parameters(Dated { args: input, dates }): Parameters<Dated<ItemIdsInput>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.delete_items(input, &ChangedBy::Mcp)).await?;
        dated(&r, dates)
    }

    #[tool(description = "Indent or outdent several items by adding a number to their levels.")]
    async fn shift_item_levels(
        &self,
        Parameters(Dated { args: input, dates }): Parameters<Dated<ShiftLevelsInput>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.shift_levels(input, &ChangedBy::Mcp)).await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Of the given items, get the IDs of those that are transclusions or that a transclusion shows."
    )]
    async fn get_transcluded_items(
        &self,
        Parameters(input): Parameters<ItemIdsInput>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.transcluded_items(input)).await?;
        ok(answer_json(&r))
    }

    #[tool(description = "Delete a property from an item.")]
    async fn delete_property(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<ItemKeyArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .delete_item_property(&a.item_id, &a.key, &ChangedBy::Mcp)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Parse Flat Markdown into its items: the level, the ID, the properties and the body of each. Nothing is saved."
    )]
    async fn parse_flat_markdown(
        &self,
        Parameters(input): Parameters<FlatMarkdownInput>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.parse_flat_markdown(input)).await?;
        ok(answer_json(&r))
    }

    // ---- more of the pages ----

    #[tool(description = "Get the pages directly under a page.")]
    async fn get_child_pages(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<PageIdArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.child_pages(&a.page_id)).await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Make the index page of a folder a plain page of the folder above, under the name of the folder."
    )]
    async fn demote_page(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfPage<DemotePageInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .demote_page(&a.page_id, a.input.overwrite, &ChangedBy::Mcp)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Upsert a property on a page (e.g. tags). The key must be defined as a key of pages."
    )]
    async fn upsert_page_property(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<PagePropertyArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state.journal.set_page_property(
                &a.page_id,
                &a.key,
                a.property.value_type.as_deref(),
                a.property.value.as_deref(),
                &ChangedBy::Mcp,
            )
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(description = "Delete a property from a page.")]
    async fn delete_page_property(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<PageKeyArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .delete_page_property(&a.page_id, &a.key, &ChangedBy::Mcp)
        })
        .await?;
        dated(&r, dates)
    }

    // ---- atlas pages ----

    #[tool(
        description = "Add a link card to an atlas page (the index_atlas page of a folder): a card that shows a page or a URL. Returns the items of the atlas."
    )]
    async fn add_atlas_link(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfPage<AtlasLinkInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.add_atlas_link(&a.page_id, a.input)).await?;
        dated(&r, dates)
    }

    #[tool(description = "Change what a link card of an atlas page shows.")]
    async fn set_atlas_link(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfAtlasItem<AtlasLinkInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .set_atlas_link(&a.page_id, &a.item_id, a.input)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Put cards of an atlas page into one group, together with a target card or into a target group."
    )]
    async fn group_atlas_cards(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfPage<AtlasGroupInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.group_atlas_cards(&a.page_id, a.input)).await?;
        dated(&r, dates)
    }

    #[tool(description = "Take cards of an atlas page out of their groups.")]
    async fn ungroup_atlas_cards(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfPage<AtlasGroupInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.ungroup_atlas_cards(&a.page_id, a.input)).await?;
        dated(&r, dates)
    }

    #[tool(description = "Set the title of a group of an atlas page and the side it is on.")]
    async fn set_atlas_group_title(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfAtlasItem<AtlasTitleInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .set_atlas_group_title(&a.page_id, &a.item_id, a.input)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(description = "Add a callout (a bubble of text) to an atlas page.")]
    async fn add_atlas_callout(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfPage<AtlasCalloutInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.add_atlas_callout(&a.page_id, a.input)).await?;
        dated(&r, dates)
    }

    #[tool(description = "Change the text or the side of a callout of an atlas page.")]
    async fn set_atlas_callout(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfAtlasItem<AtlasCalloutInput>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .set_atlas_callout(&a.page_id, &a.item_id, a.input)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Remove a link card, a group or a callout from an atlas page. The card of a page cannot be removed: it goes with its page."
    )]
    async fn remove_atlas_item(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<OfAtlasItem<Nothing>>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.remove_atlas_item(&a.page_id, &a.item_id)).await?;
        dated(&r, dates)
    }

    // ---- more of the journals and the search ----

    #[tool(description = "Get the most recent weekly journal before the specified week")]
    async fn get_most_recent_weekly_journal_before(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<BeforeWeekArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .most_recent_weekly_journal_before(&a.before_week)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Move items to the journal of a day, which is made when it does not exist. Returns the journal and the items as they are now."
    )]
    async fn move_items_to_journal(
        &self,
        Parameters(Dated { args: a, dates }): Parameters<Dated<ToJournalArgs>>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .move_items_to_journal(&a.file_name, a.input, &ChangedBy::Mcp)
        })
        .await?;
        dated(&r, dates)
    }

    #[tool(
        description = "Find the items and the pages that have a tag: as a hashtag in a body, or in the tags of a page."
    )]
    async fn search_by_tag(
        &self,
        Parameters(a): Parameters<TagArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.search_by_tag(&a.tag, a.max_results)).await?;
        ok(answer_json(&r))
    }

    // ---- property definitions ----

    #[tool(
        description = "Get the property definitions: the keys that items and pages may have, and the type of each."
    )]
    async fn get_property_definitions(&self) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.property_definitions()).await?;
        ok(answer_json(&r))
    }

    #[tool(
        description = "Define a property key. The name of the key gives its type (e.g. a name ending in '_date' is a date); a type sent with it is only checked."
    )]
    async fn add_property_definition(
        &self,
        Parameters(input): Parameters<AddPropertyDefinitionInput>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.add_property_definition(input)).await?;
        ok(answer_json(&r))
    }

    #[tool(
        description = "Delete the definition of a property key, with every value of the key on items and pages."
    )]
    async fn delete_property_definition(
        &self,
        Parameters(a): Parameters<KeyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .delete_property_definition(&a.key, &ChangedBy::Mcp)
        })
        .await?;
        ok(answer_json(&r))
    }

    // ---- assets ----

    #[tool(
        description = "Copy a file of this PC (an image, for example) into the assets folder of the journal. Returns the name it is saved under (fileName) and its original name. Link it from an item as '![](../assets/<fileName>)'."
    )]
    async fn upload_asset(
        &self,
        Parameters(a): Parameters<FilePathArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.save_asset_file(&a.file_path)).await?;
        ok(answer_json(&r))
    }

    // ---- the markdown folder ----

    #[tool(
        description = "Get the state of the sync with the markdown folder: the folder, the number of files tracked and of files that went missing."
    )]
    async fn get_sync_status(&self) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || sync::status_of(&state)).await?;
        ok(answer_json(&r))
    }

    #[tool(
        description = "Scan the markdown folder now and take in the files changed outside. Waits for the scan. Returns the numbers of files imported, in conflict, newly missing and exported; all zero when a scan is running already."
    )]
    async fn scan_folder(&self) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || Ok(sync::scan_now(&state))).await?;
        ok(answer_json(&r))
    }
}

#[tool_handler]
impl ServerHandler for JournalMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "PetaJournal: journals, documents and outline items of a local markdown folder. \
                 The pages are written in Flat Markdown, where one block is one item."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// The Streamable HTTP service to mount at `/mcp`.
pub fn service(state: SharedState) -> StreamableHttpService<JournalMcp, LocalSessionManager> {
    StreamableHttpService::new(
        move || Ok(JournalMcp::new(Arc::clone(&state))),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // 項目のASTは、答えのどの深さの項目からも外される。利用者のプロパティは、同じ名前でも残る。
    #[test]
    fn the_ast_is_dropped_from_every_item_and_a_property_of_that_name_stays() {
        let mut answer = json!({
            "item": { "body": "a", "ast": "{}", "properties": { "ast": "mine" } },
            "transcludedTrees": { "t": [{ "body": "b", "ast": "{}" }] },
        });

        drop_asts(&mut answer);

        assert_eq!(
            answer,
            json!({
                "item": { "body": "a", "properties": { "ast": "mine" } },
                "transcludedTrees": { "t": [{ "body": "b" }] },
            })
        );
    }
}
