//! Startup checks: the markdown directory, the hashtag backfill and the search index rebuild.

use std::path::Path;
use std::sync::Arc;

use crate::domain::hashtag;
use crate::domain::search::SearchIndex;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

use super::config_service::ConfigService;
use super::folder_chooser;

/// Makes sure a markdown directory is configured (asking with the native folder chooser if not),
/// creates its sub directories and queues a first export when it is empty.
pub fn check_configuration(config: &ConfigService, export: &MarkdownExportService) {
    let mut markdown_dir = config.markdown_dir();
    tracing::info!("Current markdown directory: [{markdown_dir}]");
    if markdown_dir.trim().is_empty() {
        tracing::warn!("Markdown directory not configured. Opening folder chooser...");
        let choice = folder_chooser::choose_folder();
        tracing::info!("Folder chooser returned: selected={}, path=[{}]", choice.selected, choice.path);
        if choice.selected && !choice.path.is_empty() {
            config.set_markdown_dir(&choice.path);
            markdown_dir = choice.path;
            tracing::info!("Markdown directory configured: {markdown_dir}");
        } else {
            tracing::error!("No markdown directory selected. Application may not function correctly.");
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

/// Tags every item from its hashtags once, when no tag exists yet.
pub fn backfill_hashtags(pages: &PageRepository, items: &ItemRepository, properties: &PropertyRepository) {
    if properties.find_any_tag_exists() {
        return;
    }
    tracing::info!("Backfilling hashtag tags for existing items...");
    let mut count = 0;
    let all_pages = match pages.find_all() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Hashtag backfill skipped: {e}");
            return;
        }
    };
    for page in all_pages {
        let Ok(page_items) = items.find_by_page_id(&page.id) else { continue };
        for item in page_items {
            if item.ast.trim().is_empty() {
                continue;
            }
            let hashtags = hashtag::extract_hashtags(&item.ast);
            if !hashtags.is_empty() {
                if let Err(e) = properties.sync_tags("item", item.id.as_str(), &hashtags) {
                    tracing::warn!("Failed to tag item {}: {e}", item.id);
                    continue;
                }
                count += 1;
            }
        }
    }
    tracing::info!("Hashtag backfill complete: {count} items tagged");
}

/// Builds the search index from the database when it is empty.
pub fn rebuild_search_index(pages: &PageRepository, items: &ItemRepository, search: &Arc<dyn SearchIndex>) {
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
        let Ok(page_items) = items.find_by_page_id(&page.id) else { continue };
        let with_body: Vec<_> = page_items.into_iter().filter(|i| !i.body.trim().is_empty()).collect();
        search.index_items(&with_body, &title);
    }
    search.flush();
    tracing::info!("Search index built: {} pages indexed", all_pages.len());
}
