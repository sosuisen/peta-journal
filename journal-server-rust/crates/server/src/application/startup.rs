//! Startup checks: the markdown directory, the search index rebuild, the missing tag pages
//! and the atlas pages.

use std::path::Path;
use std::sync::Arc;

use crate::domain::search::SearchIndex;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::persistence::{ItemRepository, PageRepository};

use super::atlas_service::AtlasService;
use super::config_service::ConfigService;
use super::folder_chooser;
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

/// Writes every atlas page for the pages its directory has now: they may have changed while
/// the server was down.
pub fn bring_atlases_up_to_date(atlas: &AtlasService) {
    if let Err(e) = atlas.sync_all() {
        tracing::warn!("The atlas pages were not brought up to date: {e}");
    }
}

/// Makes sure a markdown directory is configured (asking with the native folder chooser if not),
/// creates its sub directories and queues a first export when it is empty.
pub fn check_configuration(config: &ConfigService, export: &MarkdownExportService) {
    let mut markdown_dir = config.markdown_dir();
    tracing::info!("Current markdown directory: [{markdown_dir}]");
    if markdown_dir.trim().is_empty() {
        tracing::warn!("Markdown directory not configured. Opening folder chooser...");
        let choice = folder_chooser::choose_folder();
        tracing::info!(
            "Folder chooser returned: selected={}, path=[{}]",
            choice.selected,
            choice.path
        );
        if choice.selected && !choice.path.is_empty() {
            config.set_markdown_dir(&choice.path);
            markdown_dir = choice.path;
            tracing::info!("Markdown directory configured: {markdown_dir}");
        } else {
            tracing::error!(
                "No markdown directory selected. Application may not function correctly."
            );
        }
    }
    if !markdown_dir.trim().is_empty() {
        ensure_sub_dirs(Path::new(&markdown_dir));
        export.export_all_if_empty(&markdown_dir);
    }
}

fn ensure_sub_dirs(markdown_path: &Path) {
    for name in ["assets", "docs", "journals"] {
        let dir = markdown_path.join(name);
        if !dir.exists() {
            match std::fs::create_dir_all(&dir) {
                Ok(_) => tracing::info!("Created directory: {}", dir.display()),
                Err(e) => tracing::error!("Failed to create directory: {}: {e}", dir.display()),
            }
        }
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
