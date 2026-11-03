//! Everything the handlers and the scheduled tasks share, built once at startup.

use std::sync::Arc;

use crate::application::atlas_service::AtlasService;
use crate::application::combined_search::CombinedSearchService;
use crate::application::config_service::ConfigService;
use crate::application::extension_functions::ExtensionFunctionService;
use crate::application::flat_markdown_splitter::FlatMarkdownSplitter;
use crate::application::item_service::ItemService;
use crate::application::journal_service::JournalService;
use crate::application::markdown_parser::MarkdownParser;
use crate::application::page_service::PageService;
use crate::application::startup;
use crate::config::Settings;
use crate::domain::id_generator::IdGenerator;
use crate::domain::search::SearchIndex;
use crate::infrastructure::export::export_queue_repository::ExportQueueRepository;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::export::file_sync_state::FileSyncStateRepository;
use crate::infrastructure::export::import_service::MarkdownImportService;
use crate::infrastructure::export::item_merger::ItemMerger;
use crate::infrastructure::export::outline_parser::OutlineMarkdownParser;
use crate::infrastructure::export::page_matcher::PageMatcher;
use crate::infrastructure::export::pending_delete_prompt::PendingDeletePrompt;
use crate::infrastructure::export::reconcile_service::MarkdownReconcileService;
use crate::infrastructure::persistence::database_initializer;
use crate::infrastructure::persistence::{Db, ItemRepository, PageRepository, PropertyRepository};
use crate::infrastructure::search::TantivySearchService;
use crate::infrastructure::watch::MarkdownFolderWatcher;
use crate::presentation::dto::ItemDtoAssembler;
use crate::presentation::sse::SseBroadcaster;

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
    pub combined_search: Arc<CombinedSearchService>,
    pub functions: Arc<ExtensionFunctionService>,
    pub dtos: ItemDtoAssembler,
    pub sse: SseBroadcaster,
}

pub type SharedState = Arc<AppState>;

/// Options for building the state: whether to ask for the markdown folder with a native dialog.
#[derive(Debug, Clone, Copy)]
pub struct BuildOptions {
    pub interactive: bool,
}

impl AppState {
    /// Opens the database and the search index, wires the services and runs the startup steps
    /// (schema check, configuration check, hashtag backfill, index rebuild).
    pub fn build(settings: Settings, options: BuildOptions) -> anyhow::Result<SharedState> {
        let ids = Arc::new(IdGenerator::new());
        let db = Arc::new(Db::open(&settings.db_path)?);
        database_initializer::initialize(&db, &ids)?;
        database_initializer::check_schema_version(&settings.config_dir);

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
            sse.clone(),
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
            Arc::clone(&prompt),
            Arc::clone(&search),
        ));
        let watcher = Arc::new(MarkdownFolderWatcher::new(
            settings.import_enabled,
            settings.import_debounce_ms,
            Arc::clone(&export),
            Arc::clone(&import),
            Arc::clone(&reconcile),
            file_sync_state.clone(),
            Arc::clone(&prompt),
            Arc::clone(&search),
        ));
        let functions = Arc::new(ExtensionFunctionService::load(Arc::clone(&config)));
        let dtos = ItemDtoAssembler::new(properties.clone(), items.clone());

        // Startup steps, in the order of the Java server
        if options.interactive {
            startup::check_configuration(&config, &export);
        } else {
            let dir = config.markdown_dir();
            if !dir.trim().is_empty() {
                export.export_all_if_empty(&dir);
            }
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
            combined_search,
            functions,
            dtos,
            sse,
        }))
    }

    /// Starts the periodic tasks: the search index commits, the export queue, the folder scans
    /// and the folder watcher.
    pub fn start_background_tasks(self: &Arc<Self>) {
        self.tantivy.start_background_commits();

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

/// Runs blocking work (SQLite, the search index, the file system) off the async runtime.
pub async fn blocking<T, F>(f: F) -> crate::error::AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> crate::error::AppResult<T> + Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(r) => r,
        Err(e) => Err(crate::error::AppError::internal(format!(
            "Task failed: {e}"
        ))),
    }
}
