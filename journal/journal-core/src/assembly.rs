//! Builds the journal core: opens the database and the search index, and wires the services.
//! The journal server and an app that embeds the core build it the same way (ADR 029).
//!
//! The core runs no tasks. The app around it calls `commit_pending` of the index, the export
//! queue and the folder scans at the times it chooses.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::application::atlas_service::AtlasService;
use crate::application::combined_search::CombinedSearchService;
use crate::application::config_service::ConfigService;
use crate::application::confirm_service::ConfirmService;
use crate::application::extension_functions::ExtensionFunctionService;
use crate::application::extension_runtime::ExtensionRuntime;
use crate::application::extension_service::ExtensionService;
use crate::application::flat_markdown_splitter::FlatMarkdownSplitter;
use crate::application::item_service::ItemService;
use crate::application::journal_service::JournalService;
use crate::application::markdown_parser::MarkdownParser;
use crate::application::page_service::PageService;
use crate::application::startup;
use crate::clock::Clock;
use crate::domain::id_generator::IdGenerator;
use crate::domain::search::SearchIndex;
use crate::dto::ItemDtoAssembler;
use crate::events::{ClientMessages, EventSink};
use crate::infrastructure::export::definitions_file::DefinitionsFileService;
use crate::infrastructure::export::export_queue_repository::ExportQueueRepository;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::export::file_sync_state::FileSyncStateRepository;
use crate::infrastructure::export::import_service::MarkdownImportService;
use crate::infrastructure::export::item_merger::ItemMerger;
use crate::infrastructure::export::outline_parser::OutlineMarkdownParser;
use crate::infrastructure::export::page_matcher::PageMatcher;
use crate::infrastructure::export::reconcile_service::{
    MarkdownReconcileService, MissingFilesPrompt,
};
use crate::infrastructure::persistence::database_initializer;
use crate::infrastructure::persistence::{Db, ItemRepository, PageRepository, PropertyRepository};
use crate::infrastructure::search::TantivySearchService;
use crate::journal::{FolderWatch, Journal, JournalServices};

/// A person may step away before answering a question of an extension, so the wait is long.
const CONFIRM_WAIT: Duration = Duration::from_secs(60);

/// Where the core keeps its data, and how it treats the markdown folder.
#[derive(Debug, Clone)]
pub struct CoreSettings {
    pub db_path: PathBuf,
    pub index_dir: PathBuf,
    pub config_dir: PathBuf,
    /// The folder a throw-away journal stays in, as that of a test: its markdown folder is
    /// never outside it (ADR 002). None for the journal of a user.
    pub confined_to: Option<PathBuf>,
    /// Whether files of the markdown folder are taken into the database
    pub import_enabled: bool,
    pub import_copy_wait: Duration,
    pub export_overwrite_wait: Duration,
}

/// What the app around the core does for it: carrying the messages to its clients, asking
/// the user about missing files, and watching the markdown folder.
pub struct CoreHost {
    /// The changes of pages and the questions of the extensions go here
    pub events: Arc<dyn EventSink>,
    pub missing_files: Arc<dyn MissingFilesPrompt>,
    pub folder_watch: Arc<dyn FolderWatch>,
    /// Dates what the journal writes. An app gives the clock of the system
    pub clock: Arc<dyn Clock>,
}

/// The services of the core, wired together.
pub struct JournalCore {
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
    pub item_service: Arc<ItemService>,
    pub page_service: Arc<PageService>,
    pub atlas: Arc<AtlasService>,
    pub journal_service: Arc<JournalService>,
    /// The entrance that REST, MCP and an embedding app share
    pub journal: Arc<Journal>,
    pub functions: Arc<ExtensionFunctionService>,
    pub extensions: Arc<ExtensionService>,
    pub confirm: Arc<ConfirmService>,
    pub dtos: ItemDtoAssembler,
    pub ids: Arc<IdGenerator>,
}

impl JournalCore {
    /// Opens the database and the search index and wires the services. It changes no page
    /// and writes no file: `start_up` does that.
    pub fn build(settings: &CoreSettings, host: CoreHost) -> anyhow::Result<JournalCore> {
        let messages = Arc::new(ClientMessages::new(host.events));
        let ids = Arc::new(IdGenerator::new(Arc::clone(&host.clock)));
        let db = Arc::new(Db::open(&settings.db_path, host.clock)?);
        database_initializer::initialize(&db, &ids)?;

        let pages = PageRepository::new(Arc::clone(&db));
        let items = ItemRepository::new(Arc::clone(&db));
        let properties = PropertyRepository::new(Arc::clone(&db));
        let export_queue = ExportQueueRepository::new(Arc::clone(&db));
        let file_sync_state = FileSyncStateRepository::new(Arc::clone(&db));

        let tantivy = Arc::new(TantivySearchService::open(&settings.index_dir)?);
        let search: Arc<dyn SearchIndex> = tantivy.clone();
        let config = Arc::new(match &settings.confined_to {
            Some(root) => ConfigService::confined(settings.config_dir.clone(), root.clone()),
            None => ConfigService::new(settings.config_dir.clone()),
        });
        let parser = MarkdownParser;
        let splitter = FlatMarkdownSplitter::new(parser.clone());

        let export = Arc::new(MarkdownExportService::new(
            Arc::clone(&config),
            export_queue.clone(),
            file_sync_state.clone(),
            pages.clone(),
            items.clone(),
            properties.clone(),
            settings.export_overwrite_wait,
        ));
        let atlas = Arc::new(AtlasService::new(
            pages.clone(),
            items.clone(),
            properties.clone(),
            Arc::clone(&search),
            Arc::clone(&export),
            Arc::clone(&ids),
            messages.clone(),
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
        let definitions_file = Arc::new(DefinitionsFileService::new(
            properties.clone(),
            Arc::clone(&export),
        ));
        let import = Arc::new(MarkdownImportService::new(
            OutlineMarkdownParser::new(splitter.clone()),
            page_matcher,
            item_merger,
            file_sync_state.clone(),
            Arc::clone(&export),
            messages.clone(),
            Arc::clone(&page_service),
            Arc::clone(&definitions_file),
            settings.import_copy_wait,
        ));
        let reconcile = Arc::new(MarkdownReconcileService::new(
            settings.import_enabled,
            Arc::clone(&export),
            Arc::clone(&import),
            file_sync_state.clone(),
            pages.clone(),
            host.missing_files,
            Arc::clone(&search),
        ));
        let functions = Arc::new(ExtensionFunctionService::load(Arc::clone(&config)));
        let confirm = Arc::new(ConfirmService::new(messages.clone(), CONFIRM_WAIT));
        let extensions = Arc::new(ExtensionService::new(
            ExtensionRuntime::new(Arc::new(splitter.clone()), Arc::clone(&ids))
                .with_confirmer(confirm.clone()),
            Arc::clone(&functions),
            Arc::clone(&item_service),
            Arc::clone(&page_service),
            messages.clone(),
        ));
        let dtos = ItemDtoAssembler::new(properties.clone(), items.clone());
        let journal = Arc::new(Journal::new(
            JournalServices {
                combined_search,
                page_service: Arc::clone(&page_service),
                item_service: Arc::clone(&item_service),
                journal_service: Arc::clone(&journal_service),
                atlas: Arc::clone(&atlas),
                extensions: Arc::clone(&extensions),
                config: Arc::clone(&config),
                export: Arc::clone(&export),
                definitions_file: Arc::clone(&definitions_file),
                folder_watch: host.folder_watch,
                pages: pages.clone(),
                items: items.clone(),
                properties: properties.clone(),
                splitter: splitter.clone(),
                dtos: dtos.clone(),
                ids: Arc::clone(&ids),
            },
            messages,
        ));

        Ok(JournalCore {
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
        })
    }

    /// The startup steps, in the order of the Java server: the pages go to a markdown
    /// directory that is empty, the search index is rebuilt if it is empty, the tag pages
    /// are brought up to date, and the atlas pages of the old form move to the plain form.
    pub fn start_up(&self) {
        let dir = self.config.markdown_dir();
        if !dir.trim().is_empty() {
            self.export.export_all_if_empty(&dir);
        }
        startup::rebuild_search_index(&self.pages, &self.items, &self.search);
        // After the rebuild, which skips an index that is not empty: the new pages are indexed
        // one by one as they are created
        startup::ensure_tag_pages(&self.page_service);
        startup::move_atlases_to_plain_form(&self.atlas);
    }
}
