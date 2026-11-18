use axum::extract::{Path, Query, State};
use axum::{Extension, Json, Router};
use serde::Deserialize;

use crate::error::HttpResult;
use crate::presentation::client_id::ClientId;
use crate::presentation::rest::At;
use crate::state::{blocking, SharedState};
use journal_api_types::routes;
use journal_core::journal::{
    ChangedBy, JournalAnswer, MostRecentJournalAnswer, MoveToJournalInput, MovedToJournalAnswer,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .at(routes::JOURNAL_BEFORE, most_recent_before)
        .at(routes::WEEKLY_JOURNAL_BEFORE, most_recent_weekly_before)
        .at(routes::GET_OR_CREATE_JOURNAL, get_or_create)
        .at(routes::MOVE_ITEMS_TO_JOURNAL, move_items)
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
) -> HttpResult<Json<JournalAnswer>> {
    blocking(move || Ok(Json(state.journal.get_or_create_journal(&name)?))).await
}

async fn move_items(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(name): Path<String>,
    Json(req): Json<MoveToJournalInput>,
) -> HttpResult<Json<MovedToJournalAnswer>> {
    blocking(move || {
        Ok(Json(state.journal.move_items_to_journal(
            &name,
            req,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn most_recent_before(
    State(state): State<SharedState>,
    Query(q): Query<DateQuery>,
) -> HttpResult<Json<MostRecentJournalAnswer>> {
    blocking(move || {
        Ok(Json(state.journal.most_recent_journal_before(
            q.date.as_deref().unwrap_or_default(),
        )?))
    })
    .await
}

async fn most_recent_weekly_before(
    State(state): State<SharedState>,
    Query(q): Query<WeekQuery>,
) -> HttpResult<Json<MostRecentJournalAnswer>> {
    blocking(move || {
        Ok(Json(state.journal.most_recent_weekly_journal_before(
            q.week.as_deref().unwrap_or_default(),
        )?))
    })
    .await
}
