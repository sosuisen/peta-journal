//! Full scan of the markdown folder against `file_sync_state`. Runs shortly after startup and
//! then periodically, as a safety net for file events the watcher never sees.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use hmtid::Hmtid;

use crate::domain::search::SearchIndex;
use crate::infrastructure::persistence::PageRepository;

use super::export_service::{mtime_millis, MarkdownExportService};
use super::file_sync_state::{FileSyncState, FileSyncStateRepository};
use super::import_service::{MarkdownImportService, Outcome};
use super::path_mapper;
use super::pending_delete_prompt::PendingDeletePrompt;

const TOP_DIRS: [&str; 2] = ["docs", "journals"];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanSummary {
    pub imported: i32,
    pub conflicts: i32,
    pub new_missing: i32,
    pub exported: i32,
}

pub struct MarkdownReconcileService {
    import_enabled: bool,
    export: Arc<MarkdownExportService>,
    import: Arc<MarkdownImportService>,
    file_sync_state: FileSyncStateRepository,
    pages: PageRepository,
    prompt: Arc<PendingDeletePrompt>,
    search: Arc<dyn SearchIndex>,
    scanning: AtomicBool,
    scan_requested: AtomicBool,
}

impl MarkdownReconcileService {
    pub fn new(
        import_enabled: bool,
        export: Arc<MarkdownExportService>,
        import: Arc<MarkdownImportService>,
        file_sync_state: FileSyncStateRepository,
        pages: PageRepository,
        prompt: Arc<PendingDeletePrompt>,
        search: Arc<dyn SearchIndex>,
    ) -> MarkdownReconcileService {
        MarkdownReconcileService {
            import_enabled,
            export,
            import,
            file_sync_state,
            pages,
            prompt,
            search,
            scanning: AtomicBool::new(false),
            scan_requested: AtomicBool::new(false),
        }
    }

    /// Asks for a scan on the next scheduler tick (used by the watcher for directory-level events).
    pub fn request_scan(&self) {
        self.scan_requested.store(true, Ordering::SeqCst);
    }

    /// Runs a requested scan, if any. Called every 2 seconds.
    pub fn run_requested_scan(self: &Arc<Self>) {
        if self.scan_requested.swap(false, Ordering::SeqCst) {
            self.scan_async();
        }
    }

    /// Runs a scan on a background thread unless one is already running.
    pub fn scan_async(self: &Arc<Self>) {
        if !self.import_enabled {
            return;
        }
        if self.scanning.swap(true, Ordering::SeqCst) {
            return;
        }
        let me = Arc::clone(self);
        std::thread::Builder::new()
            .name("markdown-scan".into())
            .spawn(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| me.do_scan()));
                if result.is_err() {
                    tracing::error!("Folder scan failed");
                }
                me.scanning.store(false, Ordering::SeqCst);
            })
            .map(|_| ())
            .unwrap_or_else(|e| {
                tracing::error!("Cannot start scan thread: {e}");
                self.scanning.store(false, Ordering::SeqCst);
            });
    }

    /// Runs a scan on the calling thread. Returns zeros if a scan is already running.
    pub fn scan(&self) -> ScanSummary {
        if !self.import_enabled {
            return ScanSummary::default();
        }
        if self.scanning.swap(true, Ordering::SeqCst) {
            return ScanSummary::default();
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.do_scan()));
        self.scanning.store(false, Ordering::SeqCst);
        result.unwrap_or_else(|_| {
            tracing::error!("Folder scan failed");
            ScanSummary::default()
        })
    }

    fn do_scan(&self) -> ScanSummary {
        let started = std::time::Instant::now();
        let Some(root) = self.export.markdown_root() else {
            return ScanSummary::default();
        };
        // If neither top folder is there, the folder is not really available (unmounted, selective sync off)
        if !TOP_DIRS.iter().any(|d| root.join(d).is_dir()) {
            tracing::warn!(
                "Neither docs/ nor journals/ found under {}; skipping scan",
                root.display()
            );
            return ScanSummary::default();
        }

        let mut states: BTreeMap<String, FileSyncState> = BTreeMap::new();
        let mut pages_with_state: BTreeSet<Hmtid> = BTreeSet::new();
        match self.file_sync_state.find_all() {
            Ok(all) => {
                for s in all {
                    pages_with_state.insert(s.page_id.clone());
                    states.insert(s.rel_path.clone(), s);
                }
            }
            Err(e) => {
                tracing::error!("Cannot read file sync state: {e}");
                return ScanSummary::default();
            }
        }

        let mut summary = ScanSummary::default();
        for top in TOP_DIRS {
            let dir = root.join(top);
            if !dir.is_dir() {
                continue;
            }
            let mut files = Vec::new();
            if let Err(e) = walk_files(&dir, &mut files) {
                tracing::warn!("Cannot walk {}: {e}", dir.display());
                continue;
            }
            for file in files {
                let rel = path_mapper::relativize(&root, &file);
                if path_mapper::is_ignored(&rel) {
                    continue;
                }
                let state = states.remove(&rel);
                if let Some(s) = &state {
                    if !s.is_missing() && unchanged_on_disk(&file, s) {
                        continue;
                    }
                }
                let t0 = std::time::Instant::now();
                let outcome = self.import.sync_file(&root, &rel);
                let ms = t0.elapsed().as_millis();
                if ms > 5000 {
                    tracing::warn!("Syncing {rel} took {ms} ms");
                }
                match outcome {
                    Outcome::Imported => summary.imported += 1,
                    Outcome::Conflict => summary.conflicts += 1,
                    _ => {}
                }
            }
        }
        if summary.imported + summary.conflicts > 0 {
            self.search.flush();
        }

        // Files we knew about that are no longer there
        for state in states.values() {
            if state.is_missing() {
                continue;
            }
            if root.join(&state.rel_path).exists() {
                continue; // exists but ignored (e.g. renamed to .conflict)
            }
            if let Err(e) = self.file_sync_state.mark_missing(&state.rel_path) {
                tracing::warn!("Cannot mark {} missing: {e}", state.rel_path);
                continue;
            }
            summary.new_missing += 1;
        }

        // Pages that never got a file (fresh install, or created while the folder was unavailable)
        if let Ok(all_pages) = self.pages.find_all() {
            for page in all_pages {
                if pages_with_state.contains(&page.id) {
                    continue;
                }
                if root.join(path_mapper::to_rel_path_of(&page)).exists() {
                    continue;
                }
                self.export.queue_export(&page.id);
                summary.exported += 1;
            }
        }

        // Keep asking while anything is left undecided; the prompt's cooldown limits how often
        let pending_missing = self
            .file_sync_state
            .find_missing()
            .map(|m| m.len())
            .unwrap_or(0);
        if pending_missing > 0 {
            tracing::info!(
                "{pending_missing} markdown file(s) missing ({} new); asking the user what to do",
                summary.new_missing
            );
            self.prompt.notify_new_missing();
        }
        tracing::info!(
            "Folder scan finished in {} ms: {} imported, {} conflicts, {} missing, {} queued for export",
            started.elapsed().as_millis(),
            summary.imported,
            summary.conflicts,
            summary.new_missing,
            summary.exported
        );
        summary
    }
}

fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_files(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

fn unchanged_on_disk(file: &Path, state: &FileSyncState) -> bool {
    match std::fs::metadata(file) {
        Ok(meta) => meta.len() as i64 == state.file_size && mtime_millis(&meta) == state.file_mtime,
        Err(_) => false,
    }
}
