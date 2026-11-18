use axum::extract::{Query, State};
use axum::{Json, Router};
use serde::Deserialize;

use crate::error::HttpError;
use crate::presentation::rest::At;
use crate::state::{blocking, SharedState};
use journal_api_types::routes;
use journal_core::journal::SearchAnswer;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .at(routes::SEARCH_PAGE_NAMES, search_page_names)
        .at(routes::SEARCH_COMBINED, search_combined)
        .at(routes::SEARCH_BY_TAG, search_by_tag)
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct SearchQuery {
    q: Option<String>,
    max_results: Option<usize>,
}

async fn search_page_names(
    State(state): State<SharedState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchAnswer>, HttpError> {
    let query = q.q.unwrap_or_default();
    blocking(move || {
        Ok(Json(
            state.journal.search_page_names(&query, q.max_results)?,
        ))
    })
    .await
}

async fn search_combined(
    State(state): State<SharedState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchAnswer>, HttpError> {
    let query = q.q.unwrap_or_default();
    blocking(move || Ok(Json(state.journal.search_combined(&query, q.max_results)?))).await
}

async fn search_by_tag(
    State(state): State<SharedState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchAnswer>, HttpError> {
    let tag = q.q.unwrap_or_default();
    blocking(move || Ok(Json(state.journal.search_by_tag(&tag, q.max_results)?))).await
}
