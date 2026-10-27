use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{parse_id, AppError, AppResult};
use crate::domain::model::Page;
use crate::presentation::dto::{backlink_dto, page_dto_with, BacklinkEntryDto, ItemDto, PageDto};
use crate::state::{blocking, SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/pages/doc", post(get_or_create_doc))
        .route("/api/pages/recent", get(get_recent))
        .route("/api/pages/{id}", get(get_page).put(update_page).delete(delete_page))
        .route("/api/pages/{id}/backlinks", get(get_backlinks))
        .route("/api/pages/{id}/children", get(get_children))
        .route(
            "/api/pages/{id}/properties/{key}",
            axum::routing::put(upsert_property).delete(delete_property),
        )
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct UpsertPropertyRequest {
    value_type: Option<String>,
    value: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetPageByIdResponse {
    page: PageDto,
    items: Vec<ItemDto>,
    transcluded_trees: BTreeMap<String, Vec<ItemDto>>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct GetOrCreateDocRequest {
    title: Option<String>,
    dir_path: Option<String>,
}

#[derive(Serialize)]
struct GetOrCreateDocResponse {
    page: PageDto,
    items: Vec<ItemDto>,
    created: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct UpdatePageRequest {
    title: Option<String>,
    dir_path: Option<String>,
}

#[derive(Serialize)]
struct UpdatePageResponse {
    page: PageDto,
}

#[derive(Serialize)]
struct PagesResponse {
    pages: Vec<PageDto>,
}

#[derive(Serialize)]
struct GetBacklinksResponse {
    entries: Vec<BacklinkEntryDto>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct RecentQuery {
    max_results: Option<i64>,
    #[serde(rename = "type")]
    page_type: Option<String>,
}

async fn get_page(State(state): State<SharedState>, Path(id): Path<String>) -> AppResult<Json<GetPageByIdResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let page = state
            .page_service
            .get_page(&page_id)?
            .ok_or_else(|| AppError::not_found(format!("Page not found: {id}")))?;
        let items = state.page_service.get_page_items(&page_id)?;
        Ok(Json(GetPageByIdResponse {
            page: state.dtos.page_to_dto(&page)?,
            items: state.dtos.to_dtos(&items)?,
            transcluded_trees: state.dtos.transcluded_trees(&items)?,
        }))
    })
    .await
}

async fn get_or_create_doc(
    State(state): State<SharedState>,
    Json(req): Json<GetOrCreateDocRequest>,
) -> AppResult<Json<GetOrCreateDocResponse>> {
    blocking(move || {
        let result = state
            .page_service
            .get_or_create_doc(req.title.as_deref(), req.dir_path.as_deref())?;
        Ok(Json(GetOrCreateDocResponse {
            page: state.dtos.page_to_dto(&result.page)?,
            items: state.dtos.to_dtos(&result.items)?,
            created: result.created,
        }))
    })
    .await
}

async fn update_page(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePageRequest>,
) -> AppResult<Json<UpdatePageResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let (page, changed) = state
            .item_service
            .update_page(&page_id, req.title.as_deref(), req.dir_path.as_deref())?;
        state.sse.broadcast_pages_updated(&id);
        for page_id in changed {
            state.sse.broadcast_pages_updated(page_id.as_str());
        }
        Ok(Json(UpdatePageResponse {
            page: state.dtos.page_to_dto(&page)?,
        }))
    })
    .await
}

async fn upsert_property(
    State(state): State<SharedState>,
    Path((id, key)): Path<(String, String)>,
    Json(req): Json<UpsertPropertyRequest>,
) -> AppResult<Json<UpdatePageResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let page = state.page_service.set_property(
            &page_id,
            &key,
            req.value_type.as_deref().unwrap_or("text"),
            req.value.as_deref().unwrap_or(""),
        )?;
        state.sse.broadcast_pages_updated(&id);
        Ok(Json(UpdatePageResponse {
            page: state.dtos.page_to_dto(&page)?,
        }))
    })
    .await
}

async fn delete_property(
    State(state): State<SharedState>,
    Path((id, key)): Path<(String, String)>,
) -> AppResult<Json<UpdatePageResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let page = state.page_service.delete_property(&page_id, &key)?;
        state.sse.broadcast_pages_updated(&id);
        Ok(Json(UpdatePageResponse {
            page: state.dtos.page_to_dto(&page)?,
        }))
    })
    .await
}

async fn delete_page(State(state): State<SharedState>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        state.page_service.delete_page(&page_id)?;
        state.sse.broadcast_pages_updated(&id);
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

async fn get_recent(State(state): State<SharedState>, Query(q): Query<RecentQuery>) -> AppResult<Json<PagesResponse>> {
    blocking(move || {
        let page_type = q.page_type.filter(|t| !t.is_empty());
        let pages = state
            .page_service
            .get_recent_pages(q.max_results.unwrap_or(10), page_type.as_deref())?;
        Ok(Json(pages_with_scalars(&state, &pages)?))
    })
    .await
}

async fn get_backlinks(State(state): State<SharedState>, Path(id): Path<String>) -> AppResult<Json<GetBacklinksResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let entries = state.page_service.get_backlinks(&page_id)?;
        Ok(Json(GetBacklinksResponse {
            entries: entries.iter().map(backlink_dto).collect(),
        }))
    })
    .await
}

async fn get_children(State(state): State<SharedState>, Path(id): Path<String>) -> AppResult<Json<PagesResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let pages = state.pages.find_by_parent_id(&page_id)?;
        let mut response = pages_with_scalars(&state, &pages)?;
        // The atlas draws an icon from the tags of each card
        let ids: Vec<String> = pages.iter().map(|p| p.id.to_string()).collect();
        let lists = state.properties.find_list_by_targets("page", &ids)?;
        for dto in &mut response.pages {
            if let Some(tags) = lists.get(&dto.id).and_then(|l| l.get("tags")) {
                dto.properties.insert("tags".into(), json!(tags));
            }
        }
        Ok(Json(response))
    })
    .await
}

/// The sidebar sorts pages by `published` and marks the published ones, so the page lists
/// carry the scalar properties. The list properties stay out, but for the tags of children.
fn pages_with_scalars(state: &SharedState, pages: &[Page]) -> AppResult<PagesResponse> {
    let ids: Vec<String> = pages.iter().map(|p| p.id.to_string()).collect();
    let scalars = state.properties.find_scalar_by_targets("page", &ids)?;
    Ok(PagesResponse {
        pages: pages
            .iter()
            .map(|p| page_dto_with(p, scalars.get(&p.id.to_string()), None))
            .collect(),
    })
}
