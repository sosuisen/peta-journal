use std::collections::BTreeSet;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::domain::model::SearchResult;
use crate::error::AppResult;
use crate::presentation::dto::{search_result_dto, SearchResultDto};
use crate::state::{blocking, SharedState};

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

async fn search_page_names(State(state): State<SharedState>, Query(q): Query<SearchQuery>) -> AppResult<Json<SearchResponse>> {
    let query = q.q.unwrap_or_default();
    let max_results = q.max_results.unwrap_or(20);
    blocking(move || {
        let max = max_results as i64;
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut all: Vec<SearchResultDto> = Vec::new();

        // 0th: index pages whose last dir_path segment matches query
        for p in state.pages.find_index_pages_by_dir_segment(&query, max)? {
            if seen.insert(p.id.to_string()) {
                all.push(search_result_dto(&SearchResult::from_page(&p)));
            }
        }
        // 1st: prefix search on dir_path || name
        for p in state.pages.find_by_path_prefix(&query, max)? {
            if seen.insert(p.id.to_string()) {
                all.push(search_result_dto(&SearchResult::from_page(&p)));
            }
        }
        if !all.is_empty() {
            all.truncate(max_results);
            return Ok(Json(SearchResponse { results: all }));
        }
        // 2nd: the search index (token based)
        let indexed = state.search.search_page_names(&query, max_results);
        if !indexed.is_empty() {
            return Ok(Json(SearchResponse {
                results: indexed.iter().map(search_result_dto).collect(),
            }));
        }
        // 3rd: contains search
        let pages = state.pages.find_by_path_contains(&query, max)?;
        Ok(Json(SearchResponse {
            results: pages
                .iter()
                .map(|p| search_result_dto(&SearchResult::from_page(p)))
                .collect(),
        }))
    })
    .await
}

async fn search_combined(State(state): State<SharedState>, Query(q): Query<SearchQuery>) -> AppResult<Json<SearchResponse>> {
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

async fn search_by_tag(State(state): State<SharedState>, Query(q): Query<SearchQuery>) -> AppResult<Json<SearchResponse>> {
    let tag = q.q.unwrap_or_default();
    let max_results = q.max_results.unwrap_or(20);
    blocking(move || {
        let ids = state.properties.find_item_ids_by_tag_value(&tag, max_results as i64)?;
        let results = state.combined_search.results_for_items(&ids)?;
        Ok(Json(SearchResponse {
            results: results.iter().map(search_result_dto).collect(),
        }))
    })
    .await
}
