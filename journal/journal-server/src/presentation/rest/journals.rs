use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::Deserialize;

use crate::error::HttpError;
use crate::presentation::client_id::ClientId;
use crate::state::{blocking, SharedState};
use journal_core::journal::{
    ChangedBy, JournalAnswer, MostRecentJournalAnswer, MoveToJournalInput, MovedToJournalAnswer,
};

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
) -> Result<Json<JournalAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.get_or_create_journal(&name)?))).await
}

async fn move_items(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(name): Path<String>,
    Json(req): Json<MoveToJournalInput>,
) -> Result<Json<MovedToJournalAnswer>, HttpError> {
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
) -> Result<Json<MostRecentJournalAnswer>, HttpError> {
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
) -> Result<Json<MostRecentJournalAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.most_recent_weekly_journal_before(
            q.week.as_deref().unwrap_or_default(),
        )?))
    })
    .await
}
