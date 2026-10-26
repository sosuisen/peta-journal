//! Search over items and pages with filter terms: `#tag`, `key=value`, `in:path`,
//! `created:`/`modified:` date filters and plain text, combined with spaces (AND) and `OR`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use hmtid::Hmtid;

use crate::domain::model::{Page, SearchResult};
use crate::domain::search::SearchIndex;
use crate::error::AppResult;
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

/// Filter conditions are read in full, then intersected; this caps each of them.
const FILTER_LIMIT: i64 = 10000;
const SNIPPET_LENGTH: usize = 200;

pub struct CombinedSearchService {
    pages: PageRepository,
    search: Arc<dyn SearchIndex>,
    properties: PropertyRepository,
    items: ItemRepository,
}

/// The terms of one AND group, sorted by kind.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct QueryTerms {
    pub tags: Vec<String>,
    pub property_filters: Vec<(String, String)>,
    pub page_prefixes: Vec<String>,
    pub date_filters: Vec<(String, String)>,
    pub text: String,
}

impl QueryTerms {
    pub fn has_filters(&self) -> bool {
        !self.tags.is_empty()
            || !self.property_filters.is_empty()
            || !self.page_prefixes.is_empty()
            || !self.date_filters.is_empty()
    }

    pub fn parse(group: &str) -> QueryTerms {
        let mut terms = QueryTerms::default();
        let mut text_tokens: Vec<&str> = Vec::new();
        for token in group.split_whitespace() {
            if let Some(tag) = token.strip_prefix('#').filter(|t| !t.is_empty()) {
                terms.tags.push(tag.to_string());
            } else if let Some(p) = token.strip_prefix("in:").filter(|t| !t.is_empty()) {
                terms.page_prefixes.push(p.to_string());
            } else if let Some(d) = token.strip_prefix("created:").filter(|t| !t.is_empty()) {
                terms.date_filters.push(("created_date".to_string(), d.to_string()));
            } else if let Some(d) = token.strip_prefix("modified:").filter(|t| !t.is_empty()) {
                terms.date_filters.push(("modified_date".to_string(), d.to_string()));
            } else if token.contains('=') && !token.contains("==") && !token.starts_with('=') {
                let (k, v) = token.split_once('=').unwrap_or((token, ""));
                terms.property_filters.push((k.to_string(), v.to_string()));
            } else {
                text_tokens.push(token);
            }
        }
        terms.text = text_tokens.join(" ").trim().to_string();
        terms
    }
}

/// Splits by ` OR ` (case-sensitive, surrounded by whitespace).
fn split_or_groups(query: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for token in query.split_whitespace() {
        if token == "OR" && !current.is_empty() {
            groups.push(current.join(" "));
            current.clear();
        } else {
            current.push(token);
        }
    }
    groups.push(current.join(" "));
    groups
}

impl CombinedSearchService {
    pub fn new(
        pages: PageRepository,
        search: Arc<dyn SearchIndex>,
        properties: PropertyRepository,
        items: ItemRepository,
    ) -> CombinedSearchService {
        CombinedSearchService {
            pages,
            search,
            properties,
            items,
        }
    }

    pub fn search(&self, query: &str, max_results: usize) -> AppResult<Vec<SearchResult>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut merged: Vec<(String, SearchResult)> = Vec::new();
        for group in split_or_groups(query) {
            for r in self.execute_and_group(group.trim(), max_results)? {
                let key = if r.item_id.is_empty() {
                    r.page_id.to_string()
                } else {
                    format!("{}:{}", r.page_id, r.item_id)
                };
                if !merged.iter().any(|(k, _)| *k == key) {
                    merged.push((key, r));
                }
            }
        }
        Ok(merged.into_iter().map(|(_, r)| r).take(max_results).collect())
    }

    fn execute_and_group(&self, group: &str, max_results: usize) -> AppResult<Vec<SearchResult>> {
        if group.trim().is_empty() {
            return Ok(Vec::new());
        }
        let terms = QueryTerms::parse(group);
        let has_filters = terms.has_filters();
        tracing::info!(
            "Combined search: tags={:?}, props={}, prefixes={:?}, dates={}, text='{}'",
            terms.tags,
            terms.property_filters.len(),
            terms.page_prefixes,
            terms.date_filters.len(),
            terms.text
        );

        // Collect item ID sets for each condition (for AND intersection)
        let mut conditions: Vec<Vec<String>> = Vec::new();
        for tag in &terms.tags {
            let ids = self.properties.find_item_ids_by_tag_value_ignoring_case(tag, FILTER_LIMIT)?;
            tracing::info!("  tag '#{tag}' -> {} items", ids.len());
            conditions.push(ids);
        }
        for (k, v) in &terms.property_filters {
            let ids = self.properties.find_item_ids_by_scalar_property(k, v, FILTER_LIMIT)?;
            tracing::info!("  prop '{k}={v}' -> {} items", ids.len());
            conditions.push(ids);
        }
        for prefix in &terms.page_prefixes {
            let ids = self.properties.find_item_ids_by_page_prefix(prefix, FILTER_LIMIT)?;
            tracing::info!("  prefix 'in:{prefix}' -> {} items", ids.len());
            conditions.push(ids);
        }
        for (column, expr) in &terms.date_filters {
            let ids = self.properties.find_item_ids_by_date_range(column, expr, FILTER_LIMIT)?;
            tracing::info!("  date '{column}:{expr}' -> {} items", ids.len());
            conditions.push(ids);
        }

        // Text condition
        let text_query = terms.text.as_str();
        let mut text_item_results: Vec<SearchResult> = Vec::new();
        if !text_query.is_empty() {
            text_item_results = self.search.search_full_text(
                text_query,
                if has_filters { FILTER_LIMIT as usize } else { max_results },
            );
            if has_filters {
                let mut ids: Vec<String> = Vec::new();
                for r in &text_item_results {
                    if !r.item_id.is_empty() && !ids.contains(&r.item_id) {
                        ids.push(r.item_id.clone());
                    }
                }
                conditions.push(ids);
            }
        }

        // Pure text search: page results first, then the items of other pages
        if !has_filters {
            if text_query.is_empty() {
                return Ok(Vec::new());
            }
            let mut combined = Vec::new();
            let mut seen_pages: BTreeSet<String> = BTreeSet::new();
            for r in self.search_page_names_internal(text_query, max_results)? {
                seen_pages.insert(r.page_id.to_string());
                combined.push(r);
            }
            for r in text_item_results {
                if !seen_pages.contains(&r.page_id.to_string()) {
                    combined.push(r);
                }
            }
            combined.truncate(max_results);
            return Ok(combined);
        }

        // Intersect all conditions, keeping the order of the first one
        let mut matched: Vec<String> = Vec::new();
        if let Some(first) = conditions.first() {
            for id in first {
                if !matched.contains(id) {
                    matched.push(id.clone());
                }
            }
        }
        for cond in conditions.iter().skip(1) {
            let set: BTreeSet<&String> = cond.iter().collect();
            matched.retain(|id| set.contains(id));
        }
        tracing::info!("  intersection -> {} items", matched.len());

        // Items found by the text search keep their highlighted snippet
        let snippets: BTreeMap<&str, &SearchResult> = text_item_results
            .iter()
            .filter(|r| !r.item_id.is_empty())
            .map(|r| (r.item_id.as_str(), r))
            .collect();
        let mut results = Vec::new();
        for item_id in &matched {
            if results.len() >= max_results {
                break;
            }
            if let Some(r) = snippets.get(item_id.as_str()) {
                results.push((*r).clone());
                continue;
            }
            if let Some(r) = self.result_for_item(item_id)? {
                results.push(r);
            }
        }
        Ok(results)
    }

    /// Search results for the given items, with the start of the body as snippet.
    pub fn results_for_items(&self, item_ids: &[String]) -> AppResult<Vec<SearchResult>> {
        let mut results = Vec::new();
        for id in item_ids {
            if let Some(r) = self.result_for_item(id)? {
                results.push(r);
            }
        }
        Ok(results)
    }

    fn result_for_item(&self, item_id: &str) -> AppResult<Option<SearchResult>> {
        let Ok(id) = Hmtid::parse(item_id) else { return Ok(None) };
        let Some(item) = self.items.find_by_id(&id)? else { return Ok(None) };
        let Some(page) = self.pages.find_by_id(&item.page_id)? else { return Ok(None) };
        let snippet: String = item.body.chars().take(SNIPPET_LENGTH).collect();
        Ok(Some(SearchResult {
            page_id: page.id.clone(),
            page_title: page.title(),
            item_id: item_id.to_string(),
            snippet,
            score: 1.0,
        }))
    }

    /// Page-name search: index pages by directory segment and path prefixes first, then the
    /// search index, then a substring match.
    pub fn search_page_names_internal(&self, query: &str, max_results: usize) -> AppResult<Vec<SearchResult>> {
        let max = max_results as i64;
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut all: Vec<SearchResult> = Vec::new();
        let mut add = |p: &Page, all: &mut Vec<SearchResult>| {
            if seen.insert(p.id.to_string()) {
                all.push(SearchResult::from_page(p));
            }
        };
        for p in self.pages.find_index_pages_by_dir_segment(query, max)? {
            add(&p, &mut all);
        }
        for p in self.pages.find_by_path_prefix(query, max)? {
            add(&p, &mut all);
        }
        if !all.is_empty() {
            all.truncate(max_results);
            return Ok(all);
        }
        let indexed = self.search.search_page_names(query, max_results);
        if !indexed.is_empty() {
            return Ok(indexed);
        }
        Ok(self
            .pages
            .find_by_path_contains(query, max)?
            .iter()
            .map(SearchResult::from_page)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_terms() {
        let t = QueryTerms::parse("#report in:DailyReport/2026-03 status=done created:2026-03 hello world");
        assert_eq!(t.tags, vec!["report"]);
        assert_eq!(t.page_prefixes, vec!["DailyReport/2026-03"]);
        assert_eq!(t.property_filters, vec![("status".to_string(), "done".to_string())]);
        assert_eq!(t.date_filters, vec![("created_date".to_string(), "2026-03".to_string())]);
        assert_eq!(t.text, "hello world");
        assert!(t.has_filters());
    }

    #[test]
    fn or_groups() {
        assert_eq!(split_or_groups("#a OR #b"), vec!["#a", "#b"]);
        assert_eq!(split_or_groups("a b"), vec!["a b"]);
        assert_eq!(split_or_groups("OR a"), vec!["OR a"]);
    }
}
