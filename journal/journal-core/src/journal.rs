//! The entrance of journal-core (ADR 029). REST, MCP and an app that embeds the core all
//! call an operation here, so the three give the same answer.
//!
//! An operation does all of the work of one request: it checks the values, calls the
//! services, runs the extensions, tells the clients which pages changed, and builds the
//! answer. The answers are the JSON shapes of the REST API.
//!
//! The operations come here one area at a time. The search, the pages, the items, the
//! journals, the atlas pages and the settings are here. The other areas are still in the
//! handlers of the journal server.

mod assets;
mod atlas;
mod config;
mod items;
mod journals;
mod pages;
mod search;

pub use assets::{SavedAsset, ASSET_LIMIT_BYTES};
pub use atlas::{AtlasCalloutInput, AtlasGroupInput, AtlasLinkInput, AtlasTitleInput};
pub use config::{
    AddPropertyDefinitionInput, ConfigAnswer, FolderWatch, PropertyDefinitionDto,
    PropertyDefinitionsAnswer, UpdateConfigInput,
};
pub use items::{
    BatchAnswer, BatchInput, BatchOpInput, CreateItemInput, CursorDto, DeletedItemsAnswer,
    FlatMarkdownAnswer, FlatMarkdownInput, ItemAnswer, ItemIdsInput, ItemsAnswer, MergeItemsInput,
    MoveItemsInput, ParsedFlatItem, ParsedItemsAnswer, PasteAnswer, PasteInput, PropertyInput,
    ShiftLevelsInput, SplitAnswer, SplitInput, StashAnswer, StashInput, TaskStateInput,
    TranscludedItemsAnswer, TransclusionInput, TreeMarkdownAnswer, TreeMarkdownInput,
    UpdateItemInput,
};
pub use journals::{
    JournalAnswer, MostRecentJournalAnswer, MoveToJournalInput, MovedToJournalAnswer,
};
pub use pages::{
    BacklinksAnswer, DocAnswer, FindPageQuery, FoundPageAnswer, NarrowPage, PageAnswer,
    PageWithItems, PagesAnswer,
};
pub use search::SearchAnswer;

use std::sync::Arc;

use hmtid::Hmtid;

use crate::application::atlas_service::AtlasService;
use crate::application::combined_search::CombinedSearchService;
use crate::application::config_service::ConfigService;
use crate::application::extension_service::ExtensionService;
use crate::application::flat_markdown_splitter::FlatMarkdownSplitter;
use crate::application::item_service::ItemService;
use crate::application::journal_service::JournalService;
use crate::application::page_service::PageService;
use crate::domain::client_id::ClientId;
use crate::domain::id_generator::IdGenerator;
use crate::dto::ItemDtoAssembler;
use crate::error::AppError;
use crate::infrastructure::export::definitions_file::DefinitionsFileService;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

/// Who made a change. The clients that show pages are told with it (ADR-015).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangedBy {
    /// One of the clients. It knows its own ID and does not read the page again.
    Client(ClientId),
    /// An MCP tool. None of the clients made the change, so all of them read the page again.
    Mcp,
}

/// Tells the clients that a page changed. A client that shows the page reads it again.
pub trait JournalEvents: Send + Sync {
    fn page_changed(&self, page_id: &Hmtid, by: &ChangedBy);
    /// The user changed the property definitions (ADR 046).
    fn definitions_changed(&self);
}

/// What the operations work with.
pub struct JournalServices {
    pub combined_search: Arc<CombinedSearchService>,
    pub page_service: Arc<PageService>,
    pub item_service: Arc<ItemService>,
    pub journal_service: Arc<JournalService>,
    pub atlas: Arc<AtlasService>,
    pub extensions: Arc<ExtensionService>,
    pub config: Arc<ConfigService>,
    pub export: Arc<MarkdownExportService>,
    pub definitions_file: Arc<DefinitionsFileService>,
    pub folder_watch: Arc<dyn FolderWatch>,
    pub pages: PageRepository,
    pub items: ItemRepository,
    pub properties: PropertyRepository,
    pub splitter: FlatMarkdownSplitter,
    pub dtos: ItemDtoAssembler,
    pub ids: Arc<IdGenerator>,
}

pub struct Journal {
    services: JournalServices,
    events: Arc<dyn JournalEvents>,
}

impl Journal {
    pub fn new(services: JournalServices, events: Arc<dyn JournalEvents>) -> Journal {
        Journal { services, events }
    }

    /// Tells the clients of each page, in the order given.
    fn tell<'a>(&self, page_ids: impl IntoIterator<Item = &'a Hmtid>, by: &ChangedBy) {
        for page_id in page_ids {
            self.events.page_changed(page_id, by);
        }
    }

    /// The user changed the definitions: the other devices get them in the file, and the
    /// clients read them again (ADR 046).
    fn definitions_changed(&self) -> Result<(), AppError> {
        self.services.definitions_file.write_file()?;
        self.events.definitions_changed();
        Ok(())
    }
}
