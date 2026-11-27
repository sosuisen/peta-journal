//! The atlas pages. An `index_atlas` page is a plain page (ADR 053): the server writes nothing
//! into it by itself. What is left here moves the atlas pages that the server wrote before to
//! the form of a plain page.

use std::sync::Arc;

use hmtid::Hmtid;
use parking_lot::Mutex;

use crate::domain::atlas_migration;
use crate::domain::model::{Item, ItemPatch, Page};
use crate::domain::search::SearchIndex;
use crate::error::AppError;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::export::path_mapper;
use crate::infrastructure::persistence::property_repository::ScalarProps;
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

const ITEM_TARGET: &str = "item";

/// Tells the clients that the server changed an atlas page by itself.
pub trait AtlasEvents: Send + Sync {
    fn atlas_changed(&self, atlas_id: &Hmtid);
}

pub struct AtlasService {
    pages: PageRepository,
    items: ItemRepository,
    properties: PropertyRepository,
    search: Arc<dyn SearchIndex>,
    export: Arc<MarkdownExportService>,
    events: Arc<dyn AtlasEvents>,
    /// Two moves of one atlas must not start from the same items.
    lock: Mutex<()>,
}

/// An atlas page as it is now: its items in order and their properties.
struct Loaded {
    atlas: Page,
    items: Vec<Item>,
    props: ScalarProps,
}

impl AtlasService {
    pub fn new(
        pages: PageRepository,
        items: ItemRepository,
        properties: PropertyRepository,
        search: Arc<dyn SearchIndex>,
        export: Arc<MarkdownExportService>,
        events: Arc<dyn AtlasEvents>,
    ) -> AtlasService {
        AtlasService {
            pages,
            items,
            properties,
            search,
            export,
            events,
            lock: Mutex::new(()),
        }
    }

    /// Moves every atlas page of the form the server wrote to the form of a plain page
    /// (ADR 053).
    pub fn move_to_plain_form(&self) -> Result<(), AppError> {
        let _guard = self.lock.lock();
        for atlas in self.pages.find_all()?.into_iter().filter(Page::is_atlas) {
            self.move_locked(atlas)?;
        }
        Ok(())
    }

    /// Moves the atlas page to the form of a plain page, if it has the form the server wrote
    /// (ADR 053). A device that still runs an old version may bring a file of that form.
    pub fn move_page_to_plain_form(&self, atlas: &Page) -> Result<(), AppError> {
        let _guard = self.lock.lock();
        self.move_locked(atlas.clone())
    }

    /// Moves the atlas while the lock is held
    fn move_locked(&self, atlas: Page) -> Result<(), AppError> {
        let loaded = self.load(atlas)?;
        let Some(migration) = atlas_migration::migration(&loaded.items, &loaded.props) else {
            return Ok(());
        };
        self.back_up_file(&loaded.atlas);
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let now = self.items.db().now_millis();
        for (id, key) in &migration.dropped_props {
            PropertyRepository::delete_property_in(&tx, ITEM_TARGET, id.as_str(), key)?;
        }
        let removed: Vec<String> = migration.removed.iter().map(Hmtid::to_string).collect();
        for id in &migration.removed {
            ItemRepository::delete_in(&tx, id)?;
            ItemRepository::record_deleted_in(&tx, id, now)?;
        }
        PropertyRepository::delete_all_for_targets_in(&tx, ITEM_TARGET, &removed)?;
        for (id, level) in &migration.levels {
            let patch = ItemPatch::new(None, None, None, Some(*level), None, now)
                .map_err(AppError::bad_request)?;
            ItemRepository::patch_in(&tx, id, &patch)?;
        }
        ItemRepository::recalculate_parent_ids_in(&tx, &loaded.atlas.id)?;
        PageRepository::save_in(
            &tx,
            &Page {
                modified_date: now,
                ..loaded.atlas.clone()
            },
        )?;
        tx.commit()?;
        self.search.remove_items(&removed);
        self.export.queue_export(&loaded.atlas.id);
        self.events.atlas_changed(&loaded.atlas.id);
        Ok(())
    }

    /// Keeps the file of an atlas next to it, as it is before the atlas moves to the plain
    /// form. A device that still runs an old version may write the old form over the new
    /// one; the atlas can be put back from this file then.
    fn back_up_file(&self, atlas: &Page) {
        let Some(root) = self.export.markdown_root() else {
            return;
        };
        let file = root.join(path_mapper::to_rel_path_of(atlas));
        let Ok(content) = std::fs::read_to_string(&file) else {
            return;
        };
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let backup = file.with_file_name(path_mapper::conflict_file_name(&atlas.name, &ts));
        if let Err(e) = std::fs::write(&backup, content) {
            tracing::error!("Could not write the backup {}: {e}", backup.display());
        }
    }

    fn load(&self, atlas: Page) -> Result<Loaded, AppError> {
        let items = self.items.find_by_page_id(&atlas.id)?;
        let ids: Vec<String> = items.iter().map(|i| i.id.to_string()).collect();
        let props = self.properties.find_scalar_by_targets(ITEM_TARGET, &ids)?;
        Ok(Loaded {
            atlas,
            items,
            props,
        })
    }
}
