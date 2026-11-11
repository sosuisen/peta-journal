use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use chrono::NaiveDate;
use hmtid::Hmtid;
use serde::{Deserialize, Serialize};

use crate::error::{parse_id, AppError, AppResult, HttpResult};
use crate::presentation::client_id::ClientId;
use crate::state::{blocking, SharedState};
use journal_core::application::journal_service::MostRecentJournalResult;
use journal_core::domain::journal_name::JournalName;
use journal_core::domain::model::ItemPosition;
use journal_core::dto::{ItemDto, PageDto};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/journals/before", get(most_recent_before))
        .route(
            "/api/journals/weekly/before",
            get(most_recent_weekly_before),
        )
        .route("/api/journals/{name}", post(get_or_create))
        .route("/api/journals/{name}/move", post(move_items))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveToJournalRequest {
    item_ids: Vec<String>,
    position: String,
    /// The text the editor of the first item has now, when it is not saved yet.
    body: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveToJournalResponse {
    page: PageDto,
    created: bool,
    items: Vec<ItemDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetOrCreateJournalResponse {
    page: PageDto,
    items: Vec<ItemDto>,
    created: bool,
    transcluded_trees: BTreeMap<String, Vec<ItemDto>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MostRecentResponse {
    found: bool,
    page: Option<PageDto>,
    items: Vec<ItemDto>,
    transcluded_trees: BTreeMap<String, Vec<ItemDto>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct DateQuery {
    date: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct WeekQuery {
    week: Option<String>,
}

async fn get_or_create(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> HttpResult<Json<GetOrCreateJournalResponse>> {
    blocking(move || {
        let mut result = state
            .journal_service
            .get_or_create_journal_by_name(name.parse::<JournalName>()?)?;
        if result.created {
            result.items = state.extensions.page_created(&result.page)?;
        }
        Ok(Json(GetOrCreateJournalResponse {
            page: state.dtos.page_to_dto(&result.page)?,
            items: state.dtos.to_dtos(&result.items)?,
            created: result.created,
            transcluded_trees: state.dtos.transcluded_trees(&result.items)?,
        }))
    })
    .await
}

/// Moves items to the journal of a day, as a move key at the edge of a day does (ADR 011).
async fn move_items(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(name): Path<String>,
    Json(req): Json<MoveToJournalRequest>,
) -> HttpResult<Json<MoveToJournalResponse>> {
    let JournalName::Daily(date) = name.parse::<JournalName>().map_err(AppError::from)? else {
        return Err(AppError::bad_request("Items move to the journal of a day only").into());
    };
    let item_ids: Vec<Hmtid> = req
        .item_ids
        .iter()
        .map(|s| parse_id(s))
        .collect::<AppResult<_>>()?;
    let position = ItemPosition::parse(&req.position).map_err(AppError::bad_request)?;
    blocking(move || {
        let moved = state.journal_service.move_items_to_journal(
            date,
            &item_ids,
            &position,
            req.body.as_deref(),
        )?;
        if moved.created {
            // The extensions see the journal with the items that moved in
            state.extensions.page_created(&moved.page)?;
        }
        for page_id in &moved.page_ids {
            state.sse.broadcast_pages_updated(&client, page_id.as_str());
        }
        Ok(Json(MoveToJournalResponse {
            page: state.dtos.page_to_dto(&moved.page)?,
            created: moved.created,
            items: state.dtos.to_dtos(&moved.items)?,
        }))
    })
    .await
}

fn to_response(
    state: &SharedState,
    result: MostRecentJournalResult,
) -> AppResult<Json<MostRecentResponse>> {
    match result.page {
        None => Ok(Json(MostRecentResponse {
            found: false,
            page: None,
            items: Vec::new(),
            transcluded_trees: BTreeMap::new(),
        })),
        Some(page) => Ok(Json(MostRecentResponse {
            found: true,
            page: Some(state.dtos.page_to_dto(&page)?),
            items: state.dtos.to_dtos(&result.items)?,
            transcluded_trees: state.dtos.transcluded_trees(&result.items)?,
        })),
    }
}

async fn most_recent_before(
    State(state): State<SharedState>,
    Query(q): Query<DateQuery>,
) -> HttpResult<Json<MostRecentResponse>> {
    let date = q.date.unwrap_or_default();
    let before = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| AppError::bad_request(format!("Invalid date '{date}': {e}")))?;
    blocking(move || {
        to_response(
            &state,
            state
                .journal_service
                .get_most_recent_journal_before(before)?,
        )
    })
    .await
}

async fn most_recent_weekly_before(
    State(state): State<SharedState>,
    Query(q): Query<WeekQuery>,
) -> HttpResult<Json<MostRecentResponse>> {
    let week = q.week.unwrap_or_default();
    blocking(move || {
        to_response(
            &state,
            state
                .journal_service
                .get_most_recent_weekly_journal_before(&week)?,
        )
    })
    .await
}
