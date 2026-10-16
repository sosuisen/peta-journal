//! Full-text search service for pages and items.

use crate::domain::model::{Item, Page, SearchResult};

pub trait SearchIndex: Send + Sync {
    /// Indexes a page for search.
    fn index_page(&self, page: &Page);
    /// Indexes an item for full-text search.
    fn index_item(&self, item: &Item, page_title: &str);
    /// Removes a page and all its items from the search index.
    fn remove_page(&self, page_id: &str);
    /// Removes an item from the search index.
    fn remove_item(&self, item_id: &str);
    /// Indexes many items of one page with a single commit.
    fn index_items(&self, items: &[Item], page_title: &str);
    /// Removes many items from the index with a single commit.
    fn remove_items(&self, item_ids: &[String]);
    /// Makes pending index changes durable.
    fn flush(&self);
    /// Searches for pages by name using token matching.
    fn search_page_names(&self, query: &str, max_results: usize) -> Vec<SearchResult>;
    /// Full-text search across all item content, with highlighted snippets.
    fn search_full_text(&self, query: &str, max_results: usize) -> Vec<SearchResult>;
    /// Deletes all entries from the search index.
    fn delete_all(&self);
    /// Whether the index contains no documents.
    fn is_empty(&self) -> bool;
}
