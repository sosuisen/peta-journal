//! Everything the handlers and the scheduled tasks share, built once at startup.

use std::sync::Arc;
use std::time::Duration;

use crate::application::{configuration, folder_chooser};
use crate::config::Settings;
use crate::infrastructure::pending_delete_prompt::PendingDeletePrompt;
use crate::infrastructure::watch::{MarkdownFolderWatcher, WatcherRestart};
use crate::presentation::sse::SseBroadcaster;
use journal_core::application::atlas_service::AtlasService;
use journal_core::application::config_service::ConfigService;
use journal_core::application::confirm_service::ConfirmService;
use journal_core::application::extension_functions::ExtensionFunctionService;
use journal_core::application::extension_service::ExtensionService;
use journal_core::application::flat_markdown_splitter::FlatMarkdownSplitter;
use journal_core::application::item_service::ItemService;
use journal_core::application::journal_service::JournalService;
use journal_core::application::markdown_parser::MarkdownParser;
use journal_core::application::page_service::PageService;
use journal_core::assembly::{CoreHost, CoreSettings, JournalCore};
use journal_core::domain::id_generator::IdGenerator;
use journal_core::domain::search::SearchIndex;
use journal_core::dto::ItemDtoAssembler;
use journal_core::infrastructure::export::export_service::MarkdownExportService;
use journal_core::infrastructure::export::file_sync_state::FileSyncStateRepository;
use journal_core::infrastructure::export::import_service::MarkdownImportService;
use journal_core::infrastructure::export::reconcile_service::MarkdownReconcileService;
use journal_core::infrastructure::persistence::{
    Db, ItemRepository, PageRepository, PropertyRepository,
};
use journal_core::infrastructure::search::{TantivySearchService, COMMIT_INTERVAL};
use journal_core::journal::Journal;

pub struct AppState {
    pub settings: Settings,
    pub db: Arc<Db>,
    pub config: Arc<ConfigService>,
    pub pages: PageRepository,
    pub items: ItemRepository,
    pub properties: PropertyRepository,
    pub file_sync_state: FileSyncStateRepository,
    pub search: Arc<dyn SearchIndex>,
    pub tantivy: Arc<TantivySearchService>,
    pub parser: MarkdownParser,
    pub splitter: FlatMarkdownSplitter,
    pub export: Arc<MarkdownExportService>,
    pub import: Arc<MarkdownImportService>,
    pub reconcile: Arc<MarkdownReconcileService>,
    pub watcher: Arc<MarkdownFolderWatcher>,
    pub item_service: Arc<ItemService>,
    pub page_service: Arc<PageService>,
    pub atlas: Arc<AtlasService>,
    pub journal_service: Arc<JournalService>,
    /// The entrance that REST and MCP share (ADR 029). The operations move here one area at a
    /// time; the handlers of the other areas still call the services above.
    pub journal: Arc<Journal>,
    pub functions: Arc<ExtensionFunctionService>,
    pub extensions: Arc<ExtensionService>,
    pub confirm: Arc<ConfirmService>,
    pub dtos: ItemDtoAssembler,
    pub sse: SseBroadcaster,
    pub ids: Arc<IdGenerator>,
}

pub type SharedState = Arc<AppState>;

/// Options for building the state: whether to ask for the markdown folder with a native dialog.
#[derive(Debug, Clone, Copy)]
pub struct BuildOptions {
    pub interactive: bool,
}

impl AppState {
    /// Builds the journal core (ADR 029) with what the server does for it, and runs the
    /// startup steps (configuration check, index rebuild, tag pages, atlas pages).
    pub fn build(settings: Settings, options: BuildOptions) -> anyhow::Result<SharedState> {
        let sse = SseBroadcaster::new();
        let prompt = Arc::new(PendingDeletePrompt::new(
            settings.port,
            settings.import_delete_prompt_cooldown_secs,
        ));
        let watcher_restart = Arc::new(WatcherRestart::default());
        let core = JournalCore::build(
            &CoreSettings {
                db_path: settings.db_path.clone(),
                index_dir: settings.index_dir.clone(),
                config_dir: settings.config_dir.clone(),
                import_enabled: settings.import_enabled,
                import_copy_wait: Duration::from_secs(settings.import_copy_wait_secs),
                export_overwrite_wait: Duration::from_secs(settings.export_overwrite_wait_secs),
            },
            CoreHost {
                journal_events: Arc::new(sse.clone()),
                atlas_events: Arc::new(sse.clone()),
                import_events: Arc::new(sse.clone()),
                extension_events: Arc::new(sse.clone()),
                confirm_events: Arc::new(sse.clone()),
                missing_files: prompt.clone(),
                folder_watch: watcher_restart.clone(),
            },
        )?;
        let watcher = Arc::new(MarkdownFolderWatcher::new(
            settings.import_enabled,
            settings.import_debounce_ms,
            Arc::clone(&core.export),
            core.import.clone(),
            Arc::clone(&core.reconcile),
            core.file_sync_state.clone(),
            prompt,
            Arc::clone(&core.search),
        ));
        watcher_restart.set(Arc::clone(&watcher));

        if options.interactive {
            folder_chooser::ask_for_markdown_dir(&core.config);
            // Only the server makes the folders. A command run by hand must not make a
            // markdown directory that is not there, as when its drive is not mounted
            let dir = core.config.markdown_dir();
            if !dir.trim().is_empty() {
                configuration::ensure_sub_dirs(std::path::Path::new(&dir));
            }
        }
        core.start_up();

        let JournalCore {
            db,
            config,
            pages,
            items,
            properties,
            file_sync_state,
            search,
            tantivy,
            parser,
            splitter,
            export,
            import,
            reconcile,
            item_service,
            page_service,
            atlas,
            journal_service,
            journal,
            functions,
            extensions,
            confirm,
            dtos,
            ids,
        } = core;
        Ok(Arc::new(AppState {
            settings,
            db,
            config,
            pages,
            items,
            properties,
            file_sync_state,
            search,
            tantivy,
            parser,
            splitter,
            export,
            import,
            reconcile,
            watcher,
            item_service,
            page_service,
            atlas,
            journal_service,
            journal,
            functions,
            extensions,
            confirm,
            dtos,
            sse,
            ids,
        }))
    }

    /// Commits the pending writes of the search index every `COMMIT_INTERVAL`. The index
    /// starts no task of its own (ADR 029).
    ///
    /// The tests start this task alone. The other tasks would do their work at times a test
    /// does not know.
    pub fn start_index_commits(&self) {
        let index = Arc::clone(&self.tantivy);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(COMMIT_INTERVAL);
            loop {
                tick.tick().await;
                let index = Arc::clone(&index);
                let _ = tokio::task::spawn_blocking(move || index.commit_pending()).await;
            }
        });
    }

    /// Starts the periodic tasks: the search index commits, the export queue, the folder scans
    /// and the folder watcher.
    pub fn start_background_tasks(self: &Arc<Self>) {
        self.start_index_commits();

        let export = Arc::clone(&self.export);
        let every = std::time::Duration::from_secs(self.settings.export_interval_secs);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let mut tick = tokio::time::interval(every);
            loop {
                tick.tick().await;
                let e = Arc::clone(&export);
                let _ = tokio::task::spawn_blocking(move || e.process_export_queue()).await;
            }
        });

        let reconcile = Arc::clone(&self.reconcile);
        let every = std::time::Duration::from_secs(self.settings.import_scan_interval_secs);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let mut tick = tokio::time::interval(every);
            loop {
                tick.tick().await;
                reconcile.scan_async();
            }
        });

        let reconcile = Arc::clone(&self.reconcile);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(2));
            loop {
                tick.tick().await;
                reconcile.run_requested_scan();
            }
        });

        self.watcher.start();
    }

    /// Writes what is still pending before the process ends.
    pub fn shutdown(&self) {
        self.search.flush();
    }
}

/// Runs blocking work (SQLite, the search index, the file system) off the async runtime. The
/// error of the work comes out as a handler answers with it.
pub async fn blocking<T, F>(f: F) -> crate::error::HttpResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> crate::error::AppResult<T> + Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(r) => Ok(r?),
        Err(e) => Err(crate::error::AppError::internal(format!("Task failed: {e}")).into()),
    }
}
