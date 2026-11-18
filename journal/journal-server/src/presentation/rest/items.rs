use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json, Router};
use serde::Deserialize;

use crate::error::HttpError;
use crate::presentation::client_id::ClientId;
use crate::presentation::rest::At;
use crate::state::{blocking, SharedState};
use journal_api_types::routes;
use journal_core::dto::ItemFields;
use journal_core::journal::{
    BatchAnswer, BatchInput, ChangedBy, CreateItemInput, DeletedItemsAnswer, FlatMarkdownAnswer,
    FlatMarkdownInput, ItemAnswer, ItemIdsInput, ItemsAnswer, MergeItemsInput, MoveItemsInput,
    ParsedItemsAnswer, PasteAnswer, PasteInput, PropertyInput, ShiftLevelsInput, SplitAnswer,
    SplitInput, StashAnswer, StashInput, TranscludedItemsAnswer, TransclusionInput,
    TreeMarkdownAnswer, TreeMarkdownInput, UpdateItemInput,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .at(routes::CREATE_ITEM, create_item)
        .at(routes::ITEMS_BY_TYPES, get_items_by_types)
        .at(routes::MOVE_ITEMS, move_items)
        .at(routes::MERGE_ITEMS, merge_items)
        .at(routes::DELETE_ITEMS, delete_items)
        .at(routes::APPLY_BATCH, apply_batch)
        .at(routes::TRANSCLUDED_ITEMS, transcluded_items)
        .at(routes::SHIFT_ITEM_LEVELS, shift_levels)
        .at(routes::EXPORT_TREE_MARKDOWN, export_tree_markdown)
        .at(routes::EXPORT_PAGE_MARKDOWN, export_page_markdown)
        .at(routes::EXPORT_SUBTREE_MARKDOWN, export_subtree_markdown)
        .at(routes::IMPORT_TREE_MARKDOWN, import_tree_markdown)
        .at(routes::PARSE_FLAT_MARKDOWN, parse_flat_markdown)
        .at(routes::GET_ITEM, get_item)
        .at(routes::UPDATE_ITEM, update_item)
        .at(routes::DELETE_ITEM, delete_item)
        .at(routes::SET_TRANSCLUSION, set_transclusion)
        .at(routes::REMOVE_TRANSCLUSION, remove_transclusion)
        .at(routes::PASTE_TEXT, paste)
        .at(routes::SPLIT_ITEM, split)
        .at(routes::STASH_ITEM, stash)
        .at(routes::CYCLE_TASK, cycle_task)
        .at(routes::UPSERT_ITEM_PROPERTY, upsert_property)
        .at(routes::DELETE_ITEM_PROPERTY, delete_property)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FieldsQuery {
    fields: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct TypesQuery {
    types: Option<String>,
    without_property: Option<String>,
    fields: Option<String>,
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

async fn create_item(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Json(req): Json<CreateItemInput>,
) -> Result<Json<ItemAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state.journal.create_item(req, &ChangedBy::Client(client))?,
        ))
    })
    .await
}

async fn get_item(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<FieldsQuery>,
) -> Result<Response, HttpError> {
    blocking(move || {
        let whole = state.journal.get_item(&id)?;
        Ok(match ItemFields::parse(q.fields.as_deref()) {
            Some(fields) => Json(whole.with_fields(&fields)).into_response(),
            None => Json(whole).into_response(),
        })
    })
    .await
}

async fn update_item(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
    Json(req): Json<UpdateItemInput>,
) -> Result<Json<ItemAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.update_item(
            &id,
            req,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn paste(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
    Json(req): Json<PasteInput>,
) -> Result<Json<PasteAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.paste(
            &id,
            req,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn split(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
    Json(req): Json<SplitInput>,
) -> Result<Json<SplitAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.split(
            &id,
            req,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn stash(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
    Json(req): Json<StashInput>,
) -> Result<Json<StashAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.stash(
            &id,
            req,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn apply_batch(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Json(req): Json<BatchInput>,
) -> Result<Json<BatchAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state.journal.apply_batch(req, &ChangedBy::Client(client))?,
        ))
    })
    .await
}

async fn delete_items(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Json(req): Json<ItemIdsInput>,
) -> Result<Json<DeletedItemsAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state
                .journal
                .delete_items(req, &ChangedBy::Client(client))?,
        ))
    })
    .await
}

async fn merge_items(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Json(req): Json<MergeItemsInput>,
) -> Result<Json<ItemAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state.journal.merge_items(req, &ChangedBy::Client(client))?,
        ))
    })
    .await
}

async fn transcluded_items(
    State(state): State<SharedState>,
    Json(req): Json<ItemIdsInput>,
) -> Result<Json<TranscludedItemsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.transcluded_items(req)?))).await
}

async fn delete_item(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
) -> Result<StatusCode, HttpError> {
    blocking(move || {
        state.journal.delete_item(&id, &ChangedBy::Client(client))?;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

async fn set_transclusion(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
    Json(req): Json<TransclusionInput>,
) -> Result<Json<ItemAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.set_transclusion(
            &id,
            req,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn remove_transclusion(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
) -> Result<Json<ItemAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state
                .journal
                .remove_transclusion(&id, &ChangedBy::Client(client))?,
        ))
    })
    .await
}

async fn get_items_by_types(
    State(state): State<SharedState>,
    Query(q): Query<TypesQuery>,
) -> Result<Response, HttpError> {
    blocking(move || {
        let types: Vec<String> = match q.types.as_deref().filter(|t| !t.is_empty()) {
            Some(t) => t.split(',').map(|s| s.to_string()).collect(),
            None => Vec::new(),
        };
        let whole = state
            .journal
            .items_by_types(&types, q.without_property.as_deref())?;
        Ok(match ItemFields::parse(q.fields.as_deref()) {
            Some(fields) => Json(whole.with_fields(&fields)).into_response(),
            None => Json(whole).into_response(),
        })
    })
    .await
}

async fn cycle_task(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
) -> Result<Json<ItemAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state.journal.cycle_task(&id, &ChangedBy::Client(client))?,
        ))
    })
    .await
}

async fn move_items(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Json(req): Json<MoveItemsInput>,
) -> Result<Json<ItemsAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state.journal.move_items(req, &ChangedBy::Client(client))?,
        ))
    })
    .await
}

async fn shift_levels(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Json(req): Json<ShiftLevelsInput>,
) -> Result<Json<ItemsAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state
                .journal
                .shift_levels(req, &ChangedBy::Client(client))?,
        ))
    })
    .await
}

async fn upsert_property(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path((id, key)): Path<(String, String)>,
    Json(req): Json<PropertyInput>,
) -> Result<Json<ItemAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.set_item_property(
            &id,
            &key,
            req,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn delete_property(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path((id, key)): Path<(String, String)>,
) -> Result<Json<ItemAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.delete_item_property(
            &id,
            &key,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn export_tree_markdown(
    State(state): State<SharedState>,
    Json(req): Json<ItemIdsInput>,
) -> Result<Json<TreeMarkdownAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.export_tree_markdown(req)?))).await
}

async fn export_page_markdown(
    State(state): State<SharedState>,
    Query(q): Query<PageIdQuery>,
) -> Result<Json<TreeMarkdownAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state.journal.export_page_markdown(q.page_id.as_deref())?,
        ))
    })
    .await
}

async fn export_subtree_markdown(
    State(state): State<SharedState>,
    Query(q): Query<ItemIdQuery>,
) -> Result<Json<TreeMarkdownAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state
                .journal
                .export_subtree_markdown(q.item_id.as_deref())?,
        ))
    })
    .await
}

async fn import_tree_markdown(
    State(state): State<SharedState>,
    Json(req): Json<TreeMarkdownInput>,
) -> Result<Json<FlatMarkdownAnswer>, HttpError> {
    Ok(Json(state.journal.import_tree_markdown(req)))
}

async fn parse_flat_markdown(
    State(state): State<SharedState>,
    Json(req): Json<FlatMarkdownInput>,
) -> Result<Json<ParsedItemsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.parse_flat_markdown(req)?))).await
}
