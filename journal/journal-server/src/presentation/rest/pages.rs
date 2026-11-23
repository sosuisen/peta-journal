use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json, Router};
use serde::Deserialize;

use crate::error::{parse_id, AppError, HttpError};
use crate::infrastructure::launcher;
use crate::presentation::client_id::ClientId;
use crate::presentation::rest::At;
use crate::state::{blocking, SharedState};
use journal_api_types::routes;
use journal_api_types::{
    DemotePageInput, FindPageQuery, GetOrCreateDocInput, PropertyInput, UpdatePageInput,
};
use journal_core::dto::ItemFields;
use journal_core::journal::{
    BacklinksAnswer, ChangedBy, DocAnswer, FoundPageAnswer, NarrowPage, PageAnswer, PagesAnswer,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .at(routes::GET_OR_CREATE_DOC, get_or_create_doc)
        .at(routes::FIND_PAGE, find_page)
        .at(routes::RECENT_PAGES, get_recent)
        .at(routes::GET_PAGE, get_page)
        .at(routes::UPDATE_PAGE, update_page)
        .at(routes::DELETE_PAGE, delete_page)
        .at(routes::BACKLINKS, get_backlinks)
        .at(routes::DEMOTE_PAGE, demote_page)
        .at(routes::CHILD_PAGES, get_children)
        .at(routes::OPEN_EXPORT_FOLDER, open_export_folder)
        .at(routes::UPSERT_PAGE_PROPERTY, upsert_property)
        .at(routes::DELETE_PAGE_PROPERTY, delete_property)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FieldsQuery {
    fields: Option<String>,
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
) -> Result<Response, HttpError> {
    blocking(move || {
        let whole = state.journal.get_page(&id)?;
        Ok(match ItemFields::parse(q.fields.as_deref()) {
            Some(fields) => Json(whole.with_fields(&fields)).into_response(),
            None => Json(whole).into_response(),
        })
    })
    .await
}

async fn get_or_create_doc(
    State(state): State<SharedState>,
    Json(req): Json<GetOrCreateDocInput>,
) -> Result<Json<DocAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.get_or_create_doc(
            req.title.as_deref(),
            req.dir_path.as_deref(),
        )?))
    })
    .await
}

async fn find_page(
    State(state): State<SharedState>,
    Query(q): Query<FindPageQuery>,
) -> Result<Json<FoundPageAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.find_page(q)?))).await
}

async fn update_page(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePageInput>,
) -> Result<Json<PageAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.update_page(
            &id,
            req.title.as_deref(),
            req.dir_path.as_deref(),
            req.overwrite,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn demote_page(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
    Json(req): Json<DemotePageInput>,
) -> Result<Json<PageAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.demote_page(
            &id,
            req.overwrite,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn upsert_property(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path((id, key)): Path<(String, String)>,
    Json(req): Json<PropertyInput>,
) -> Result<Json<PageAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.set_page_property(
            &id,
            &key,
            req.value_type.as_deref(),
            req.value.as_deref(),
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn delete_property(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path((id, key)): Path<(String, String)>,
) -> Result<Json<PageAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.delete_page_property(
            &id,
            &key,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn delete_page(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(id): Path<String>,
) -> Result<StatusCode, HttpError> {
    blocking(move || {
        state.journal.delete_page(&id, &ChangedBy::Client(client))?;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

/// Opens the folder that holds the markdown file of a page in the file manager of the OS.
/// The folder of a page that was not exported yet does not exist, so it cannot be opened.
///
/// It is not an operation of the journal: it changes nothing and hands over to the OS.
async fn open_export_folder(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<StatusCode, HttpError> {
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
) -> Result<Json<PagesAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state
                .journal
                .recent_pages(q.max_results, q.page_type.as_deref())?,
        ))
    })
    .await
}

async fn get_backlinks(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<BacklinksAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.backlinks(&id)?))).await
}

async fn get_children(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<PagesAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.child_pages(&id)?))).await
}
