use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::application::journal_service::MostRecentJournalResult;
use crate::error::{AppError, AppResult};
use crate::presentation::dto::{ItemDto, PageDto};
use crate::state::{blocking, SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/journals/before", get(most_recent_before))
        .route("/api/journals/weekly/before", get(most_recent_weekly_before))
        .route("/api/journals/{name}", post(get_or_create))
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

async fn get_or_create(State(state): State<SharedState>, Path(name): Path<String>) -> AppResult<Json<GetOrCreateJournalResponse>> {
    blocking(move || {
        let result = state.journal_service.get_or_create_journal_by_name(&name)?;
        Ok(Json(GetOrCreateJournalResponse {
            page: state.dtos.page_to_dto(&result.page)?,
            items: state.dtos.to_dtos(&result.items)?,
            created: result.created,
            transcluded_trees: state.dtos.transcluded_trees(&result.items)?,
        }))
    })
    .await
}

fn to_response(state: &SharedState, result: MostRecentJournalResult) -> AppResult<Json<MostRecentResponse>> {
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

async fn most_recent_before(State(state): State<SharedState>, Query(q): Query<DateQuery>) -> AppResult<Json<MostRecentResponse>> {
    let date = q.date.unwrap_or_default();
    let before = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| AppError::bad_request(format!("Invalid date '{date}': {e}")))?;
    blocking(move || to_response(&state, state.journal_service.get_most_recent_journal_before(before)?)).await
}

async fn most_recent_weekly_before(State(state): State<SharedState>, Query(q): Query<WeekQuery>) -> AppResult<Json<MostRecentResponse>> {
    let week = q.week.unwrap_or_default();
    blocking(move || to_response(&state, state.journal_service.get_most_recent_weekly_journal_before(&week)?)).await
}
