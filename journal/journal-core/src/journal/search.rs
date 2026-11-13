//! The search operations.

use serde::Serialize;

use crate::domain::model::SearchResult;
use crate::dto::{search_result_dto, SearchResultDto};
use crate::error::AppResult;

use super::Journal;

/// How many results a search gives when the caller names no number.
const DEFAULT_MAX_RESULTS: usize = 20;

/// The answer of every search.
#[derive(Debug, Clone, Serialize)]
pub struct SearchAnswer {
    pub results: Vec<SearchResultDto>,
}

impl SearchAnswer {
    fn of(results: &[SearchResult]) -> SearchAnswer {
        SearchAnswer {
            results: results.iter().map(search_result_dto).collect(),
        }
    }
}

impl Journal {
    /// Finds pages by their names.
    pub fn search_page_names(
        &self,
        query: &str,
        max_results: Option<usize>,
    ) -> AppResult<SearchAnswer> {
        let max_results = max_results.unwrap_or(DEFAULT_MAX_RESULTS);
        let results = self
            .services
            .combined_search
            .search_page_names_internal(query, max_results)?;
        Ok(SearchAnswer::of(&results))
    }

    /// Finds pages and items by tags, properties, page paths, dates and text.
    pub fn search_combined(
        &self,
        query: &str,
        max_results: Option<usize>,
    ) -> AppResult<SearchAnswer> {
        let max_results = max_results.unwrap_or(DEFAULT_MAX_RESULTS);
        let results = self.services.combined_search.search(query, max_results)?;
        Ok(SearchAnswer::of(&results))
    }

    /// Finds the items that have the tag. The case of the letters does not count.
    pub fn search_by_tag(&self, tag: &str, max_results: Option<usize>) -> AppResult<SearchAnswer> {
        let max_results = max_results.unwrap_or(DEFAULT_MAX_RESULTS);
        let ids = self
            .services
            .properties
            .find_item_ids_by_tag_value_ignoring_case(tag, max_results as i64)?;
        let results = self.services.combined_search.results_for_items(&ids)?;
        Ok(SearchAnswer::of(&results))
    }
}
