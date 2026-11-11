//! Watches the markdown folder and feeds changed files to the import service. Events are debounced
//! per file so a burst (Dropbox catching up, an editor saving in several steps) turns into one
//! import once the file has been quiet for a while.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use journal_core::infrastructure::export::reconcile_service::MissingFilesPrompt;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;

use journal_core::domain::search::SearchIndex;
use journal_core::infrastructure::persistence::db::now_millis;

use super::pending_delete_prompt::PendingDeletePrompt;
use journal_core::infrastructure::export::export_service::MarkdownExportService;
use journal_core::infrastructure::export::file_sync_state::FileSyncStateRepository;
use journal_core::infrastructure::export::import_service::{MarkdownImportService, Outcome};
use journal_core::infrastructure::export::path_mapper;
use journal_core::infrastructure::export::reconcile_service::MarkdownReconcileService;
use journal_core::journal::FolderWatch;

const MAX_RETRIES: i32 = 3;

/// What the watcher needs of the import: taking in one file. The import service does it.
/// A test gives the watcher an import that fails, as a locked file makes the real one fail.
pub trait FileImport: Send + Sync {
    fn sync_file(&self, root: &Path, rel_path: &str) -> Outcome;
}

impl FileImport for MarkdownImportService {
    fn sync_file(&self, root: &Path, rel_path: &str) -> Outcome {
        MarkdownImportService::sync_file(self, root, rel_path)
    }
}

struct WatchState {
    watcher: Option<RecommendedWatcher>,
    root: Option<PathBuf>,
}

pub struct MarkdownFolderWatcher {
    import_enabled: bool,
    debounce_ms: i64,
    export: Arc<MarkdownExportService>,
    import: Arc<dyn FileImport>,
    reconcile: Arc<MarkdownReconcileService>,
    file_sync_state: FileSyncStateRepository,
    prompt: Arc<PendingDeletePrompt>,
    search: Arc<dyn SearchIndex>,
    /// rel_path -> time of the last event (millis)
    pending: Mutex<HashMap<String, i64>>,
    /// rel_path -> failed attempts so far; we give up after MAX_RETRIES and leave it to the scan
    retries: Mutex<HashMap<String, i32>>,
    state: Mutex<WatchState>,
}

impl MarkdownFolderWatcher {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        import_enabled: bool,
        debounce_ms: u64,
        export: Arc<MarkdownExportService>,
        import: Arc<dyn FileImport>,
        reconcile: Arc<MarkdownReconcileService>,
        file_sync_state: FileSyncStateRepository,
        prompt: Arc<PendingDeletePrompt>,
        search: Arc<dyn SearchIndex>,
    ) -> MarkdownFolderWatcher {
        MarkdownFolderWatcher {
            import_enabled,
            debounce_ms: debounce_ms as i64,
            export,
            import,
            reconcile,
            file_sync_state,
            prompt,
            search,
            pending: Mutex::new(HashMap::new()),
            retries: Mutex::new(HashMap::new()),
            state: Mutex::new(WatchState {
                watcher: None,
                root: None,
            }),
        }
    }

    /// Starts the periodic tasks: flushing debounced events every 500 ms and (re)attaching the
    /// watcher every 15 s until the folder is configured and mounted.
    pub fn start(self: &Arc<Self>) {
        if !self.import_enabled {
            tracing::info!("Folder -> DB import is disabled (import.enabled=false)");
            return;
        }
        let me = Arc::clone(self);
        tokio::spawn(async move {
            let mut flush = tokio::time::interval(std::time::Duration::from_millis(500));
            loop {
                flush.tick().await;
                let w = Arc::clone(&me);
                let _ = tokio::task::spawn_blocking(move || w.flush_pending()).await;
            }
        });
        let me = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let mut ensure = tokio::time::interval(std::time::Duration::from_secs(15));
            loop {
                ensure.tick().await;
                let w = Arc::clone(&me);
                let _ = tokio::task::spawn_blocking(move || w.ensure_watching()).await;
            }
        });
    }

    /// Called when the markdown directory setting changes.
    pub fn restart(self: &Arc<Self>) {
        self.stop_watching();
        self.ensure_watching();
    }

    fn ensure_watching(self: &Arc<Self>) {
        let Some(new_root) = self.export.markdown_root() else {
            if self.state.lock().watcher.is_some() {
                self.stop_watching();
            }
            return;
        };
        {
            let st = self.state.lock();
            if st.watcher.is_some() && st.root.as_deref() == Some(new_root.as_path()) {
                return;
            }
        }
        self.stop_watching();

        let me = Arc::clone(self);
        let root_for_events = new_root.clone();
        let watcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                Ok(event) => me.handle(&root_for_events, event),
                Err(e) => {
                    tracing::debug!("Watch error, requesting full scan: {e}");
                    me.reconcile.request_scan();
                }
            });
        let mut watcher = match watcher {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("Cannot watch {}: {e}", new_root.display());
                return;
            }
        };
        let mut any = false;
        for top in ["docs", "journals"] {
            let dir = new_root.join(top);
            if dir.is_dir() {
                match watcher.watch(&dir, RecursiveMode::Recursive) {
                    Ok(_) => any = true,
                    Err(e) => tracing::warn!("Cannot watch {}: {e}", dir.display()),
                }
            }
        }
        if !any {
            tracing::warn!(
                "Cannot watch {}: no docs/ or journals/ folder",
                new_root.display()
            );
        }
        {
            let mut st = self.state.lock();
            st.watcher = Some(watcher);
            st.root = Some(new_root.clone());
        }
        tracing::info!("Watching markdown folder: {}", new_root.display());
        self.reconcile.request_scan();
    }

    fn stop_watching(&self) {
        let mut st = self.state.lock();
        st.watcher = None;
        st.root = None;
    }

    fn handle(&self, root: &Path, event: Event) {
        match event.kind {
            EventKind::Other | EventKind::Any => {
                self.reconcile.request_scan();
                return;
            }
            _ => {}
        }
        for path in event.paths {
            if path.is_dir() {
                // A new or removed directory: pick up the files that came with it
                self.reconcile.request_scan();
                continue;
            }
            let rel = path_mapper::relativize(root, &path);
            if matches!(event.kind, EventKind::Remove(_)) && !rel.ends_with(path_mapper::MD_EXT) {
                // Could be a directory we were watching
                self.reconcile.request_scan();
                continue;
            }
            if path_mapper::is_ignored(&rel) {
                continue;
            }
            self.pending.lock().insert(rel, now_millis());
        }
    }

    /// Imports files that have been quiet for the debounce period.
    fn flush_pending(&self) {
        let root = match self.state.lock().root.clone() {
            Some(r) => r,
            None => return,
        };
        let cutoff = now_millis() - self.debounce_ms;
        let due: Vec<String> = {
            let mut pending = self.pending.lock();
            if pending.is_empty() {
                return;
            }
            let due: Vec<String> = pending
                .iter()
                .filter(|(_, t)| **t <= cutoff)
                .map(|(rel, _)| rel.clone())
                .collect();
            for rel in &due {
                pending.remove(rel);
            }
            due
        };
        let mut new_missing = 0;
        let mut index_changed = false;
        for rel in due {
            let outcome = self.import.sync_file(&root, &rel);
            match outcome {
                Outcome::Imported | Outcome::Conflict => {
                    index_changed = true;
                    self.retries.lock().remove(&rel);
                }
                Outcome::Missing => {
                    // A file that waited for this one may have come in as the page moved
                    index_changed = true;
                    if let Ok(Some(state)) = self.file_sync_state.find_by_path(&rel) {
                        if !state.is_missing() {
                            if let Err(e) = self.file_sync_state.mark_missing(&rel) {
                                tracing::warn!("Cannot mark {rel} missing: {e}");
                            } else {
                                new_missing += 1;
                            }
                        }
                    }
                    self.retries.lock().remove(&rel);
                }
                Outcome::Error => {
                    let n = {
                        let mut retries = self.retries.lock();
                        let n = retries.entry(rel.clone()).or_insert(0);
                        *n += 1;
                        *n
                    };
                    if n <= MAX_RETRIES {
                        self.pending.lock().entry(rel).or_insert_with(now_millis);
                    // try again after the next quiet period
                    } else {
                        self.retries.lock().remove(&rel);
                        tracing::warn!("Giving up on {rel} for now; the periodic scan will retry");
                    }
                }
                _ => {
                    self.retries.lock().remove(&rel);
                }
            }
        }
        if index_changed {
            self.search.flush();
        }
        if new_missing > 0 {
            tracing::info!("{new_missing} markdown file(s) deleted; asking the user what to do");
            self.prompt.notify_new_missing();
        }
    }
}

/// Lets the journal restart the watcher when the markdown directory setting changes.
pub struct WatcherRestart(pub Arc<MarkdownFolderWatcher>);

impl FolderWatch for WatcherRestart {
    fn restart(&self) {
        self.0.restart();
    }
}
