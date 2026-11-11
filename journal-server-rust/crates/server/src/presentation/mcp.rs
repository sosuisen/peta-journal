//! The MCP server: the same 26 tools as the Java server, served over Streamable HTTP at `/mcp`.

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

use crate::error::{AppError, AppResult};
use crate::state::{blocking, SharedState};
use journal_core::dto::{BacklinkEntryDto, ItemDto, ItemFields, PageDto};
use journal_core::infrastructure::export::export_service::format_instant;
use journal_core::journal::{
    ChangedBy, CreateItemInput, ItemIdsInput, MoveItemsInput, PropertyInput, TransclusionInput,
    TreeMarkdownInput, UpdateItemInput,
};

// ---- JSON of the domain records, the way the Java server's Jackson wrote them ----

// These tools have always written a page and an item as such a record: the dates as text, no
// properties. The answers of the journal (ADR 029) carry the same values in the shape of the
// REST API, so a record is written from that shape.

fn page_record_json(p: &PageDto) -> Value {
    json!({
        "id": p.id,
        // The REST shape has an empty text for no parent
        "parentId": Some(&p.parent_id).filter(|id| !id.is_empty()),
        "dirPath": p.dir_path,
        "name": p.name,
        "type": p.page_type,
        "createdDate": format_instant(p.created_date),
        "modifiedDate": format_instant(p.modified_date),
    })
}

fn item_record_json(i: &ItemDto) -> Value {
    json!({
        "id": i.id,
        "pageId": i.page_id,
        "parentItemId": i.parent_item_id,
        "dirPath": i.dir_path,
        "name": i.name,
        "type": i.item_type,
        "createdDate": format_instant(i.created_date),
        "modifiedDate": format_instant(i.modified_date),
        "idx": i.idx,
        "level": i.level,
        "body": i.body,
        "ast": i.ast,
        "collapsed": i.collapsed,
        "types": i.types,
        "transcludeItemId": i.transclude_item_id,
    })
}

fn item_records_json(items: &[ItemDto]) -> Value {
    Value::Array(items.iter().map(item_record_json).collect())
}

/// The answer of an operation of the journal (ADR 029), as the JSON a tool gives back.
fn answer_json(answer: &impl serde::Serialize) -> Value {
    serde_json::to_value(answer).expect("an answer of the journal is plain data")
}

fn backlink_json(e: &BacklinkEntryDto) -> Value {
    json!({
        "sourcePage": page_record_json(&e.source_page),
        "sourceItem": e.source_item.as_ref().map(item_record_json),
    })
}

fn ok(value: Value) -> Result<CallToolResult, McpError> {
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
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    blocking(f).await.map_err(|refusal| mcp_error(refusal.0))
}

/// The IDs of a comma-separated list, as the journal takes them.
fn id_texts(csv: &str) -> Vec<String> {
    csv.split(',').map(|s| s.trim().to_string()).collect()
}

// ---- tool arguments ----

#[derive(Deserialize, JsonSchema)]
pub struct GetItemArgs {
    /// ID of the item
    #[serde(rename = "itemId")]
    pub item_id: String,
    /// Comma-separated field names to return (e.g. 'body,level'). Empty string for all fields.
    #[serde(default)]
    pub fields: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateItemArgs {
    /// ID of the page
    #[serde(rename = "pageId")]
    pub page_id: String,
    /// Position: 'first', 'last', or 'after:<id>'
    pub position: String,
    /// Indentation level (1-based)
    pub level: i32,
    /// Markdown body text
    #[serde(default)]
    pub body: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateItemArgs {
    /// ID of the item
    #[serde(rename = "itemId")]
    pub item_id: String,
    /// New markdown body (null to keep current)
    #[serde(default)]
    pub body: Option<String>,
    /// New indentation level, or -1 to keep current
    #[serde(default = "minus_one")]
    pub level: i32,
    /// New collapsed state: 'true', 'false', or 'null' to keep current
    #[serde(default)]
    pub collapsed: Option<String>,
}

fn minus_one() -> i32 {
    -1
}

#[derive(Deserialize, JsonSchema)]
pub struct ItemIdArgs {
    /// ID of the item
    #[serde(rename = "itemId")]
    pub item_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct MoveItemsArgs {
    /// Comma-separated item IDs to move
    #[serde(rename = "itemIds")]
    pub item_ids: String,
    /// ID of the target page
    #[serde(rename = "targetPageId")]
    pub target_page_id: String,
    /// Position: 'first', 'last', or 'after:<id>'
    pub position: String,
    /// Level delta adjustment, or 0 for no change
    #[serde(rename = "levelDelta", default)]
    pub level_delta: i32,
}

#[derive(Deserialize, JsonSchema)]
pub struct TypesArgs {
    /// Comma-separated types to filter by (e.g. 'task')
    pub types: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ItemIdsArgs {
    /// Comma-separated item IDs to export
    #[serde(rename = "itemIds")]
    pub item_ids: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct SetTransclusionArgs {
    /// ID of the item to set as a transclusion
    #[serde(rename = "itemId")]
    pub item_id: String,
    /// ID of the source item to transclude
    #[serde(rename = "sourceItemId")]
    pub source_item_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpsertPropertyArgs {
    /// ID of the item
    #[serde(rename = "itemId")]
    pub item_id: String,
    /// Property key (e.g. 'begin', 'end', 'due', 'plan')
    pub key: String,
    /// Value type: 'text', 'number', 'checkbox', 'date', 'datetime', or 'bounds' (four numbers 'x,y,width,height')
    #[serde(rename = "valueType")]
    pub value_type: String,
    /// Property value as string
    pub value: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct TreeMarkdownArgs {
    /// Tree markdown string to convert
    #[serde(rename = "treeMarkdown")]
    pub tree_markdown: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct PageIdArgs {
    /// HMTID of the page
    #[serde(rename = "pageId")]
    pub page_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetOrCreateDocArgs {
    /// Title of the document (used as file name)
    pub title: String,
    /// Directory path (e.g. 'projects/'). Empty string for root.
    #[serde(rename = "dirPath", default)]
    pub dir_path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdatePageArgs {
    /// HMTID of the page
    #[serde(rename = "pageId")]
    pub page_id: String,
    /// New title (null to keep current)
    #[serde(default)]
    pub title: Option<String>,
    /// New directory path (null to keep current)
    #[serde(rename = "dirPath", default)]
    pub dir_path: Option<String>,
    /// Delete another page that already has the new name and take its place (default false:
    /// the update fails instead)
    #[serde(default)]
    pub overwrite: bool,
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
pub struct UpdateConfigArgs {
    /// Week numbering system: 'us' or 'iso'
    #[serde(rename = "weekNumbering", default)]
    pub week_numbering: Option<String>,
    /// Markdown export directory path
    #[serde(rename = "markdownDir", default)]
    pub markdown_dir: Option<String>,
    /// Logical outdent: move children with parent
    #[serde(rename = "logicalOutdent", default)]
    pub logical_outdent: Option<bool>,
    /// Directory of the tag pages relative to the pages root, e.g. 'tags/' (empty for the root)
    #[serde(rename = "tagsDir", default)]
    pub tags_dir: Option<String>,
    /// The page the Home button opens, as the target of a wiki link, e.g. 'today' (empty for today's journal)
    #[serde(rename = "homeLink", default)]
    pub home_link: Option<String>,
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

    fn config_json(&self) -> Value {
        json!({
            "markdownDir": self.state.config.markdown_dir(),
            "weekNumbering": self.state.config.week_numbering(),
            "logicalOutdent": self.state.config.logical_outdent(),
            "tagsDir": self.state.config.tags_dir(),
            "homeLink": self.state.config.home_link(),
        })
    }

    // ---- items ----

    #[tool(
        description = "Get a single item by its ID. Returns the item with its properties. Use 'fields' to return only specific fields (comma-separated, e.g. 'body,level,types')."
    )]
    async fn get_item(
        &self,
        Parameters(a): Parameters<GetItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.get_item(&a.item_id)).await?;
        if let Some(fields) = ItemFields::parse(Some(&a.fields)) {
            return ok(Value::Object(fields.of(&r.item)));
        }
        ok(answer_json(&r))
    }

    #[tool(
        description = "Create a new item in a page. Use position to specify where: 'first' for page start, 'last' for page end, 'after:<id>' to insert after a specific item."
    )]
    async fn create_item(
        &self,
        Parameters(a): Parameters<CreateItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let input = CreateItemInput {
            page_id: Some(a.page_id),
            position: Some(a.position),
            level: a.level,
            body: Some(a.body),
            id: None,
        };
        let r = run(move || state.journal.create_item(input, &ChangedBy::Mcp)).await?;
        ok(json!({ "item": item_record_json(&r.item) }))
    }

    #[tool(
        description = "Update an existing item's body, level, or collapsed state. Only non-null fields are updated."
    )]
    async fn update_item(
        &self,
        Parameters(a): Parameters<UpdateItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let input = UpdateItemInput {
            body: a.body,
            level: if a.level < 1 { None } else { Some(a.level) },
            collapsed: match a.collapsed.as_deref() {
                None | Some("null") => None,
                Some(s) => Some(s == "true"),
            },
        };
        let r = run(move || {
            state
                .journal
                .update_item(&a.item_id, input, &ChangedBy::Mcp)
        })
        .await?;
        ok(json!({ "item": item_record_json(&r.item) }))
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
        Parameters(a): Parameters<MoveItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let input = MoveItemsInput {
            item_ids: Some(id_texts(&a.item_ids)),
            target_page_id: Some(a.target_page_id),
            position: Some(a.position),
            level_delta: Some(a.level_delta),
            body: None,
        };
        let r = run(move || state.journal.move_items(input, &ChangedBy::Mcp)).await?;
        ok(json!({ "items": item_records_json(&r.items) }))
    }

    #[tool(description = "Get all items with the specified types (e.g. all 'task' items)")]
    async fn get_items_by_type(
        &self,
        Parameters(a): Parameters<TypesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let types: Vec<String> = a.types.split(',').map(|s| s.to_string()).collect();
        let r = run(move || state.journal.items_by_types(&types, None)).await?;
        ok(json!({ "items": item_records_json(&r.items) }))
    }

    #[tool(
        description = "Export items as hierarchical tree markdown. Returns tree markdown, flat markdown, and HTML representations."
    )]
    async fn export_tree_markdown(
        &self,
        Parameters(a): Parameters<ItemIdsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let input = ItemIdsInput {
            item_ids: Some(id_texts(&a.item_ids)),
        };
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
        Parameters(a): Parameters<ItemIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.cycle_task(&a.item_id, &ChangedBy::Mcp)).await?;
        ok(answer_json(&r))
    }

    #[tool(
        description = "Set a transclusion on an item, making it mirror a source item's content."
    )]
    async fn set_transclusion(
        &self,
        Parameters(a): Parameters<SetTransclusionArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let input = TransclusionInput {
            source_item_id: Some(a.source_item_id),
        };
        let r = run(move || {
            state
                .journal
                .set_transclusion(&a.item_id, input, &ChangedBy::Mcp)
        })
        .await?;
        ok(answer_json(&r))
    }

    #[tool(description = "Remove a transclusion from an item, clearing its body.")]
    async fn remove_transclusion(
        &self,
        Parameters(a): Parameters<ItemIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .remove_transclusion(&a.item_id, &ChangedBy::Mcp)
        })
        .await?;
        ok(answer_json(&r))
    }

    #[tool(
        description = "Upsert a scalar property on an item (e.g. begin, end, due, plan, cancelled)."
    )]
    async fn upsert_property(
        &self,
        Parameters(a): Parameters<UpsertPropertyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let input = PropertyInput {
            value_type: Some(a.value_type),
            value: Some(a.value),
        };
        let r = run(move || {
            state
                .journal
                .set_item_property(&a.item_id, &a.key, input, &ChangedBy::Mcp)
        })
        .await?;
        ok(answer_json(&r))
    }

    #[tool(description = "Convert tree markdown (indented list format) to flat markdown.")]
    async fn import_tree_markdown(
        &self,
        Parameters(a): Parameters<TreeMarkdownArgs>,
    ) -> Result<CallToolResult, McpError> {
        ok(answer_json(&self.state.journal.import_tree_markdown(
            TreeMarkdownInput {
                tree_markdown: Some(a.tree_markdown),
            },
        )))
    }

    // ---- pages ----

    #[tool(description = "Get a page and its items by page ID")]
    async fn get_page_by_id(
        &self,
        Parameters(a): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.get_page(&a.page_id)).await?;
        ok(json!({ "page": page_record_json(&r.page), "items": item_records_json(&r.items) }))
    }

    #[tool(
        description = "Get or create a document page by title. Returns the page and its items. If the page does not exist, it is created with an empty item."
    )]
    async fn get_or_create_doc(
        &self,
        Parameters(a): Parameters<GetOrCreateDocArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .get_or_create_doc(Some(&a.title), Some(&a.dir_path))
        })
        .await?;
        ok(json!({
            "page": page_record_json(&r.page),
            "items": item_records_json(&r.items),
            "created": r.created,
        }))
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
        Parameters(a): Parameters<UpdatePageArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        // An empty title keeps the current one, as no title does
        let title = a.title.filter(|t| !t.is_empty());
        let r = run(move || {
            state.journal.update_page(
                &a.page_id,
                title.as_deref(),
                a.dir_path.as_deref(),
                a.overwrite,
                &ChangedBy::Mcp,
            )
        })
        .await?;
        ok(json!({ "page": page_record_json(&r.page) }))
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
        ok(self.config_json())
    }

    #[tool(description = "Update server configuration")]
    async fn update_config(
        &self,
        Parameters(a): Parameters<UpdateConfigArgs>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(w) = &a.week_numbering {
            self.state.config.set_week_numbering(w);
        }
        if let Some(d) = &a.markdown_dir {
            self.state.config.set_markdown_dir(d);
        }
        if let Some(l) = a.logical_outdent {
            self.state.config.set_logical_outdent(l);
        }
        if let Some(t) = &a.tags_dir {
            self.state.config.set_tags_dir(t);
        }
        if let Some(h) = &a.home_link {
            self.state.config.set_home_link(h);
        }
        ok(self.config_json())
    }

    // ---- navigation ----

    #[tool(
        description = "Get pages and items that link to a given page (backlinks): by a wikilink, or by the tag of a tag page as a hashtag or in a page's tags. Each entry contains the source page and the item containing the link; the item is null when the source page links by its tags."
    )]
    async fn get_backlinks(
        &self,
        Parameters(a): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.backlinks(&a.page_id)).await?;
        ok(json!({ "entries": r.entries.iter().map(backlink_json).collect::<Vec<_>>() }))
    }

    // ---- journals ----

    #[tool(
        description = "Get or create a journal page by file name. Supports daily (YYYY-MM-DD), weekly (YYYY-WNN), monthly (YYYY-MM), and yearly (YYYY) formats."
    )]
    async fn get_or_create_journal(
        &self,
        Parameters(a): Parameters<JournalNameArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.get_or_create_journal(&a.file_name)).await?;
        ok(json!({
            "page": page_record_json(&r.page),
            "items": item_records_json(&r.items),
            "created": r.created,
        }))
    }

    #[tool(description = "Get the most recent daily journal before the specified date")]
    async fn get_most_recent_journal_before(
        &self,
        Parameters(a): Parameters<BeforeDateArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal.most_recent_journal_before(&a.before_date)).await?;
        match &r.page {
            Some(page) => ok(json!({
                "found": true,
                "page": page_record_json(page),
                "items": item_records_json(&r.items),
            })),
            None => ok(json!({ "found": false })),
        }
    }

    #[tool(
        description = "Get recently modified pages, ordered by modification date (newest first)"
    )]
    async fn get_recent_pages(
        &self,
        Parameters(a): Parameters<RecentPagesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || {
            state
                .journal
                .recent_pages(Some(a.max_results), Some(&a.page_type))
        })
        .await?;
        ok(json!({ "pages": r.pages.iter().map(page_record_json).collect::<Vec<_>>() }))
    }
}

#[tool_handler]
impl ServerHandler for JournalMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "PetaJournal: journals, documents and outline items of a local markdown folder."
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
