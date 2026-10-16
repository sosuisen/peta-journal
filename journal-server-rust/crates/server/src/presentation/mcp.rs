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

use crate::domain::model::{BacklinkEntry, Item, ItemPosition, Page, SearchResult};
use crate::error::{parse_id, AppError, AppResult};
use crate::infrastructure::export::export_service::format_instant;
use crate::presentation::dto::{select_fields, ItemDto};
use crate::state::{blocking, SharedState};

// ---- JSON of the domain records, the way the Java server's Jackson wrote them ----

fn page_json(p: &Page) -> Value {
    json!({
        "id": p.id.to_string(),
        "parentId": p.parent_id.as_ref().map(|x| x.to_string()),
        "dirPath": p.dir_path,
        "name": p.name,
        "type": p.page_type,
        "createdDate": format_instant(p.created_date),
        "modifiedDate": format_instant(p.modified_date),
    })
}

fn item_json(i: &Item) -> Value {
    json!({
        "id": i.id.to_string(),
        "pageId": i.page_id.to_string(),
        "parentItemId": i.parent_item_id.as_ref().map(|x| x.to_string()),
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
        "transcludeItemId": i.transclude_item_id.as_ref().map(|x| x.to_string()),
    })
}

fn items_json(items: &[Item]) -> Value {
    Value::Array(items.iter().map(item_json).collect())
}

fn search_result_json(r: &SearchResult) -> Value {
    json!({
        "pageId": r.page_id.to_string(),
        "pageTitle": r.page_title,
        "itemId": r.item_id,
        "snippet": r.snippet,
        "score": r.score,
    })
}

fn backlink_json(e: &BacklinkEntry) -> Value {
    json!({
        "sourcePage": page_json(&e.source_page),
        "sourceItem": e.source_item.as_ref().map(item_json),
    })
}

fn item_dto_json(dto: &ItemDto) -> Value {
    serde_json::to_value(dto).unwrap_or(Value::Null)
}

fn ok(value: Value) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(value.to_string())]))
}

fn mcp_error(e: AppError) -> McpError {
    match e {
        AppError::BadRequest(m) | AppError::NotFound(m) => McpError::invalid_params(m, None),
        AppError::Internal(err) => McpError::internal_error(err.to_string(), None),
    }
}

async fn run<T, F>(f: F) -> Result<T, McpError>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    blocking(f).await.map_err(mcp_error)
}

fn id(s: &str) -> Result<hmtid::Hmtid, McpError> {
    parse_id(s).map_err(mcp_error)
}

fn ids(csv: &str) -> Result<Vec<hmtid::Hmtid>, McpError> {
    csv.split(',').map(|s| id(s.trim())).collect()
}

fn position(s: &str) -> Result<ItemPosition, McpError> {
    ItemPosition::parse(s).map_err(|m| McpError::invalid_params(m, None))
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
    /// Value type: 'text', 'number', 'checkbox', 'date', or 'datetime'
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
}

#[derive(Deserialize, JsonSchema)]
pub struct SearchArgs {
    /// Search query string
    pub query: String,
    /// Maximum number of results (default 20)
    #[serde(rename = "maxResults", default)]
    pub max_results: i64,
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
        })
    }

    fn export_json(r: crate::application::item_service::TreeMarkdownExport) -> Value {
        json!({
            "treeMarkdown": r.tree_markdown,
            "ast": r.ast,
            "flatMarkdown": r.flat_markdown,
        })
    }

    // ---- items ----

    #[tool(description = "Get a single item by its ID. Returns the item with its properties. Use 'fields' to return only specific fields (comma-separated, e.g. 'body,level,types').")]
    async fn get_item(&self, Parameters(a): Parameters<GetItemArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let item_id = id(&a.item_id)?;
        let dto = run(move || state.dtos.to_dto(&state.item_service.get_item_by_id(&item_id)?)).await?;
        if !a.fields.is_empty() {
            return ok(Value::Object(select_fields(&dto, &a.fields)));
        }
        ok(json!({ "item": item_dto_json(&dto) }))
    }

    #[tool(description = "Create a new item in a page. Use position to specify where: 'first' for page start, 'last' for page end, 'after:<id>' to insert after a specific item.")]
    async fn create_item(&self, Parameters(a): Parameters<CreateItemArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let page_id = id(&a.page_id)?;
        let pos = position(&a.position)?;
        let item = run(move || {
            state
                .item_service
                .create_item(&page_id, &pos, a.level, Some(&a.body), None)
        })
        .await?;
        ok(json!({ "item": item_json(&item) }))
    }

    #[tool(description = "Update an existing item's body, level, or collapsed state. Only non-null fields are updated.")]
    async fn update_item(&self, Parameters(a): Parameters<UpdateItemArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let item_id = id(&a.item_id)?;
        let level = if a.level < 1 { None } else { Some(a.level) };
        let collapsed = match a.collapsed.as_deref() {
            None | Some("null") => None,
            Some(s) => Some(s == "true"),
        };
        let item = run(move || {
            state
                .item_service
                .update_item(&item_id, a.body.as_deref(), level, collapsed)
        })
        .await?;
        ok(json!({ "item": item_json(&item) }))
    }

    #[tool(description = "Delete an item by its ID")]
    async fn delete_item(&self, Parameters(a): Parameters<ItemIdArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let item_id = id(&a.item_id)?;
        run(move || state.item_service.delete_item(&item_id)).await?;
        ok(json!({ "deleted": true }))
    }

    #[tool(description = "Move items to a target page and position. Returns items with updated indices.")]
    async fn move_items(&self, Parameters(a): Parameters<MoveItemsArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let item_ids = ids(&a.item_ids)?;
        let target = id(&a.target_page_id)?;
        let pos = position(&a.position)?;
        let delta = if a.level_delta == 0 { None } else { Some(a.level_delta) };
        let items = run(move || state.item_service.move_items(&item_ids, &target, &pos, delta)).await?;
        ok(json!({ "items": items_json(&items) }))
    }

    #[tool(description = "Get all items with the specified types (e.g. all 'task' items)")]
    async fn get_items_by_type(&self, Parameters(a): Parameters<TypesArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let types: Vec<String> = a.types.split(',').map(|s| s.to_string()).collect();
        let items = run(move || state.item_service.get_items_by_types(&types)).await?;
        ok(json!({ "items": items_json(&items) }))
    }

    #[tool(description = "Export items as hierarchical tree markdown. Returns tree markdown, flat markdown, and HTML representations.")]
    async fn export_tree_markdown(&self, Parameters(a): Parameters<ItemIdsArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let item_ids = ids(&a.item_ids)?;
        let r = run(move || state.item_service.export_tree_markdown(&item_ids)).await?;
        ok(Self::export_json(r))
    }

    #[tool(description = "Export an item and all its descendants (subtree) as tree markdown, flat markdown, and AST. Use this to get a complete subtree rooted at a specific item.")]
    async fn export_subtree_markdown(&self, Parameters(a): Parameters<ItemIdArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let item_id = id(&a.item_id)?;
        let r = run(move || state.item_service.export_subtree_markdown(&item_id)).await?;
        ok(Self::export_json(r))
    }

    #[tool(description = "Cycle the task state of an item: non-task → task, task → started (begin), started → completed (end), completed → non-task.")]
    async fn cycle_task(&self, Parameters(a): Parameters<ItemIdArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let item_id = id(&a.item_id)?;
        let dto = run(move || state.dtos.to_dto(&state.item_service.cycle_task(&item_id)?)).await?;
        ok(json!({ "item": item_dto_json(&dto) }))
    }

    #[tool(description = "Set a transclusion on an item, making it mirror a source item's content.")]
    async fn set_transclusion(&self, Parameters(a): Parameters<SetTransclusionArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let item_id = id(&a.item_id)?;
        let source = id(&a.source_item_id)?;
        let dto = run(move || state.dtos.to_dto(&state.item_service.set_transclusion(&item_id, &source)?)).await?;
        ok(json!({ "item": item_dto_json(&dto) }))
    }

    #[tool(description = "Remove a transclusion from an item, clearing its body.")]
    async fn remove_transclusion(&self, Parameters(a): Parameters<ItemIdArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let item_id = id(&a.item_id)?;
        let dto = run(move || state.dtos.to_dto(&state.item_service.remove_transclusion(&item_id)?)).await?;
        ok(json!({ "item": item_dto_json(&dto) }))
    }

    #[tool(description = "Upsert a scalar property on an item (e.g. begin, end, due, plan, cancelled).")]
    async fn upsert_property(&self, Parameters(a): Parameters<UpsertPropertyArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let item_id = id(&a.item_id)?;
        let dto = run(move || {
            let item = state.item_service.get_item_by_id(&item_id)?;
            state
                .properties
                .upsert_scalar("item", item_id.as_str(), &a.key, &a.value_type, &a.value)?;
            state.dtos.to_dto(&item)
        })
        .await?;
        ok(json!({ "item": item_dto_json(&dto) }))
    }

    #[tool(description = "Convert tree markdown (indented list format) to flat markdown.")]
    async fn import_tree_markdown(&self, Parameters(a): Parameters<TreeMarkdownArgs>) -> Result<CallToolResult, McpError> {
        let flat = self.state.item_service.import_tree_markdown(&a.tree_markdown);
        ok(json!({ "flatMarkdown": flat }))
    }

    // ---- pages ----

    #[tool(description = "Get a page and its items by page ID")]
    async fn get_page_by_id(&self, Parameters(a): Parameters<PageIdArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let page_id = id(&a.page_id)?;
        let (page, items) = run(move || {
            let page = state
                .page_service
                .get_page(&page_id)?
                .ok_or_else(|| AppError::bad_request(format!("Page not found: {page_id}")))?;
            let items = state.page_service.get_page_items(&page.id)?;
            Ok((page, items))
        })
        .await?;
        ok(json!({ "page": page_json(&page), "items": items_json(&items) }))
    }

    #[tool(description = "Get or create a document page by title. Returns the page and its items. If the page does not exist, it is created with an empty item.")]
    async fn get_or_create_doc(&self, Parameters(a): Parameters<GetOrCreateDocArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.page_service.get_or_create_doc(Some(&a.title), Some(&a.dir_path))).await?;
        ok(json!({ "page": page_json(&r.page), "items": items_json(&r.items), "created": r.created }))
    }

    #[tool(description = "Export a page's items as flat markdown, tree markdown, and AST. Includes item properties (begin, end, tags, etc.) in HTML comments.")]
    async fn export_page_markdown(&self, Parameters(a): Parameters<PageIdArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let page_id = id(&a.page_id)?;
        let r = run(move || state.item_service.export_page_markdown(&page_id)).await?;
        ok(Self::export_json(r))
    }

    #[tool(description = "Update a page's title or directory path")]
    async fn update_page(&self, Parameters(a): Parameters<UpdatePageArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let page_id = id(&a.page_id)?;
        let title = a.title.filter(|t| !t.is_empty());
        let page = run(move || {
            state
                .page_service
                .update_page(&page_id, title.as_deref(), a.dir_path.as_deref())
        })
        .await?;
        ok(json!({ "page": page_json(&page) }))
    }

    #[tool(description = "Delete a page and all its items")]
    async fn delete_page(&self, Parameters(a): Parameters<PageIdArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let page_id = id(&a.page_id)?;
        run(move || state.page_service.delete_page(&page_id)).await?;
        ok(json!({ "deleted": true }))
    }

    // ---- search ----

    #[tool(description = "Search pages by name. Uses prefix matching first, then fuzzy/token search, then substring matching as fallback.")]
    async fn search_page_names(&self, Parameters(a): Parameters<SearchArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let max = if a.max_results > 0 { a.max_results } else { 20 };
        let results = run(move || {
            let prefix = state.pages.find_by_path_prefix(&a.query, max)?;
            if !prefix.is_empty() {
                return Ok(prefix.iter().map(SearchResult::from_page).collect::<Vec<_>>());
            }
            let indexed = state.search.search_page_names(&a.query, max as usize);
            if !indexed.is_empty() {
                return Ok(indexed);
            }
            Ok(state
                .pages
                .find_by_path_contains(&a.query, max)?
                .iter()
                .map(SearchResult::from_page)
                .collect())
        })
        .await?;
        ok(json!({ "results": results.iter().map(search_result_json).collect::<Vec<_>>() }))
    }

    #[tool(description = "Combined search supporting #tag, key=value properties, in:path page prefix, created:/modified: date filters, plain text, and OR/AND operators. Examples: '#report in:DailyReport/2026-03', 'status=done', 'created:2026-03', '#urgent OR #important'")]
    async fn search_combined(&self, Parameters(a): Parameters<SearchArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let max = if a.max_results > 0 { a.max_results as usize } else { 20 };
        let results = run(move || state.combined_search.search(&a.query, max)).await?;
        ok(json!({ "results": results.iter().map(search_result_json).collect::<Vec<_>>() }))
    }

    // ---- config ----

    #[tool(description = "Get server configuration (markdownDir, weekNumbering, logicalOutdent)")]
    async fn get_config(&self) -> Result<CallToolResult, McpError> {
        ok(self.config_json())
    }

    #[tool(description = "Update server configuration")]
    async fn update_config(&self, Parameters(a): Parameters<UpdateConfigArgs>) -> Result<CallToolResult, McpError> {
        if let Some(w) = &a.week_numbering {
            self.state.config.set_week_numbering(w);
        }
        if let Some(d) = &a.markdown_dir {
            self.state.config.set_markdown_dir(d);
        }
        if let Some(l) = a.logical_outdent {
            self.state.config.set_logical_outdent(l);
        }
        ok(self.config_json())
    }

    // ---- navigation ----

    #[tool(description = "Get pages and items that link to a given page (backlinks). Each entry contains the source page and the item containing the link.")]
    async fn get_backlinks(&self, Parameters(a): Parameters<PageIdArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let page_id = id(&a.page_id)?;
        let entries = run(move || state.page_service.get_backlinks(&page_id)).await?;
        ok(json!({ "entries": entries.iter().map(backlink_json).collect::<Vec<_>>() }))
    }

    // ---- journals ----

    #[tool(description = "Get or create a journal page by file name. Supports daily (YYYY-MM-DD), weekly (YYYY-WNN), monthly (YYYY-MM), and yearly (YYYY) formats.")]
    async fn get_or_create_journal(&self, Parameters(a): Parameters<JournalNameArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let r = run(move || state.journal_service.get_or_create_journal_by_name(&a.file_name)).await?;
        ok(json!({ "page": page_json(&r.page), "items": items_json(&r.items), "created": r.created }))
    }

    #[tool(description = "Get the most recent daily journal before the specified date")]
    async fn get_most_recent_journal_before(&self, Parameters(a): Parameters<BeforeDateArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let before = chrono::NaiveDate::parse_from_str(&a.before_date, "%Y-%m-%d")
            .map_err(|e| McpError::invalid_params(format!("Invalid date '{}': {e}", a.before_date), None))?;
        let r = run(move || state.journal_service.get_most_recent_journal_before(before)).await?;
        match r.page {
            Some(page) => ok(json!({ "found": true, "page": page_json(&page), "items": items_json(&r.items) })),
            None => ok(json!({ "found": false })),
        }
    }

    #[tool(description = "Get recently modified pages, ordered by modification date (newest first)")]
    async fn get_recent_pages(&self, Parameters(a): Parameters<RecentPagesArgs>) -> Result<CallToolResult, McpError> {
        let state = Arc::clone(&self.state);
        let max = if a.max_results > 0 { a.max_results } else { 10 };
        let page_type = if a.page_type.is_empty() { None } else { Some(a.page_type) };
        let pages = run(move || state.page_service.get_recent_pages(max, page_type.as_deref())).await?;
        ok(json!({ "pages": pages.iter().map(page_json).collect::<Vec<_>>() }))
    }
}

#[tool_handler]
impl ServerHandler for JournalMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("PetaJournal: journals, documents and outline items of a local markdown folder.".into()),
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
