use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::model::Page;
use crate::error::{parse_id, AppError, AppResult};
use crate::infrastructure::launcher;
use crate::presentation::client_id::ClientId;
use crate::presentation::dto::{
    backlink_dto, page_dto_with, BacklinkEntryDto, ItemDto, ItemFields, PageDto,
};
use crate::state::{blocking, SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/pages/doc", post(get_or_create_doc))
        .route("/api/pages/recent", get(get_recent))
        .route(
            "/api/pages/{id}",
            get(get_page).put(update_page).delete(delete_page),
        )
        .route("/api/pages/{id}/backlinks", get(get_backlinks))
        .route("/api/pages/{id}/demote", post(demote_page))
        .route("/api/pages/{id}/children", get(get_children))
        .route("/api/pages/{id}/open-folder", post(open_export_folder))
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
struct GetPageByIdResponse<I> {
    page: PageDto,
    items: Vec<I>,
    transcluded_trees: BTreeMap<String, Vec<I>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FieldsQuery {
    fields: Option<String>,
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
    /// Deletes another page that has the new name, instead of refusing the update.
    overwrite: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct DemotePageRequest {
    /// Deletes another page that has the new name, instead of refusing the demotion.
    overwrite: bool,
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

async fn get_page(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Query(q): Query<FieldsQuery>,
) -> AppResult<Response> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let page = state
            .page_service
            .get_page(&page_id)?
            .ok_or_else(|| AppError::not_found(format!("Page not found: {id}")))?;
        let items = state.page_service.get_page_items(&page_id)?;
        let whole = GetPageByIdResponse {
            page: state.dtos.page_to_dto(&page)?,
            items: state.dtos.to_dtos(&items)?,
            transcluded_trees: state.dtos.transcluded_trees(&items)?,
        };
        let response = match ItemFields::parse(q.fields.as_deref()) {
            Some(fields) => Json(GetPageByIdResponse {
                items: fields.of_each(&whole.items),
                transcluded_trees: whole
                    .transcluded_trees
                    .iter()
                    .map(|(source, tree)| (source.clone(), fields.of_each(tree)))
                    .collect(),
                page: whole.page,
            })
            .into_response(),
            None => Json(whole).into_response(),
        };
        Ok(response)
    })
    .await
}

async fn get_or_create_doc(
    State(state): State<SharedState>,
    Json(req): Json<GetOrCreateDocRequest>,
) -> AppResult<Json<GetOrCreateDocResponse>> {
    blocking(move || {
        let mut result = state
            .page_service
            .get_or_create_doc(req.title.as_deref(), req.dir_path.as_deref())?;
        if result.created {
            result.items = state.extensions.page_created(&result.page)?;
        }
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
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePageRequest>,
) -> AppResult<Json<UpdatePageResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let (page, changed) = state.item_service.update_page(
            &page_id,
            req.title.as_deref(),
            req.dir_path.as_deref(),
            req.overwrite,
        )?;
        state.sse.broadcast_pages_updated(&client, &id);
        for page_id in changed {
            state.sse.broadcast_pages_updated(&client, page_id.as_str());
        }
        Ok(Json(UpdatePageResponse {
            page: state.dtos.page_to_dto(&page)?,
        }))
    })
    .await
}

async fn demote_page(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
    Json(req): Json<DemotePageRequest>,
) -> AppResult<Json<UpdatePageResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let (page, changed) = state
            .item_service
            .demote_index_page(&page_id, req.overwrite)?;
        state.sse.broadcast_pages_updated(&client, &id);
        for page_id in changed {
            state.sse.broadcast_pages_updated(&client, page_id.as_str());
        }
        Ok(Json(UpdatePageResponse {
            page: state.dtos.page_to_dto(&page)?,
        }))
    })
    .await
}

async fn upsert_property(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path((id, key)): Path<(String, String)>,
    Json(req): Json<UpsertPropertyRequest>,
) -> AppResult<Json<UpdatePageResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let tags_before = state.properties.find_tags("page", &id)?;
        let page = state.page_service.set_property(
            &page_id,
            &key,
            req.value_type.as_deref().unwrap_or("text"),
            req.value.as_deref().unwrap_or(""),
        )?;
        state.sse.broadcast_pages_updated(&client, &id);
        if key == "tags" {
            let tags_after = state.properties.find_tags("page", &id)?;
            state
                .extensions
                .tags_changed(&page, &tags_before, &tags_after);
        }
        Ok(Json(UpdatePageResponse {
            page: state.dtos.page_to_dto(&page)?,
        }))
    })
    .await
}

async fn delete_property(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path((id, key)): Path<(String, String)>,
) -> AppResult<Json<UpdatePageResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let page = state.page_service.delete_property(&page_id, &key)?;
        state.sse.broadcast_pages_updated(&client, &id);
        Ok(Json(UpdatePageResponse {
            page: state.dtos.page_to_dto(&page)?,
        }))
    })
    .await
}

async fn delete_page(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        state.page_service.delete_page(&page_id)?;
        state.sse.broadcast_pages_updated(&client, &id);
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

/// Opens the folder that holds the markdown file of a page in the file manager of the OS.
/// The folder of a page that was not exported yet does not exist, so it cannot be opened.
async fn open_export_folder(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let page = state
            .page_service
            .get_page(&page_id)?
            .ok_or_else(|| AppError::not_found(format!("Page not found: {id}")))?;
        let folder = state
            .export
            .export_folder_of(&page)
            .filter(|f| f.is_dir())
            .ok_or_else(|| AppError::not_found("Export folder not found"))?;
        if !launcher::open_folder(&folder) {
            return Err(AppError::internal("Failed to open the export folder"));
        }
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

async fn get_recent(
    State(state): State<SharedState>,
    Query(q): Query<RecentQuery>,
) -> AppResult<Json<PagesResponse>> {
    blocking(move || {
        let page_type = q.page_type.filter(|t| !t.is_empty());
        let pages = state
            .page_service
            .get_recent_pages(q.max_results.unwrap_or(10), page_type.as_deref())?;
        Ok(Json(pages_with_scalars(&state, &pages)?))
    })
    .await
}

async fn get_backlinks(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> AppResult<Json<GetBacklinksResponse>> {
    let page_id = parse_id(&id)?;
    blocking(move || {
        let entries = state.page_service.get_backlinks(&page_id)?;
        Ok(Json(GetBacklinksResponse {
            entries: entries.iter().map(backlink_dto).collect(),
        }))
    })
    .await
}

async fn get_children(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> AppResult<Json<PagesResponse>> {
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
