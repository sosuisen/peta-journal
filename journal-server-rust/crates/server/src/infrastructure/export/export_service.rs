//! Writes pages to the markdown folder.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use hmtid::Hmtid;

use crate::application::config_service::ConfigService;
use crate::application::flat_markdown_serializer;
use crate::domain::model::{Item, Page};
use crate::error::{AppError, AppResult};
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

use super::export_queue_repository::ExportQueueRepository;
use super::file_sync_state::FileSyncStateRepository;
use super::{page_link_converter, path_mapper};

pub struct MarkdownExportService {
    config: Arc<ConfigService>,
    export_queue: ExportQueueRepository,
    file_sync_state: FileSyncStateRepository,
    pages: PageRepository,
    items: ItemRepository,
    properties: PropertyRepository,
    /// True while a scheduler run is writing files; a run that overlaps is skipped.
    processing: AtomicBool,
    warned_dir_unavailable: AtomicBool,
}

impl MarkdownExportService {
    pub fn new(
        config: Arc<ConfigService>,
        export_queue: ExportQueueRepository,
        file_sync_state: FileSyncStateRepository,
        pages: PageRepository,
        items: ItemRepository,
        properties: PropertyRepository,
    ) -> MarkdownExportService {
        MarkdownExportService {
            config,
            export_queue,
            file_sync_state,
            pages,
            items,
            properties,
            processing: AtomicBool::new(false),
            warned_dir_unavailable: AtomicBool::new(false),
        }
    }

    pub fn queue_export(&self, page_id: &Hmtid) {
        if let Err(e) = self.export_queue.enqueue(page_id) {
            tracing::warn!("Failed to queue export of page {page_id}: {e}");
        }
    }

    /// Queues every page when the markdown directory holds no .md file yet.
    pub fn export_all_if_empty(&self, dir: &str) {
        if dir.trim().is_empty() {
            return;
        }
        let path = Path::new(dir);
        if !path.is_dir() {
            return;
        }
        match has_markdown_files(path) {
            Ok(true) => return,
            Ok(false) => {}
            Err(e) => {
                tracing::warn!("Failed to scan directory {dir}: {e}");
                return;
            }
        }
        let Ok(all_pages) = self.pages.find_all() else { return };
        for page in &all_pages {
            self.queue_export(&page.id);
        }
        tracing::info!("Directory empty, queued {} pages for export", all_pages.len());
    }

    /// The markdown root if it is configured and currently reachable. None when Dropbox (or the
    /// drive) is not mounted; callers must not create it.
    pub fn markdown_root(&self) -> Option<PathBuf> {
        let markdown_dir = self.config.markdown_dir();
        if markdown_dir.trim().is_empty() {
            return None;
        }
        let root = PathBuf::from(&markdown_dir);
        if !root.is_dir() {
            if !self.warned_dir_unavailable.swap(true, Ordering::SeqCst) {
                tracing::warn!("Markdown directory is not available: {}", root.display());
            }
            return None;
        }
        if self.warned_dir_unavailable.swap(false, Ordering::SeqCst) {
            tracing::info!("Markdown directory is available again: {}", root.display());
        }
        Some(root)
    }

    /// Writes every queued page. Skipped when a run is already in progress.
    pub fn process_export_queue(&self) {
        if self.processing.swap(true, Ordering::SeqCst) {
            tracing::debug!("Export queue processing already in progress, skipping this run");
            return;
        }
        let page_ids = match self.export_queue.dequeue_all() {
            Ok(ids) => ids,
            Err(e) => {
                tracing::warn!("Failed to dequeue exports: {e}");
                self.processing.store(false, Ordering::SeqCst);
                return;
            }
        };
        for page_id in page_ids {
            if let Err(e) = self.export_page(&page_id) {
                tracing::warn!("Failed to export page {page_id}: {e}");
                // Re-queue for retry
                self.queue_export(&page_id);
            }
        }
        self.processing.store(false, Ordering::SeqCst);
    }

    /// Writes the markdown file of one page right now, without going through the queue.
    /// Unchanged files are left alone. A page that no longer exists only loses its sync state.
    pub fn export_page(&self, page_id: &Hmtid) -> AppResult<()> {
        let root = self
            .markdown_root()
            .ok_or_else(|| AppError::internal("Markdown directory not available"))?;
        let Some(page) = self.pages.find_by_id(page_id)? else {
            self.file_sync_state.delete_by_page_id(page_id)?;
            return Ok(());
        };
        let items = self.items.find_by_page_id(page_id)?;
        let markdown = self.serialize_to_outline_markdown(&page, &items)?;
        let rel_path = path_mapper::to_rel_path_of(&page);
        let hash = path_mapper::sha256(&markdown);

        // The page was renamed or moved: remove the file at its old location
        for stale in self.file_sync_state.find_by_page_id(page_id)? {
            if stale.rel_path != rel_path {
                delete_quietly(&root.join(&stale.rel_path));
                self.file_sync_state.delete_by_path(&stale.rel_path)?;
            }
        }

        let file_path = root.join(&rel_path);
        if let Some(state) = self.file_sync_state.find_by_path(&rel_path)? {
            if !state.is_missing() && hash == state.content_hash && file_path.exists() {
                tracing::debug!("Page {} unchanged on disk, skipping export", page.name);
                return Ok(());
            }
        }

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Err(e) = write_atomic(&file_path, &markdown) {
            // File locked by another process, write to alternate name
            let alt = file_path.with_file_name(format!("{}.conflict.md", page.name));
            tracing::warn!(
                "Cannot write {} ({e}), writing to alternate: {}",
                file_path.display(),
                alt.display()
            );
            std::fs::write(&alt, &markdown)?;
            return Ok(());
        }

        let meta = std::fs::metadata(&file_path)?;
        self.file_sync_state.upsert_synced(
            &rel_path,
            page_id,
            &hash,
            meta.len() as i64,
            mtime_millis(&meta),
        )?;
        tracing::debug!("Exported page {} to {}", page.name, file_path.display());
        Ok(())
    }

    /// Removes the markdown file of a page that was deleted from the DB. Safe when the file is gone.
    pub fn delete_page_file(&self, page: &Page) {
        if let Some(root) = self.markdown_root() {
            delete_quietly(&root.join(path_mapper::to_rel_path_of(page)));
            if let Ok(states) = self.file_sync_state.find_by_page_id(&page.id) {
                for state in states {
                    delete_quietly(&root.join(&state.rel_path));
                }
            }
        }
        if let Err(e) = self.file_sync_state.delete_by_page_id(&page.id) {
            tracing::warn!("Failed to delete file sync state of page {}: {e}", page.id);
        }
    }

    pub fn serialize_to_outline_markdown(&self, page: &Page, items: &[Item]) -> AppResult<String> {
        let mut sb = String::new();
        append_frontmatter(&mut sb, page);
        let item_ids: Vec<String> = items.iter().map(|i| i.id.to_string()).collect();
        let scalar_props = self.properties.find_scalar_by_targets("item", &item_ids)?;
        let list_props = self.properties.find_list_by_targets("item", &item_ids)?;
        // Links are written relative to this file so the exported Markdown works on its own
        let rel_path = path_mapper::to_rel_path_of(page);
        sb.push_str(&flat_markdown_serializer::serialize_with(
            items,
            &scalar_props,
            &list_props,
            &|body| page_link_converter::to_markdown_links(body, &rel_path),
        ));
        Ok(sb)
    }
}

fn append_frontmatter(sb: &mut String, page: &Page) {
    sb.push_str("---\n");
    sb.push_str(&format!("id: {}\n", page.id));
    sb.push_str(&format!("created: {}\n", format_instant(page.created_date)));
    sb.push_str(&format!("modified: {}\n", format_instant(page.modified_date)));
    sb.push_str("---\n\n");
}

/// Formats epoch millis like Java's `Instant.toString()`: ISO-8601 UTC with the fraction only
/// when it is not zero.
pub fn format_instant(millis: i64) -> String {
    use chrono::{DateTime, Utc};
    let dt = DateTime::<Utc>::from_timestamp_millis(millis).unwrap_or_default();
    if millis.rem_euclid(1000) == 0 {
        dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    } else {
        dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    }
}

/// Modification time of a file as epoch millis.
pub fn mtime_millis(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn has_markdown_files(dir: &Path) -> std::io::Result<bool> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if has_markdown_files(&path)? {
                return Ok(true);
            }
        } else if path.is_file() && path.to_string_lossy().ends_with(".md") {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Writes to a temp file in the same directory and renames it over the target, so Dropbox and
/// the folder watcher never see a half-written file.
fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    std::fs::write(&tmp, content)?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        delete_quietly(&tmp);
        return Err(e);
    }
    Ok(())
}

pub fn delete_quietly(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => tracing::debug!("Could not delete {}: {e}", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_instants_like_java() {
        assert_eq!(format_instant(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_instant(1_700_000_000_123), "2023-11-14T22:13:20.123Z");
    }
}
