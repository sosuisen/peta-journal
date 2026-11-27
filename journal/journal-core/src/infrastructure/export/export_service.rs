//! Writes pages to the markdown folder. A file that changed on disk and was not taken in yet,
//! as one another device sent, is not written over: the page waits in the queue until the
//! import has taken the file in. After a while the file is kept as a conflict backup instead.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hmtid::Hmtid;
use parking_lot::Mutex;

use crate::application::config_service::ConfigService;
use crate::application::flat_markdown_serializer;
use crate::domain::model::{Item, Page};
use crate::error::AppError;
use crate::infrastructure::files::{delete_quietly, write_atomic};
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

use super::export_queue_repository::{ExportQueueRepository, BATCH};
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
    /// How long a page waits for the import of its file before the file is written over.
    overwrite_wait: Duration,
    /// The pages that wait for the import of their file, and since when.
    waiting: Mutex<BTreeMap<Hmtid, Instant>>,
}

impl MarkdownExportService {
    pub fn new(
        config: Arc<ConfigService>,
        export_queue: ExportQueueRepository,
        file_sync_state: FileSyncStateRepository,
        pages: PageRepository,
        items: ItemRepository,
        properties: PropertyRepository,
        overwrite_wait: Duration,
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
            overwrite_wait,
            waiting: Mutex::new(BTreeMap::new()),
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
        let Ok(all_pages) = self.pages.find_all() else {
            return;
        };
        for page in &all_pages {
            self.queue_export(&page.id);
        }
        tracing::info!(
            "Directory empty, queued {} pages for export",
            all_pages.len()
        );
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

    /// The folder that holds the markdown file of a page. None when the markdown root is not
    /// configured or not reachable.
    pub fn export_folder_of(&self, page: &Page) -> Option<PathBuf> {
        let root = self.markdown_root()?;
        Some(path_mapper::export_folder(&root, page))
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

    /// True when the page has a change that its file does not hold yet: it waits in the queue.
    pub fn waits(&self, page_id: &Hmtid) -> bool {
        self.export_queue.holds(page_id).unwrap_or(false)
    }

    /// Writes every page that waits now, however many they are. A page that cannot be written
    /// goes back to the queue, and it stays there for the next call.
    pub fn process_whole_queue(&self) {
        let waiting = self.export_queue.waiting().unwrap_or(0);
        for _ in 0..waiting.div_ceil(BATCH) {
            self.process_export_queue();
        }
    }

    /// Writes the markdown file of one page right now, without going through the queue.
    /// Unchanged files are left alone. A page that no longer exists only loses its sync state.
    pub fn export_page(&self, page_id: &Hmtid) -> Result<(), AppError> {
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

        if self.has_changes_not_taken_in(&file_path, &rel_path, &markdown)? {
            let since = *self
                .waiting
                .lock()
                .entry(page_id.clone())
                .or_insert_with(Instant::now);
            if since.elapsed() < self.overwrite_wait {
                tracing::debug!("{rel_path} changed on disk, waiting for the import");
                self.queue_export(page_id);
                return Ok(());
            }
            backup_before_overwrite(&file_path, &page.name)?;
        }
        self.waiting.lock().remove(page_id);

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
            items
                .iter()
                .map(|i| i.modified_date)
                .fold(page.modified_date, i64::max),
        )?;
        tracing::debug!("Exported page {} to {}", page.name, file_path.display());
        Ok(())
    }

    /// Whether the file has content that was neither written here nor taken in: it differs
    /// from what the sync state recorded, and from what is to be written.
    fn has_changes_not_taken_in(
        &self,
        file: &Path,
        rel_path: &str,
        markdown: &str,
    ) -> Result<bool, AppError> {
        let Ok(meta) = std::fs::metadata(file) else {
            return Ok(false);
        };
        let state = self.file_sync_state.find_by_path(rel_path)?;
        if let Some(s) = &state {
            if !s.is_missing()
                && s.file_size == meta.len() as i64
                && s.file_mtime == mtime_millis(&meta)
            {
                return Ok(false);
            }
        }
        // Content that is not text cannot be taken in either; it is kept as a backup too
        let content = std::fs::read_to_string(file).unwrap_or_default();
        if content == markdown {
            return Ok(false);
        }
        let hash = path_mapper::sha256(&content);
        Ok(match &state {
            Some(s) => s.is_missing() || s.content_hash != hash,
            None => true,
        })
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

    pub fn serialize_to_outline_markdown(
        &self,
        page: &Page,
        items: &[Item],
    ) -> Result<String, AppError> {
        let mut sb = String::new();
        let page_id = page.id.to_string();
        let page_scalars = self
            .properties
            .find_scalar_by_targets("page", std::slice::from_ref(&page_id))?;
        let page_lists = self
            .properties
            .find_list_by_targets("page", std::slice::from_ref(&page_id))?;
        append_frontmatter(
            &mut sb,
            page,
            page_scalars.get(&page_id),
            page_lists.get(&page_id),
        );
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
            &|item| Some(format_instant(item.modified_date)),
        ));
        Ok(sb)
    }
}

/// The front matter: the page's id and dates, then its properties. A list property is
/// written as a YAML list, which is what `outline_parser` reads back.
pub fn append_frontmatter(
    sb: &mut String,
    page: &Page,
    scalars: Option<&BTreeMap<String, String>>,
    lists: Option<&BTreeMap<String, Vec<String>>>,
) {
    sb.push_str("---\n");
    sb.push_str(&format!("id: {}\n", page.id));
    sb.push_str(&format!("created: {}\n", format_instant(page.created_date)));
    sb.push_str(&format!(
        "modified: {}\n",
        format_instant(page.modified_date)
    ));
    for (key, value) in scalars.into_iter().flatten() {
        sb.push_str(&format!("{key}: {}\n", yaml_scalar(value)));
    }
    for (key, values) in lists.into_iter().flatten() {
        sb.push_str(&format!("{key}:\n"));
        for value in values {
            sb.push_str(&format!("  - {}\n", yaml_scalar(value)));
        }
    }
    sb.push_str("---\n\n");
}

/// Quotes a value that YAML would otherwise misread: an empty value, a `#` comment, a `: `
/// mapping, quotes, or spaces at either end. A date-time like `2026-01-02T03:04:05Z` stays
/// plain. `outline_parser` strips the quotes again.
fn yaml_scalar(value: &str) -> String {
    let plain = !value.is_empty()
        && !value.contains(['"', '\\'])
        && !value.contains(": ")
        && !value.contains(" #")
        && !value.starts_with('#')
        && !value.ends_with(':')
        && value.trim() == value
        && !value.starts_with(['-', '[', '{', '&', '*', '!', '|', '>', '%', '@', '`', '\'']);
    if plain {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('"', "\\\""))
    }
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
/// Keeps the file that is to be written over as a conflict backup next to it. The file is
/// not written over when this fails.
fn backup_before_overwrite(file: &Path, name: &str) -> std::io::Result<()> {
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup = file.with_file_name(path_mapper::conflict_file_name(name, &ts));
    std::fs::copy(file, &backup)?;
    tracing::warn!(
        "{} was not taken in; kept it as {} and wrote the page over it",
        file.display(),
        backup.display()
    );
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // 時刻はJavaと同じ形式に整形される。
    #[test]
    fn formats_instants_like_java() {
        assert_eq!(format_instant(0), "1970-01-01T00:00:00Z");
        assert_eq!(
            format_instant(1_700_000_000_123),
            "2023-11-14T22:13:20.123Z"
        );
    }
}
