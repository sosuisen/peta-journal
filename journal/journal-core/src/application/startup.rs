//! Startup steps: the search index rebuild, the missing tag pages and the atlas pages.

use std::sync::Arc;

use crate::domain::search::SearchIndex;
use crate::infrastructure::persistence::{ItemRepository, PageRepository};

use super::atlas_service::AtlasService;
use super::page_service::PageService;

/// Creates the tag pages that are missing. Tags added before tag pages were made have none.
/// Tag pages that repeat another in a different case go first, so their files stop
/// overwriting each other.
pub fn ensure_tag_pages(page_service: &PageService) {
    match page_service.delete_case_duplicate_tag_pages() {
        Ok(0) => {}
        Ok(n) => tracing::info!("Deleted {n} tag pages that repeated another in a different case"),
        Err(e) => tracing::warn!("Tag pages repeated in a different case were not deleted: {e}"),
    }
    match page_service.ensure_all_tag_pages() {
        Ok(0) => {}
        Ok(n) => tracing::info!("Created {n} missing tag pages"),
        Err(e) => tracing::warn!("Missing tag pages were not created: {e}"),
    }
}

/// Moves the atlas pages that the server wrote before to the form of plain pages (ADR 053).
/// The server writes no atlas by itself any more.
pub fn move_atlases_to_plain_form(atlas: &AtlasService) {
    if let Err(e) = atlas.move_to_plain_form() {
        tracing::warn!("The atlas pages were not moved to the plain form: {e}");
    }
}

/// Builds the search index from the database when it is empty.
pub fn rebuild_search_index(
    pages: &PageRepository,
    items: &ItemRepository,
    search: &Arc<dyn SearchIndex>,
) {
    if !search.is_empty() {
        return;
    }
    tracing::info!("Building search index...");
    let all_pages = match pages.find_all() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Search index rebuild skipped: {e}");
            return;
        }
    };
    for page in &all_pages {
        search.index_page(page);
        let title = page.title();
        let Ok(page_items) = items.find_by_page_id(&page.id) else {
            continue;
        };
        let with_body: Vec<_> = page_items
            .into_iter()
            .filter(|i| !i.body.trim().is_empty())
            .collect();
        search.index_items(&with_body, &title);
    }
    search.flush();
    tracing::info!("Search index built: {} pages indexed", all_pages.len());
}
