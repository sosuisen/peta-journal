//! The changes of an atlas page. The operations are in the journal (ADR 029).

use axum::extract::{Path, State};
use axum::routing::{delete, post, put};
use axum::{Json, Router};

use crate::error::HttpError;
use crate::state::{blocking, SharedState};
use journal_core::journal::{
    AtlasCalloutInput, AtlasGroupInput, AtlasLinkInput, AtlasTitleInput, ItemsAnswer,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/pages/{id}/atlas/links", post(add_link))
        .route("/api/pages/{id}/atlas/links/{item_id}", put(set_link))
        .route("/api/pages/{id}/atlas/group", post(group))
        .route("/api/pages/{id}/atlas/ungroup", post(ungroup))
        .route(
            "/api/pages/{id}/atlas/groups/{item_id}",
            put(set_group_title),
        )
        .route("/api/pages/{id}/atlas/callouts", post(add_callout))
        .route("/api/pages/{id}/atlas/callouts/{item_id}", put(set_callout))
        .route("/api/pages/{id}/atlas/items/{item_id}", delete(remove))
}

async fn add_link(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<AtlasLinkInput>,
) -> Result<Json<ItemsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.add_atlas_link(&id, req)?))).await
}

async fn set_link(
    State(state): State<SharedState>,
    Path((id, item_id)): Path<(String, String)>,
    Json(req): Json<AtlasLinkInput>,
) -> Result<Json<ItemsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.set_atlas_link(&id, &item_id, req)?))).await
}

async fn group(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<AtlasGroupInput>,
) -> Result<Json<ItemsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.group_atlas_cards(&id, req)?))).await
}

async fn ungroup(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<AtlasGroupInput>,
) -> Result<Json<ItemsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.ungroup_atlas_cards(&id, req)?))).await
}

async fn set_group_title(
    State(state): State<SharedState>,
    Path((id, item_id)): Path<(String, String)>,
    Json(req): Json<AtlasTitleInput>,
) -> Result<Json<ItemsAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(
            state.journal.set_atlas_group_title(&id, &item_id, req)?,
        ))
    })
    .await
}

async fn add_callout(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<AtlasCalloutInput>,
) -> Result<Json<ItemsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.add_atlas_callout(&id, req)?))).await
}

async fn set_callout(
    State(state): State<SharedState>,
    Path((id, item_id)): Path<(String, String)>,
    Json(req): Json<AtlasCalloutInput>,
) -> Result<Json<ItemsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.set_atlas_callout(&id, &item_id, req)?))).await
}

async fn remove(
    State(state): State<SharedState>,
    Path((id, item_id)): Path<(String, String)>,
) -> Result<Json<ItemsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.remove_atlas_item(&id, &item_id)?))).await
}
