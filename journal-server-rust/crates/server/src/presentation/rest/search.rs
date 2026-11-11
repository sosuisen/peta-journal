use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::HttpResult;
use crate::state::{blocking, SharedState};
use journal_core::dto::{search_result_dto, SearchResultDto};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/search/pages", get(search_page_names))
        .route("/api/search/combined", get(search_combined))
        .route("/api/search/tags", get(search_by_tag))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct SearchQuery {
    q: Option<String>,
    max_results: Option<usize>,
}

#[derive(Serialize)]
struct SearchResponse {
    results: Vec<SearchResultDto>,
}

async fn search_page_names(
    State(state): State<SharedState>,
    Query(q): Query<SearchQuery>,
) -> HttpResult<Json<SearchResponse>> {
    let query = q.q.unwrap_or_default();
    let max_results = q.max_results.unwrap_or(20);
    blocking(move || {
        let results = state
            .combined_search
            .search_page_names_internal(&query, max_results)?;
        Ok(Json(SearchResponse {
            results: results.iter().map(search_result_dto).collect(),
        }))
    })
    .await
}

async fn search_combined(
    State(state): State<SharedState>,
    Query(q): Query<SearchQuery>,
) -> HttpResult<Json<SearchResponse>> {
    let query = q.q.unwrap_or_default();
    let max_results = q.max_results.unwrap_or(20);
    blocking(move || {
        let results = state.combined_search.search(&query, max_results)?;
        Ok(Json(SearchResponse {
            results: results.iter().map(search_result_dto).collect(),
        }))
    })
    .await
}

async fn search_by_tag(
    State(state): State<SharedState>,
    Query(q): Query<SearchQuery>,
) -> HttpResult<Json<SearchResponse>> {
    let tag = q.q.unwrap_or_default();
    let max_results = q.max_results.unwrap_or(20);
    blocking(move || {
        let ids = state
            .properties
            .find_item_ids_by_tag_value_ignoring_case(&tag, max_results as i64)?;
        let results = state.combined_search.results_for_items(&ids)?;
        Ok(Json(SearchResponse {
            results: results.iter().map(search_result_dto).collect(),
        }))
    })
    .await
}
