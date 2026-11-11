//! The changes of an atlas page. The server writes the outline of an atlas page, so every
//! change of it but the place of a card or a callout comes here. Each answers with the items
//! of the page.

use axum::extract::{Path, State};
use axum::routing::{delete, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::{parse_id, AppResult, HttpResult};
use crate::state::{blocking, SharedState};
use journal_core::domain::bounds::Bounds;
use journal_core::domain::model::Item;
use journal_core::dto::ItemDto;

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

#[derive(Serialize)]
struct ItemsResponse {
    items: Vec<ItemDto>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct LinkRequest {
    target: String,
    bounds: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct GroupRequest {
    /// Each a card by its item id, or a page card by the id of its page
    cards: Vec<String>,
    target: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct TitleRequest {
    side: String,
    title: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct CalloutRequest {
    bounds: String,
    side: String,
    text: String,
}

fn respond(state: &SharedState, items: AppResult<Vec<Item>>) -> AppResult<Json<ItemsResponse>> {
    Ok(Json(ItemsResponse {
        items: state.dtos.to_dtos(&items?)?,
    }))
}

async fn add_link(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<LinkRequest>,
) -> HttpResult<Json<ItemsResponse>> {
    let atlas_id = parse_id(&id)?;
    blocking(move || {
        let bounds = req.bounds.parse::<Bounds>()?;
        let items = state.atlas.add_link(&atlas_id, &req.target, bounds);
        respond(&state, items)
    })
    .await
}

async fn set_link(
    State(state): State<SharedState>,
    Path((id, item_id)): Path<(String, String)>,
    Json(req): Json<LinkRequest>,
) -> HttpResult<Json<ItemsResponse>> {
    let atlas_id = parse_id(&id)?;
    let item_id = parse_id(&item_id)?;
    blocking(move || {
        let items = state.atlas.set_link(&atlas_id, &item_id, &req.target);
        respond(&state, items)
    })
    .await
}

async fn group(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<GroupRequest>,
) -> HttpResult<Json<ItemsResponse>> {
    let atlas_id = parse_id(&id)?;
    blocking(move || {
        let items = state.atlas.group(&atlas_id, &req.cards, &req.target);
        respond(&state, items)
    })
    .await
}

async fn ungroup(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<GroupRequest>,
) -> HttpResult<Json<ItemsResponse>> {
    let atlas_id = parse_id(&id)?;
    blocking(move || {
        let items = state.atlas.ungroup(&atlas_id, &req.cards);
        respond(&state, items)
    })
    .await
}

async fn set_group_title(
    State(state): State<SharedState>,
    Path((id, item_id)): Path<(String, String)>,
    Json(req): Json<TitleRequest>,
) -> HttpResult<Json<ItemsResponse>> {
    let atlas_id = parse_id(&id)?;
    let item_id = parse_id(&item_id)?;
    blocking(move || {
        let items = state
            .atlas
            .set_group_title(&atlas_id, &item_id, &req.side, &req.title);
        respond(&state, items)
    })
    .await
}

async fn add_callout(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<CalloutRequest>,
) -> HttpResult<Json<ItemsResponse>> {
    let atlas_id = parse_id(&id)?;
    blocking(move || {
        let bounds = req.bounds.parse::<Bounds>()?;
        let items = state
            .atlas
            .add_callout(&atlas_id, bounds, &req.side, &req.text);
        respond(&state, items)
    })
    .await
}

async fn set_callout(
    State(state): State<SharedState>,
    Path((id, item_id)): Path<(String, String)>,
    Json(req): Json<CalloutRequest>,
) -> HttpResult<Json<ItemsResponse>> {
    let atlas_id = parse_id(&id)?;
    let item_id = parse_id(&item_id)?;
    blocking(move || {
        let items = state
            .atlas
            .set_callout(&atlas_id, &item_id, &req.side, &req.text);
        respond(&state, items)
    })
    .await
}

async fn remove(
    State(state): State<SharedState>,
    Path((id, item_id)): Path<(String, String)>,
) -> HttpResult<Json<ItemsResponse>> {
    let atlas_id = parse_id(&id)?;
    let item_id = parse_id(&item_id)?;
    blocking(move || {
        let items = state.atlas.remove(&atlas_id, &item_id);
        respond(&state, items)
    })
    .await
}
