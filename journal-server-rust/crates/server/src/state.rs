//! Everything the handlers and the scheduled tasks share, built once at startup.

use std::sync::Arc;

use crate::application::{configuration, folder_chooser};
use crate::config::Settings;
use crate::infrastructure::pending_delete_prompt::PendingDeletePrompt;
use crate::infrastructure::watch::MarkdownFolderWatcher;
use crate::presentation::sse::SseBroadcaster;
use journal_core::application::atlas_service::AtlasService;
use journal_core::application::combined_search::CombinedSearchService;
use journal_core::application::config_service::ConfigService;
use journal_core::application::confirm_service::ConfirmService;
use journal_core::application::extension_functions::ExtensionFunctionService;
use journal_core::application::extension_runtime::ExtensionRuntime;
use journal_core::application::extension_service::ExtensionService;
use journal_core::application::flat_markdown_splitter::FlatMarkdownSplitter;
use journal_core::application::item_service::ItemService;
use journal_core::application::journal_service::JournalService;
use journal_core::application::markdown_parser::MarkdownParser;
use journal_core::application::page_service::PageService;
use journal_core::application::startup;
use journal_core::domain::id_generator::IdGenerator;
use journal_core::domain::search::SearchIndex;
use journal_core::dto::ItemDtoAssembler;
use journal_core::infrastructure::export::export_queue_repository::ExportQueueRepository;
use journal_core::infrastructure::export::export_service::MarkdownExportService;
use journal_core::infrastructure::export::file_sync_state::FileSyncStateRepository;
use journal_core::infrastructure::export::import_service::MarkdownImportService;
use journal_core::infrastructure::export::item_merger::ItemMerger;
use journal_core::infrastructure::export::outline_parser::OutlineMarkdownParser;
use journal_core::infrastructure::export::page_matcher::PageMatcher;
use journal_core::infrastructure::export::reconcile_service::MarkdownReconcileService;
use journal_core::infrastructure::persistence::database_initializer;
use journal_core::infrastructure::persistence::{
    Db, ItemRepository, PageRepository, PropertyRepository,
};
use journal_core::infrastructure::search::{TantivySearchService, COMMIT_INTERVAL};
use journal_core::journal::{Journal, JournalServices};

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
    /// Opens the database and the search index, wires the services and runs the startup steps
    /// (configuration check, index rebuild, tag pages, atlas pages).
    pub fn build(settings: Settings, options: BuildOptions) -> anyhow::Result<SharedState> {
        let ids = Arc::new(IdGenerator::new());
        let db = Arc::new(Db::open(&settings.db_path)?);
        database_initializer::initialize(&db, &ids)?;

        let pages = PageRepository::new(Arc::clone(&db));
        let items = ItemRepository::new(Arc::clone(&db));
        let properties = PropertyRepository::new(Arc::clone(&db));
        let export_queue = ExportQueueRepository::new(Arc::clone(&db));
        let file_sync_state = FileSyncStateRepository::new(Arc::clone(&db));

        let tantivy = Arc::new(TantivySearchService::open(&settings.index_dir)?);
        let search: Arc<dyn SearchIndex> = tantivy.clone();
        let config = Arc::new(ConfigService::new(settings.config_dir.clone()));
        let parser = MarkdownParser;
        let splitter = FlatMarkdownSplitter::new(parser.clone());
        let sse = SseBroadcaster::new();

        let export = Arc::new(MarkdownExportService::new(
            Arc::clone(&config),
            export_queue.clone(),
            file_sync_state.clone(),
            pages.clone(),
            items.clone(),
            properties.clone(),
            std::time::Duration::from_secs(settings.export_overwrite_wait_secs),
        ));
        let atlas = Arc::new(AtlasService::new(
            pages.clone(),
            items.clone(),
            properties.clone(),
            parser.clone(),
            Arc::clone(&search),
            Arc::clone(&export),
            Arc::clone(&ids),
            Arc::new(sse.clone()),
        ));
        let page_service = Arc::new(PageService::new(
            pages.clone(),
            items.clone(),
            Arc::clone(&search),
            Arc::clone(&export),
            properties.clone(),
            Arc::clone(&ids),
            Arc::clone(&config),
            Arc::clone(&atlas),
        ));
        let item_service = Arc::new(ItemService::new(
            items.clone(),
            parser.clone(),
            Arc::clone(&search),
            Arc::clone(&export),
            properties.clone(),
            Arc::clone(&ids),
            Arc::clone(&page_service),
        ));
        let journal_service = Arc::new(JournalService::new(
            pages.clone(),
            items.clone(),
            Arc::clone(&search),
            Arc::clone(&config),
            Arc::clone(&item_service),
            Arc::clone(&ids),
            Arc::clone(&export),
        ));
        let combined_search = Arc::new(CombinedSearchService::new(
            pages.clone(),
            Arc::clone(&search),
            properties.clone(),
            items.clone(),
        ));

        let page_matcher = PageMatcher::new(
            pages.clone(),
            items.clone(),
            Arc::clone(&search),
            Arc::clone(&ids),
            Arc::clone(&page_service),
        );
        let item_merger = ItemMerger::new(
            items.clone(),
            parser.clone(),
            Arc::clone(&search),
            Arc::clone(&ids),
        );
        let import = Arc::new(MarkdownImportService::new(
            OutlineMarkdownParser::new(splitter.clone()),
            page_matcher,
            item_merger,
            file_sync_state.clone(),
            Arc::clone(&export),
            Arc::new(sse.clone()),
            Arc::clone(&page_service),
            std::time::Duration::from_secs(settings.import_copy_wait_secs),
        ));
        let prompt = Arc::new(PendingDeletePrompt::new(
            settings.port,
            settings.import_delete_prompt_cooldown_secs,
        ));
        let reconcile = Arc::new(MarkdownReconcileService::new(
            settings.import_enabled,
            Arc::clone(&export),
            Arc::clone(&import),
            file_sync_state.clone(),
            pages.clone(),
            prompt.clone(),
            Arc::clone(&search),
        ));
        let watcher = Arc::new(MarkdownFolderWatcher::new(
            settings.import_enabled,
            settings.import_debounce_ms,
            Arc::clone(&export),
            import.clone(),
            Arc::clone(&reconcile),
            file_sync_state.clone(),
            Arc::clone(&prompt),
            Arc::clone(&search),
        ));
        let functions = Arc::new(ExtensionFunctionService::load(Arc::clone(&config)));
        // A person may step away before answering, so the wait is long
        let confirm = Arc::new(ConfirmService::new(
            Arc::new(sse.clone()),
            std::time::Duration::from_secs(60),
        ));
        let extensions = Arc::new(ExtensionService::new(
            ExtensionRuntime::new(Arc::new(splitter.clone()), Arc::clone(&ids))
                .with_confirmer(confirm.clone()),
            Arc::clone(&functions),
            Arc::clone(&item_service),
            Arc::clone(&page_service),
            Arc::new(sse.clone()),
        ));
        let dtos = ItemDtoAssembler::new(properties.clone(), items.clone());
        let journal = Arc::new(Journal::new(
            JournalServices {
                combined_search,
                page_service: Arc::clone(&page_service),
                item_service: Arc::clone(&item_service),
                journal_service: Arc::clone(&journal_service),
                extensions: Arc::clone(&extensions),
                pages: pages.clone(),
                items: items.clone(),
                properties: properties.clone(),
                splitter: splitter.clone(),
                dtos: dtos.clone(),
            },
            Arc::new(sse.clone()),
        ));

        // Startup steps, in the order of the Java server
        if options.interactive {
            folder_chooser::ask_for_markdown_dir(&config);
        }
        let dir = config.markdown_dir();
        if !dir.trim().is_empty() {
            // Only the server makes the folders. A command run by hand must not make a
            // markdown directory that is not there, as when its drive is not mounted
            if options.interactive {
                configuration::ensure_sub_dirs(std::path::Path::new(&dir));
            }
            export.export_all_if_empty(&dir);
        }
        startup::rebuild_search_index(&pages, &items, &search);
        // After the rebuild, which skips an index that is not empty: the new pages are indexed
        // one by one as they are created
        startup::ensure_tag_pages(&page_service);
        startup::bring_atlases_up_to_date(&atlas);

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
