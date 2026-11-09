use std::collections::{BTreeMap, BTreeSet};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hmtid::Hmtid;
use serde::{Deserialize, Serialize};

use crate::application::item_service::{BatchOp, TaskState};
use crate::application::paste::PasteMode;
use crate::domain::model::{Item, ItemPosition};
use crate::error::{parse_id, AppError, AppResult};
use crate::presentation::dto::{select_fields, ItemDto};
use crate::state::{blocking, SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/items", post(create_item).get(get_items_by_types))
        .route("/api/items/move", post(move_items))
        .route("/api/items/merge", post(merge_items))
        .route("/api/items/delete", post(delete_items))
        .route("/api/items/batch", post(apply_batch))
        .route("/api/items/transcluded", post(transcluded_items))
        .route("/api/items/shift-level", post(shift_levels))
        .route("/api/items/export", post(export_tree_markdown))
        .route("/api/items/export-page", post(export_page_markdown))
        .route("/api/items/export-subtree", post(export_subtree_markdown))
        .route("/api/items/import", post(import_tree_markdown))
        .route("/api/items/parse", post(parse_flat_markdown))
        .route(
            "/api/items/{id}",
            get(get_item).put(update_item).delete(delete_item),
        )
        .route(
            "/api/items/{id}/transclude",
            post(set_transclusion).delete(remove_transclusion),
        )
        .route("/api/items/{id}/paste", post(paste))
        .route("/api/items/{id}/cycle-task", post(cycle_task))
        .route("/api/items/{id}/task", axum::routing::put(set_task))
        .route(
            "/api/items/{id}/properties/{key}",
            axum::routing::put(upsert_property).delete(delete_property),
        )
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct CreateItemRequest {
    page_id: Option<String>,
    position: Option<String>,
    level: i32,
    body: Option<String>,
    id: Option<String>,
}

#[derive(Serialize)]
struct ItemResponse {
    item: ItemDto,
}

#[derive(Serialize)]
struct ItemsResponse {
    items: Vec<ItemDto>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct UpdateItemRequest {
    body: Option<String>,
    level: Option<i32>,
    collapsed: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct MoveItemsRequest {
    item_ids: Option<Vec<String>>,
    target_page_id: Option<String>,
    position: Option<String>,
    level_delta: Option<i32>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct DeleteItemsRequest {
    item_ids: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct BatchRequest {
    ops: Vec<BatchOpRequest>,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
enum BatchOpRequest {
    #[serde(rename_all = "camelCase")]
    Create {
        id: String,
        page_id: String,
        position: String,
        level: i32,
        #[serde(default)]
        body: String,
    },
    Update {
        id: String,
        body: Option<String>,
        level: Option<i32>,
        collapsed: Option<bool>,
        task: Option<SetTaskRequest>,
    },
    Delete {
        id: String,
    },
}

impl BatchOpRequest {
    fn parse(self) -> AppResult<BatchOp> {
        Ok(match self {
            BatchOpRequest::Create {
                id,
                page_id,
                position,
                level,
                body,
            } => BatchOp::Create {
                id: parse_id(&id)?,
                page_id: parse_id(&page_id)?,
                position: parse_position(Some(&position))?,
                level,
                body,
            },
            BatchOpRequest::Update {
                id,
                body,
                level,
                collapsed,
                task,
            } => BatchOp::Update {
                id: parse_id(&id)?,
                body,
                level,
                collapsed,
                task: task.map(|task| TaskState {
                    task: task.task,
                    begin: task.begin,
                    end: task.end,
                    plan: task.plan,
                }),
            },
            BatchOpRequest::Delete { id } => BatchOp::Delete { id: parse_id(&id)? },
        })
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct MergeItemsRequest {
    item_id: Option<String>,
    into_id: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct PasteRequest {
    body: Option<String>,
    cursor: usize,
    text: Option<String>,
    mode: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CursorDto {
    item_id: String,
    position: usize,
}

#[derive(Serialize)]
struct PasteResponse {
    items: Vec<ItemDto>,
    cursor: CursorDto,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ShiftLevelsRequest {
    item_ids: Option<Vec<String>>,
    delta: i32,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ExportTreeMarkdownRequest {
    item_ids: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct TranscludedItemsRequest {
    item_ids: Option<Vec<String>>,
}

/// The ids, among those asked about, of the items some other item transcludes
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TranscludedItemsResponse {
    item_ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportTreeMarkdownResponse {
    tree_markdown: String,
    ast: String,
    flat_markdown: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ImportTreeMarkdownRequest {
    tree_markdown: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportTreeMarkdownResponse {
    flat_markdown: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ParseFlatMarkdownRequest {
    markdown: Option<String>,
}

#[derive(Serialize)]
struct ParsedFlatItem {
    level: i32,
    id: Option<String>,
    properties: BTreeMap<String, String>,
    body: String,
}

#[derive(Serialize)]
struct ParseFlatMarkdownResponse {
    items: Vec<ParsedFlatItem>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct UpsertPropertyRequest {
    value_type: Option<String>,
    value: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct SetTaskRequest {
    task: bool,
    begin: Option<String>,
    end: Option<String>,
    plan: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct SetTransclusionRequest {
    source_item_id: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FieldsQuery {
    fields: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TypesQuery {
    types: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct PageIdQuery {
    page_id: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ItemIdQuery {
    item_id: Option<String>,
}

fn required<'a>(value: Option<&'a str>, name: &str) -> AppResult<&'a str> {
    value.ok_or_else(|| AppError::bad_request(format!("{name} is required")))
}

fn parse_ids(ids: Option<Vec<String>>, name: &str) -> AppResult<Vec<Hmtid>> {
    let ids = ids.ok_or_else(|| AppError::bad_request(format!("{name} is required")))?;
    ids.iter().map(|s| parse_id(s)).collect()
}

fn parse_position(position: Option<&str>) -> AppResult<ItemPosition> {
    ItemPosition::parse(position.unwrap_or("")).map_err(AppError::bad_request)
}

/// An item as the API answers with it, with its properties.
fn item_response(state: &SharedState, item: &Item) -> AppResult<Json<ItemResponse>> {
    Ok(Json(ItemResponse {
        item: state.dtos.to_dto(item)?,
    }))
}

fn items_response(state: &SharedState, items: &[Item]) -> AppResult<Json<ItemsResponse>> {
    Ok(Json(ItemsResponse {
        items: state.dtos.to_dtos(items)?,
    }))
}

fn broadcast_to_transcluders(
    state: &SharedState,
    source_item_id: &Hmtid,
    exclude_page_id: &str,
) -> AppResult<()> {
    for t in state.items.find_transcluders(source_item_id)? {
        let page_id = t.page_id.to_string();
        if page_id != exclude_page_id {
            state.sse.broadcast_pages_updated(&page_id);
        }
    }
    Ok(())
}

async fn create_item(
    State(state): State<SharedState>,
    Json(req): Json<CreateItemRequest>,
) -> AppResult<Json<ItemResponse>> {
    let page_id = parse_id(required(req.page_id.as_deref(), "pageId")?)?;
    let client_id = match req.id.as_deref() {
        Some(s) if !s.is_empty() => Some(parse_id(s)?),
        _ => None,
    };
    let position = parse_position(req.position.as_deref())?;
    blocking(move || {
        let item = state.item_service.create_item(
            &page_id,
            &position,
            req.level,
            req.body.as_deref().unwrap_or_default(),
            client_id,
        )?;
        state.sse.broadcast_pages_updated(page_id.as_str());
        item_response(&state, &item)
    })
    .await
}

async fn get_item(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<FieldsQuery>,
) -> AppResult<Response> {
    let item_id = parse_id(&id)?;
    blocking(move || {
        let dto = state
            .dtos
            .to_dto(&state.item_service.get_item_by_id(&item_id)?)?;
        match q.fields.as_deref().filter(|f| !f.is_empty()) {
            Some(fields) => Ok(Json(select_fields(&dto, fields)).into_response()),
            None => Ok(Json(ItemResponse { item: dto }).into_response()),
        }
    })
    .await
}

async fn update_item(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateItemRequest>,
) -> AppResult<Json<ItemResponse>> {
    let item_id = parse_id(&id)?;
    blocking(move || {
        let item = state.item_service.update_item(
            &item_id,
            req.body.as_deref(),
            req.level,
            req.collapsed,
        )?;
        let page_id = item.page_id.to_string();
        state.sse.broadcast_pages_updated(&page_id);
        broadcast_to_transcluders(&state, &item_id, &page_id)?;
        item_response(&state, &item)
    })
    .await
}

async fn paste(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<PasteRequest>,
) -> AppResult<Json<PasteResponse>> {
    let item_id = parse_id(&id)?;
    let mode = match req.mode.as_deref() {
        None | Some("auto") => PasteMode::Auto,
        Some("flat") => PasteMode::Flat,
        Some("raw") => PasteMode::Raw,
        Some(other) => {
            return Err(AppError::bad_request(format!(
                "mode must be auto, flat or raw, but was {other}"
            )))
        }
    };
    blocking(move || {
        let pasted = state.item_service.paste(
            &item_id,
            req.body.as_deref().unwrap_or_default(),
            req.cursor,
            req.text.as_deref().unwrap_or_default(),
            mode,
        )?;
        // The first item is the one written to: the source item, when a transclusion was
        // pasted into
        let written = &pasted.items[0];
        let page_id = written.page_id.to_string();
        state.sse.broadcast_pages_updated(&page_id);
        broadcast_to_transcluders(&state, &written.id, &page_id)?;
        Ok(Json(PasteResponse {
            items: state.dtos.to_dtos(&pasted.items)?,
            cursor: CursorDto {
                item_id: pasted.cursor_item_id.to_string(),
                position: pasted.cursor,
            },
        }))
    })
    .await
}

async fn apply_batch(
    State(state): State<SharedState>,
    Json(req): Json<BatchRequest>,
) -> AppResult<Json<ItemsResponse>> {
    let ops = req
        .ops
        .into_iter()
        .map(BatchOpRequest::parse)
        .collect::<AppResult<Vec<_>>>()?;
    blocking(move || {
        let applied = state.item_service.apply_batch(&ops)?;
        for page_id in &applied.page_ids {
            state.sse.broadcast_pages_updated(page_id.as_str());
        }
        items_response(&state, &applied.items)
    })
    .await
}

async fn delete_items(
    State(state): State<SharedState>,
    Json(req): Json<DeleteItemsRequest>,
) -> AppResult<StatusCode> {
    let item_ids = parse_ids(req.item_ids, "itemIds")?;
    blocking(move || delete_and_tell(&state, &item_ids)).await
}

/// Deletes items and tells the pages they were on and the pages that transcluded them.
fn delete_and_tell(state: &SharedState, item_ids: &[Hmtid]) -> AppResult<StatusCode> {
    // The pages to tell are found first: after the deletion the items are gone
    let mut pages = BTreeSet::new();
    for id in item_ids {
        pages.extend(state.item_service.get_item_page_id(id)?);
        for t in state.items.find_transcluders(id)? {
            pages.insert(t.page_id.to_string());
        }
    }
    state.item_service.delete_items(item_ids)?;
    for page_id in pages {
        state.sse.broadcast_pages_updated(&page_id);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn merge_items(
    State(state): State<SharedState>,
    Json(req): Json<MergeItemsRequest>,
) -> AppResult<Json<ItemResponse>> {
    let item_id = parse_id(req.item_id.as_deref().unwrap_or(""))?;
    let into_id = parse_id(req.into_id.as_deref().unwrap_or(""))?;
    blocking(move || {
        // The merged item goes away: its transcluders are found first so their pages can be told
        let transcluders = state.items.find_transcluders(&item_id)?;
        let item_page_id = state.item_service.get_item_page_id(&item_id)?;
        let into = state.item_service.merge_item(&item_id, &into_id)?;
        let page_id = into.page_id.to_string();
        state.sse.broadcast_pages_updated(&page_id);
        if let Some(item_page_id) = item_page_id {
            state.sse.broadcast_pages_updated(&item_page_id);
        }
        broadcast_to_transcluders(&state, &into_id, &page_id)?;
        for t in transcluders {
            state.sse.broadcast_pages_updated(t.page_id.as_str());
        }
        item_response(&state, &into)
    })
    .await
}

/// Which of the items are transcluded by other items, as a client asks before it deletes them.
async fn transcluded_items(
    State(state): State<SharedState>,
    Json(req): Json<TranscludedItemsRequest>,
) -> AppResult<Json<TranscludedItemsResponse>> {
    let ids = parse_ids(req.item_ids, "itemIds")?;
    blocking(move || {
        let mut item_ids = Vec::new();
        for id in ids {
            if !state.items.find_transcluders(&id)?.is_empty() {
                item_ids.push(id.to_string());
            }
        }
        Ok(Json(TranscludedItemsResponse { item_ids }))
    })
    .await
}

async fn delete_item(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let item_id = parse_id(&id)?;
    blocking(move || delete_and_tell(&state, &[item_id])).await
}

async fn set_transclusion(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<SetTransclusionRequest>,
) -> AppResult<Json<ItemResponse>> {
    let item_id = parse_id(&id)?;
    let source_id = parse_id(required(req.source_item_id.as_deref(), "sourceItemId")?)?;
    blocking(move || {
        let item = state.item_service.set_transclusion(&item_id, &source_id)?;
        state.sse.broadcast_pages_updated(item.page_id.as_str());
        item_response(&state, &item)
    })
    .await
}

async fn remove_transclusion(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> AppResult<Json<ItemResponse>> {
    let item_id = parse_id(&id)?;
    blocking(move || {
        let item = state.item_service.remove_transclusion(&item_id)?;
        state.sse.broadcast_pages_updated(item.page_id.as_str());
        item_response(&state, &item)
    })
    .await
}

async fn get_items_by_types(
    State(state): State<SharedState>,
    Query(q): Query<TypesQuery>,
) -> AppResult<Json<ItemsResponse>> {
    blocking(move || {
        let types: Vec<String> = match q.types.as_deref().filter(|t| !t.is_empty()) {
            Some(t) => t.split(',').map(|s| s.to_string()).collect(),
            None => Vec::new(),
        };
        let items = state.item_service.get_items_by_types(&types)?;
        items_response(&state, &items)
    })
    .await
}

async fn cycle_task(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> AppResult<Json<ItemResponse>> {
    let item_id = parse_id(&id)?;
    blocking(move || {
        let item = state.item_service.cycle_task(&item_id)?;
        state.sse.broadcast_pages_updated(item.page_id.as_str());
        item_response(&state, &item)
    })
    .await
}

async fn set_task(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<SetTaskRequest>,
) -> AppResult<Json<ItemResponse>> {
    let item_id = parse_id(&id)?;
    let task = TaskState {
        task: req.task,
        begin: req.begin,
        end: req.end,
        plan: req.plan,
    };
    blocking(move || {
        let item = state.item_service.set_task(&item_id, &task)?;
        state.sse.broadcast_pages_updated(item.page_id.as_str());
        item_response(&state, &item)
    })
    .await
}

async fn move_items(
    State(state): State<SharedState>,
    Json(req): Json<MoveItemsRequest>,
) -> AppResult<Json<ItemsResponse>> {
    let position = parse_position(req.position.as_deref())?;
    let ids = parse_ids(req.item_ids, "itemIds")?;
    let target = parse_id(required(req.target_page_id.as_deref(), "targetPageId")?)?;
    blocking(move || {
        let items = state
            .item_service
            .move_items(&ids, &target, &position, req.level_delta)?;
        state.sse.broadcast_pages_updated(target.as_str());
        items_response(&state, &items)
    })
    .await
}

/// Indents or outdents a group of items in one request. The change is relative, so requests that
/// arrive out of order still add up correctly.
async fn shift_levels(
    State(state): State<SharedState>,
    Json(req): Json<ShiftLevelsRequest>,
) -> AppResult<Json<ItemsResponse>> {
    let ids = parse_ids(req.item_ids, "itemIds")?;
    blocking(move || {
        let items = state.item_service.shift_levels(&ids, req.delta)?;
        let mut seen: Vec<String> = Vec::new();
        for item in &items {
            let page_id = item.page_id.to_string();
            if !seen.contains(&page_id) {
                state.sse.broadcast_pages_updated(&page_id);
                seen.push(page_id);
            }
        }
        for item in &items {
            broadcast_to_transcluders(&state, &item.id, item.page_id.as_str())?;
        }
        items_response(&state, &items)
    })
    .await
}

async fn upsert_property(
    State(state): State<SharedState>,
    Path((id, key)): Path<(String, String)>,
    Json(req): Json<UpsertPropertyRequest>,
) -> AppResult<Json<ItemResponse>> {
    let item_id = parse_id(&id)?;
    blocking(move || {
        let item = state.item_service.set_property(
            &item_id,
            &key,
            req.value_type.as_deref().unwrap_or("text"),
            req.value.as_deref().unwrap_or(""),
        )?;
        state.sse.broadcast_pages_updated(item.page_id.as_str());
        // The DTO is built after the upsert, so it carries the new property
        item_response(&state, &item)
    })
    .await
}

async fn delete_property(
    State(state): State<SharedState>,
    Path((id, key)): Path<(String, String)>,
) -> AppResult<Json<ItemResponse>> {
    let item_id = parse_id(&id)?;
    blocking(move || {
        let item = state.item_service.delete_property(&item_id, &key)?;
        state.sse.broadcast_pages_updated(item.page_id.as_str());
        // The DTO is built after the delete, so the property is already gone from it
        item_response(&state, &item)
    })
    .await
}

fn export_response(
    r: crate::application::item_service::TreeMarkdownExport,
) -> Json<ExportTreeMarkdownResponse> {
    Json(ExportTreeMarkdownResponse {
        tree_markdown: r.tree_markdown,
        ast: r.ast,
        flat_markdown: r.flat_markdown,
    })
}

async fn export_tree_markdown(
    State(state): State<SharedState>,
    Json(req): Json<ExportTreeMarkdownRequest>,
) -> AppResult<Json<ExportTreeMarkdownResponse>> {
    let ids = parse_ids(req.item_ids, "itemIds")?;
    blocking(move || {
        Ok(export_response(
            state.item_service.export_tree_markdown(&ids)?,
        ))
    })
    .await
}

async fn export_page_markdown(
    State(state): State<SharedState>,
    Query(q): Query<PageIdQuery>,
) -> AppResult<Json<ExportTreeMarkdownResponse>> {
    let page_id = parse_id(required(q.page_id.as_deref(), "pageId")?)?;
    blocking(move || {
        Ok(export_response(
            state.item_service.export_page_markdown(&page_id)?,
        ))
    })
    .await
}

async fn export_subtree_markdown(
    State(state): State<SharedState>,
    Query(q): Query<ItemIdQuery>,
) -> AppResult<Json<ExportTreeMarkdownResponse>> {
    let item_id = parse_id(required(q.item_id.as_deref(), "itemId")?)?;
    blocking(move || {
        Ok(export_response(
            state.item_service.export_subtree_markdown(&item_id)?,
        ))
    })
    .await
}

async fn import_tree_markdown(
    State(state): State<SharedState>,
    Json(req): Json<ImportTreeMarkdownRequest>,
) -> AppResult<Json<ImportTreeMarkdownResponse>> {
    Ok(Json(ImportTreeMarkdownResponse {
        flat_markdown: state
            .item_service
            .import_tree_markdown(req.tree_markdown.as_deref().unwrap_or("")),
    }))
}

/// Splits Flat Markdown into items by the rules of the spec. Nothing is stored.
/// Bodies keep their GFM task prefix, so they can be passed to the create endpoint as-is.
async fn parse_flat_markdown(
    State(state): State<SharedState>,
    Json(req): Json<ParseFlatMarkdownRequest>,
) -> AppResult<Json<ParseFlatMarkdownResponse>> {
    blocking(move || {
        let page = state
            .splitter
            .split(req.markdown.as_deref().unwrap_or(""))?;
        Ok(Json(ParseFlatMarkdownResponse {
            items: page
                .items
                .into_iter()
                .map(|i| ParsedFlatItem {
                    level: i.level,
                    id: i.id.clone(),
                    properties: i.properties_map(),
                    body: i.body,
                })
                .collect(),
        }))
    })
    .await
}
