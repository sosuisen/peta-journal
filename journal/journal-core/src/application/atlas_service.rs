//! The atlas pages: the server writes the whole outline of each `index_atlas` page, as
//! `domain::atlas` lays it out. It writes it again whenever the pages of a directory may have
//! changed, and on each change the atlas asks for.

use std::collections::BTreeMap;
use std::sync::Arc;

use hmtid::Hmtid;
use parking_lot::Mutex;
use rusqlite::Connection;

use crate::domain::atlas::{self, Outline, BOUNDS_KEY, PAGE_KEY, SIDE_KEY, TYPE_KEY};
use crate::domain::atlas_migration;
use crate::domain::bounds::Bounds;
use crate::domain::fractional_index;
use crate::domain::id_generator::IdGenerator;
use crate::domain::item_body::{self, ItemBody};
use crate::domain::model::{Item, ItemPatch, Page};
use crate::domain::search::SearchIndex;
use crate::error::AppError;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::export::path_mapper;
use crate::infrastructure::persistence::item_repository::parked_idx;
use crate::infrastructure::persistence::property_repository::ScalarProps;
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

const ITEM_TARGET: &str = "item";

/// The atlas keys an item can have; the server keeps them as the outline says.
const ITEM_KEYS: [&str; 4] = [PAGE_KEY, TYPE_KEY, SIDE_KEY, BOUNDS_KEY];

/// Tells the clients that the server changed an atlas page by itself.
pub trait AtlasEvents: Send + Sync {
    fn atlas_changed(&self, atlas_id: &Hmtid);
}

/// What a write of an atlas does outside the database, once its transaction is committed:
/// the search index, the file and the clients.
#[derive(Default)]
struct AfterCommit {
    removed_item_ids: Vec<String>,
    /// The items of each atlas written, with the title to index them under.
    indexed: Vec<(Vec<Item>, String)>,
    written: Vec<Hmtid>,
}

pub struct AtlasService {
    pages: PageRepository,
    items: ItemRepository,
    properties: PropertyRepository,
    search: Arc<dyn SearchIndex>,
    export: Arc<MarkdownExportService>,
    ids: Arc<IdGenerator>,
    events: Arc<dyn AtlasEvents>,
    /// Two writes of one atlas must not start from the same outline.
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
        ids: Arc<IdGenerator>,
        events: Arc<dyn AtlasEvents>,
    ) -> AtlasService {
        AtlasService {
            pages,
            items,
            properties,
            search,
            export,
            ids,
            events,
            lock: Mutex::new(()),
        }
    }

    /// Moves every atlas page of the form the server wrote to the form of a plain page
    /// (ADR 053).
    pub fn move_to_plain_form(&self) -> Result<(), AppError> {
        let _guard = self.lock.lock();
        for atlas in self.pages.find_all()?.into_iter().filter(Page::is_atlas) {
            let loaded = self.load(atlas)?;
            let Some(migration) = atlas_migration::migration(&loaded.items, &loaded.props) else {
                continue;
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
        }
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

    /// Writes the outline of every atlas page again where the pages of its directory changed.
    /// There are few atlas pages, so all of them are looked at: a move of a folder changes
    /// the paths of many pages at once.
    pub fn sync_all(&self) -> Result<(), AppError> {
        let _guard = self.lock.lock();
        for atlas in self.pages.find_all()?.into_iter().filter(Page::is_atlas) {
            let loaded = self.load(atlas)?;
            let mut outline = Outline::read(&loaded.items, &loaded.props);
            self.fit(&mut outline, &loaded.atlas)?;
            self.write(&loaded, &outline, auto_modified(&loaded.atlas))?;
        }
        Ok(())
    }

    /// Takes the card of a deleted page off every atlas. A page that is only not in the
    /// directory keeps its card, as `Outline::fit` says.
    pub fn forget_page(&self, page_id: &Hmtid) -> Result<(), AppError> {
        let _guard = self.lock.lock();
        for atlas in self.pages.find_all()?.into_iter().filter(Page::is_atlas) {
            let loaded = self.load(atlas)?;
            let mut outline = Outline::read(&loaded.items, &loaded.props);
            if outline.forget(page_id.as_str()) {
                self.fit(&mut outline, &loaded.atlas)?;
                self.write(&loaded, &outline, auto_modified(&loaded.atlas))?;
            }
        }
        Ok(())
    }

    /// Adds a link card to the atlas page. Returns the items of the page.
    pub fn add_link(
        &self,
        atlas_id: &Hmtid,
        target: &str,
        bounds: Bounds,
    ) -> Result<Vec<Item>, AppError> {
        let target = link_target(target)?;
        let id = self.ids.generate_item_id();
        self.edit(atlas_id, |outline| {
            outline.add_link(id, &target, bounds.to_string());
            Ok(())
        })
    }

    /// Links a link card to another page. Returns the items of the page.
    pub fn set_link(
        &self,
        atlas_id: &Hmtid,
        item_id: &Hmtid,
        target: &str,
    ) -> Result<Vec<Item>, AppError> {
        let target = link_target(target)?;
        self.edit(atlas_id, |outline| {
            found(outline.set_link(item_id, &target), item_id)
        })
    }

    /// Puts cards into the group of the target card, or into a new group with it. A card is
    /// named by its own id, or a page card by the id of its page. Returns the items of the page.
    pub fn group(
        &self,
        atlas_id: &Hmtid,
        cards: &[String],
        target: &str,
    ) -> Result<Vec<Item>, AppError> {
        let new_id = self.ids.generate_item_id();
        self.edit(atlas_id, |outline| {
            let target = outline
                .card_id(target)
                .ok_or_else(|| AppError::bad_request(format!("No card: {target}")))?;
            let moving: Vec<Hmtid> = cards.iter().filter_map(|c| outline.card_id(c)).collect();
            outline.group(&moving, &target, || new_id);
            Ok(())
        })
    }

    /// Takes cards out of their groups. Returns the items of the page.
    pub fn ungroup(&self, atlas_id: &Hmtid, cards: &[String]) -> Result<Vec<Item>, AppError> {
        self.edit(atlas_id, |outline| {
            let ids: Vec<Hmtid> = cards.iter().filter_map(|c| outline.card_id(c)).collect();
            outline.ungroup(&ids);
            Ok(())
        })
    }

    /// Gives a group a title along a side of it. Returns the items of the page.
    pub fn set_group_title(
        &self,
        atlas_id: &Hmtid,
        group_id: &Hmtid,
        side: &str,
        title: &str,
    ) -> Result<Vec<Item>, AppError> {
        self.edit(atlas_id, |outline| {
            found(outline.set_group_title(group_id, side, title), group_id)
        })
    }

    /// Adds a callout. An empty text adds nothing. Returns the items of the page.
    pub fn add_callout(
        &self,
        atlas_id: &Hmtid,
        bounds: Bounds,
        side: &str,
        text: &str,
    ) -> Result<Vec<Item>, AppError> {
        let id = self.ids.generate_item_id();
        self.edit(atlas_id, |outline| {
            if !text.trim().is_empty() {
                outline.add_callout(id, bounds.to_string(), side, text);
            }
            Ok(())
        })
    }

    /// Changes the text and the side of a callout; an empty text takes it away. Returns the
    /// items of the page.
    pub fn set_callout(
        &self,
        atlas_id: &Hmtid,
        item_id: &Hmtid,
        side: &str,
        text: &str,
    ) -> Result<Vec<Item>, AppError> {
        self.edit(atlas_id, |outline| {
            found(outline.set_callout(item_id, side, text), item_id)
        })
    }

    /// Takes a link card or a callout away. Returns the items of the page.
    pub fn remove(&self, atlas_id: &Hmtid, item_id: &Hmtid) -> Result<Vec<Item>, AppError> {
        self.edit(atlas_id, |outline| found(outline.remove(item_id), item_id))
    }

    /// Changes the outline of one atlas page and writes it. Returns the items of the page.
    fn edit(
        &self,
        atlas_id: &Hmtid,
        change: impl FnOnce(&mut Outline) -> Result<(), AppError>,
    ) -> Result<Vec<Item>, AppError> {
        let _guard = self.lock.lock();
        let atlas = self
            .pages
            .find_by_id(atlas_id)?
            .filter(Page::is_atlas)
            .ok_or_else(|| AppError::bad_request(format!("Not an atlas page: {atlas_id}")))?;
        let loaded = self.load(atlas)?;
        let mut outline = Outline::read(&loaded.items, &loaded.props);
        self.fit(&mut outline, &loaded.atlas)?;
        change(&mut outline)?;
        self.fit(&mut outline, &loaded.atlas)?;
        self.write(&loaded, &outline, self.items.db().now_millis())?;
        self.items.find_by_page_id(atlas_id)
    }

    /// Makes the outline fit the pages of the directory of the atlas. A card whose page is
    /// in another directory goes; one whose page is not known waits for it.
    fn fit(&self, outline: &mut Outline, atlas: &Page) -> Result<(), AppError> {
        let pages = self.dir_pages(atlas)?;
        outline.fit(&atlas.id, &pages, |page_id| {
            let page = Hmtid::parse(page_id)
                .ok()
                .and_then(|id| self.pages.find_by_id(&id).ok().flatten());
            page.is_some_and(|p| !atlas::is_in_dir(&p, &atlas.dir_path))
        });
        Ok(())
    }

    /// The pages the atlas shows: the pages and the folders of its directory, as the index
    /// list shows them. The root lists no journal.
    fn dir_pages(&self, atlas: &Page) -> Result<Vec<Page>, AppError> {
        let Some(index) =
            self.pages
                .find_by_type_dir_path_file_name("doc", &atlas.dir_path, "index")?
        else {
            return Ok(Vec::new());
        };
        Ok(self
            .pages
            .find_by_parent_id(&index.id)?
            .into_iter()
            // By the path too, as a card of a page elsewhere goes: the two must agree, or two
            // devices would write the card and take it away in turn
            .filter(|p| atlas::is_in_dir(p, &atlas.dir_path))
            .collect())
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

    /// Writes the outline over the items of the atlas page. Nothing is written when they are
    /// the same already; otherwise the page takes the modified date, its file is written and
    /// the clients are told.
    fn write(&self, loaded: &Loaded, outline: &Outline, modified: i64) -> Result<(), AppError> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        self.write_in(&tx, loaded, outline, modified, &mut effects)?;
        tx.commit()?;
        self.finish(effects);
        Ok(())
    }

    /// Writes the outline over the items of the atlas page, in the transaction. What the
    /// write does outside the database goes into `effects`.
    fn write_in(
        &self,
        tx: &Connection,
        loaded: &Loaded,
        outline: &Outline,
        modified: i64,
        effects: &mut AfterCommit,
    ) -> Result<(), AppError> {
        let rows = outline.rows();
        let before: BTreeMap<&str, &Item> =
            loaded.items.iter().map(|i| (i.id.as_str(), i)).collect();
        let kept: Vec<&str> = rows.iter().map(|r| r.node.id.as_str()).collect();
        let removed: Vec<String> = loaded
            .items
            .iter()
            .filter(|i| !kept.contains(&i.id.as_str()))
            .map(|i| i.id.to_string())
            .collect();
        let arranged = rows
            .iter()
            .map(|r| (r.node.id.as_str(), r.level))
            .ne(loaded.items.iter().map(|i| (i.id.as_str(), i.level)));
        let mut changed = arranged || !removed.is_empty();
        let now = self.items.db().now_millis();
        let atlas = &loaded.atlas;

        for id in &removed {
            if let Ok(id) = Hmtid::parse(id) {
                ItemRepository::delete_in(tx, &id)?;
            }
        }
        PropertyRepository::delete_all_for_targets_in(tx, ITEM_TARGET, &removed)?;
        effects.removed_item_ids.extend(removed);

        let mut saved = Vec::with_capacity(rows.len());
        for row in &rows {
            let node = &row.node;
            let item = match before.get(node.id.as_str()) {
                Some(item) if item.body == node.body => (*item).clone(),
                found => {
                    changed = true;
                    item_body::check_closed(&node.body).map_err(AppError::bad_request)?;
                    let body = ItemBody {
                        text: node.body.clone(),
                        task: false,
                    };
                    let item = Item {
                        modified_date: now,
                        body: node.body.clone(),
                        types: body.text_types(),
                        ..found
                            .map_or_else(|| self.new_item(atlas, &node.id, now), |i| (*i).clone())
                    };
                    ItemRepository::save_in(tx, &item)?;
                    item
                }
            };
            changed |= Self::write_properties_in(tx, node, loaded.props.get(node.id.as_str()))?;
            saved.push(item);
        }
        if !changed {
            return Ok(());
        }
        if arranged {
            let indices = fractional_index::generate_indices(rows.len());
            let placements: Vec<(Hmtid, String, i32)> = rows
                .iter()
                .zip(indices)
                .map(|(r, idx)| (r.node.id.clone(), idx, r.level))
                .collect();
            ItemRepository::arrange_in(tx, &atlas.id, &placements)?;
            ItemRepository::recalculate_parent_ids_in(tx, &atlas.id)?;
        }
        PageRepository::save_in(
            tx,
            &Page {
                modified_date: modified,
                ..atlas.clone()
            },
        )?;
        effects.indexed.push((saved, atlas.title()));
        effects.written.push(atlas.id.clone());
        Ok(())
    }

    /// The work that lives outside the database, done once the commit is through.
    fn finish(&self, effects: AfterCommit) {
        self.search.remove_items(&effects.removed_item_ids);
        for (items, title) in &effects.indexed {
            self.search.index_items(items, title);
        }
        for atlas_id in &effects.written {
            self.export.queue_export(atlas_id);
            self.events.atlas_changed(atlas_id);
        }
    }

    /// Gives an item the atlas properties of its node. True when one changed.
    fn write_properties_in(
        tx: &Connection,
        node: &atlas::Node,
        stored: Option<&BTreeMap<String, String>>,
    ) -> Result<bool, AppError> {
        let wanted = node.properties();
        let id = node.id.as_str();
        let mut changed = false;
        for key in ITEM_KEYS {
            let have = stored.and_then(|s| s.get(key));
            match wanted.get(key) {
                Some(value) if have != Some(value) => {
                    PropertyRepository::write_scalar_in(tx, ITEM_TARGET, id, key, value)?;
                    changed = true;
                }
                None if have.is_some() => {
                    PropertyRepository::delete_property_in(tx, ITEM_TARGET, id, key)?;
                    changed = true;
                }
                _ => {}
            }
        }
        Ok(changed)
    }

    fn new_item(&self, atlas: &Page, id: &Hmtid, now: i64) -> Item {
        Item {
            id: id.clone(),
            page_id: atlas.id.clone(),
            parent_item_id: None,
            dir_path: atlas.dir_path.clone(),
            name: atlas.name.clone(),
            item_type: atlas.page_type.clone(),
            created_date: now,
            modified_date: now,
            idx: parked_idx(id),
            level: 1,
            body: String::new(),
            collapsed: false,
            types: Vec::new(),
            transclude_item_id: None,
        }
    }
}

/// The modified date of an atlas the server wrote by itself: one millisecond after the last.
/// Another device that writes the same outline from the same pages then writes the same
/// file. A file of the same date and content is no change there, and a later one wins over
/// the outline it had. A change made in the app takes the time of the change instead.
fn auto_modified(atlas: &Page) -> i64 {
    atlas.modified_date + 1
}

/// The page a link card is to link to, without the brackets of a link around it, as the
/// card adds its own.
fn link_target(target: &str) -> Result<String, AppError> {
    let target = target.trim();
    let target = atlas::link_target(target).unwrap_or(target).trim();
    if target.is_empty() || target.contains("[[") || target.contains("]]") {
        return Err(AppError::bad_request(format!(
            "Not a link target: {target}"
        )));
    }
    Ok(target.to_string())
}

fn found(found: bool, id: &Hmtid) -> Result<(), AppError> {
    if found {
        Ok(())
    } else {
        Err(AppError::bad_request(format!("Not on the atlas: {id}")))
    }
}
