//! The changes of an atlas page. The operations are in the journal (ADR 029).

use axum::extract::{Path, State};
use axum::{Json, Router};

use crate::error::HttpError;
use crate::presentation::rest::At;
use crate::state::{blocking, SharedState};
use journal_api_types::routes;
use journal_core::journal::{
    AtlasCalloutInput, AtlasGroupInput, AtlasLinkInput, AtlasTitleInput, ItemsAnswer,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .at(routes::ADD_ATLAS_LINK, add_link)
        .at(routes::SET_ATLAS_LINK, set_link)
        .at(routes::GROUP_ATLAS_CARDS, group)
        .at(routes::UNGROUP_ATLAS_CARDS, ungroup)
        .at(routes::SET_ATLAS_GROUP_TITLE, set_group_title)
        .at(routes::ADD_ATLAS_CALLOUT, add_callout)
        .at(routes::SET_ATLAS_CALLOUT, set_callout)
        .at(routes::REMOVE_ATLAS_ITEM, remove)
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
