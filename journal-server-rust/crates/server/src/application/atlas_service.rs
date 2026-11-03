//! The atlas pages: the server writes the whole outline of each `index_atlas` page, as
//! `domain::atlas` lays it out. It writes it again whenever the pages of a directory may have
//! changed, and on each change the atlas asks for.

use std::collections::BTreeMap;
use std::sync::Arc;

use hmtid::Hmtid;
use parking_lot::Mutex;
use rusqlite::Connection;

use crate::domain::atlas::{self, Outline, BOUNDS_KEY, PAGE_KEY, SIDE_KEY, TYPE_KEY};
use crate::domain::fractional_index;
use crate::domain::id_generator::IdGenerator;
use crate::domain::item_body::ItemBody;
use crate::domain::model::{Item, Page};
use crate::domain::search::SearchIndex;
use crate::error::{AppError, AppResult};
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::persistence::db::now_millis;
use crate::infrastructure::persistence::item_repository::parked_idx;
use crate::infrastructure::persistence::property_repository::{normalize_bounds, ScalarProps};
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

use super::markdown_parser::MarkdownParser;

const ITEM_TARGET: &str = "item";
const PAGE_TARGET: &str = "page";

/// The atlas keys an item can have; the server keeps them as the outline says.
const ITEM_KEYS: [&str; 4] = [PAGE_KEY, TYPE_KEY, SIDE_KEY, BOUNDS_KEY];

/// The keys of the atlas of old, on the pages: the place of a card on the page itself, and
/// the groups, their titles and the callouts on the atlas page.
const OLD_GROUP_PREFIX: &str = "atlas_group_";
const OLD_TITLE_PREFIX: &str = "atlas_title_";
const OLD_CALLOUT_PREFIX: &str = "atlas_callout_";

/// The height of a card or a callout at scale 1.
const CARD_HEIGHT: f64 = 40.0;
/// The smallest a card or a callout can be scaled to.
const MIN_SCALE: f64 = 0.5;

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
    parser: MarkdownParser,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pages: PageRepository,
        items: ItemRepository,
        properties: PropertyRepository,
        parser: MarkdownParser,
        search: Arc<dyn SearchIndex>,
        export: Arc<MarkdownExportService>,
        ids: Arc<IdGenerator>,
        events: Arc<dyn AtlasEvents>,
    ) -> AtlasService {
        AtlasService {
            pages,
            items,
            properties,
            parser,
            search,
            export,
            ids,
            events,
            lock: Mutex::new(()),
        }
    }

    /// Writes the outline of every atlas page again where the pages of its directory changed.
    /// There are few atlas pages, so all of them are looked at: a move of a folder changes
    /// the paths of many pages at once.
    pub fn sync_all(&self) -> AppResult<()> {
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
    pub fn forget_page(&self, page_id: &Hmtid) -> AppResult<()> {
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
    pub fn add_link(&self, atlas_id: &Hmtid, target: &str, bounds: &str) -> AppResult<Vec<Item>> {
        let target = link_target(target)?;
        let bounds = normalize_bounds(bounds)?;
        let id = self.ids.generate_item_id();
        self.edit(atlas_id, |outline| {
            outline.add_link(id, &target, bounds);
            Ok(())
        })
    }

    /// Links a link card to another page. Returns the items of the page.
    pub fn set_link(
        &self,
        atlas_id: &Hmtid,
        item_id: &Hmtid,
        target: &str,
    ) -> AppResult<Vec<Item>> {
        let target = link_target(target)?;
        self.edit(atlas_id, |outline| {
            found(outline.set_link(item_id, &target), item_id)
        })
    }

    /// Puts cards into the group of the target card, or into a new group with it. A card is
    /// named by its own id, or a page card by the id of its page. Returns the items of the page.
    pub fn group(&self, atlas_id: &Hmtid, cards: &[String], target: &str) -> AppResult<Vec<Item>> {
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
    pub fn ungroup(&self, atlas_id: &Hmtid, cards: &[String]) -> AppResult<Vec<Item>> {
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
    ) -> AppResult<Vec<Item>> {
        self.edit(atlas_id, |outline| {
            found(outline.set_group_title(group_id, side, title), group_id)
        })
    }

    /// Adds a callout. An empty text adds nothing. Returns the items of the page.
    pub fn add_callout(
        &self,
        atlas_id: &Hmtid,
        bounds: &str,
        side: &str,
        text: &str,
    ) -> AppResult<Vec<Item>> {
        let bounds = normalize_bounds(bounds)?;
        let id = self.ids.generate_item_id();
        self.edit(atlas_id, |outline| {
            if !text.trim().is_empty() {
                outline.add_callout(id, bounds, side, text);
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
    ) -> AppResult<Vec<Item>> {
        self.edit(atlas_id, |outline| {
            found(outline.set_callout(item_id, side, text), item_id)
        })
    }

    /// Takes a link card or a callout away. Returns the items of the page.
    pub fn remove(&self, atlas_id: &Hmtid, item_id: &Hmtid) -> AppResult<Vec<Item>> {
        self.edit(atlas_id, |outline| found(outline.remove(item_id), item_id))
    }

    /// Moves the atlas of old into the outlines: the place of each card from its page, and the
    /// groups, their titles and the callouts from the atlas page. The old keys go, and so do
    /// their definitions. A page whose place moved keeps its modified date: only how its file
    /// is written changes.
    pub fn move_old_properties(&self) -> AppResult<usize> {
        let _guard = self.lock.lock();
        let all = self.pages.find_all()?;
        let ids: Vec<String> = all.iter().map(|p| p.id.to_string()).collect();
        let scalars = self.properties.find_scalar_by_targets(PAGE_TARGET, &ids)?;
        let lists = self.properties.find_list_by_targets(PAGE_TARGET, &ids)?;
        let is_old = |key: &str| {
            key == BOUNDS_KEY
                || [OLD_GROUP_PREFIX, OLD_TITLE_PREFIX, OLD_CALLOUT_PREFIX]
                    .iter()
                    .any(|prefix| key.starts_with(prefix))
        };
        let old_pages: Vec<&Page> = all
            .iter()
            .filter(|p| {
                let id = p.id.to_string();
                scalars
                    .get(&id)
                    .is_some_and(|s| s.keys().any(|k| is_old(k)))
                    || lists.get(&id).is_some_and(|l| l.keys().any(|k| is_old(k)))
            })
            .collect();
        if old_pages.is_empty() {
            return Ok(0);
        }
        let empty = BTreeMap::new();
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        for atlas in all.iter().filter(|p| p.is_atlas()) {
            let atlas_scalars = scalars.get(atlas.id.as_str()).unwrap_or(&empty);
            // Read outside the transaction: what it wrote so far is other atlases
            let pages = self.dir_pages(atlas)?;
            let loaded = self.load(atlas.clone())?;
            let mut outline = Outline::read(&loaded.items, &loaded.props);
            self.fit(&mut outline, atlas)?;
            for page in &pages {
                let bounds = scalars
                    .get(page.id.as_str())
                    .and_then(|s| s.get(BOUNDS_KEY));
                if let (Some(bounds), Some(card)) = (bounds, outline.card_id(page.id.as_str())) {
                    outline.set_bounds(&card, bounds.clone());
                }
            }
            let groups = lists
                .get(atlas.id.as_str())
                .into_iter()
                .flatten()
                .filter(|(key, _)| key.starts_with(OLD_GROUP_PREFIX));
            for (key, page_ids) in groups {
                let cards: Vec<Hmtid> = page_ids
                    .iter()
                    .filter_map(|id| outline.card_id(id))
                    .collect();
                let Some((target, moving)) = cards.split_first() else {
                    continue;
                };
                // From the old key, so another device moving the same atlas makes the same group
                let group_id = atlas::derived_id(&atlas.id, key);
                outline.group(moving, target, || group_id.clone());
                let title_key = key.replacen(OLD_GROUP_PREFIX, OLD_TITLE_PREFIX, 1);
                if let Some(value) = atlas_scalars.get(&title_key) {
                    let (side, title) = old_side_and_text(value);
                    outline.set_group_title(&group_id, side, title);
                }
            }
            let callouts = atlas_scalars
                .iter()
                .filter(|(key, _)| key.starts_with(OLD_CALLOUT_PREFIX));
            for (key, value) in callouts {
                if let Some((bounds, side, text)) = old_callout(value) {
                    outline.add_callout(atlas::derived_id(&atlas.id, key), bounds, side, text);
                }
            }
            // Again, for the parent of the callouts
            self.fit(&mut outline, atlas)?;
            self.write_in(
                &tx,
                &loaded,
                &outline,
                auto_modified(&loaded.atlas),
                &mut effects,
            )?;
        }
        for page in &old_pages {
            let id = page.id.to_string();
            let keys = scalars
                .get(&id)
                .into_iter()
                .flat_map(|s| s.keys())
                .chain(lists.get(&id).into_iter().flat_map(|l| l.keys()))
                .filter(|k| is_old(k));
            for key in keys {
                PropertyRepository::delete_property_in(&tx, PAGE_TARGET, &id, key)?;
            }
        }
        for prefix in [OLD_GROUP_PREFIX, OLD_TITLE_PREFIX, OLD_CALLOUT_PREFIX] {
            PropertyRepository::delete_unused_definitions_with_prefix_in(&tx, prefix)?;
        }
        tx.commit()?;
        self.finish(effects);
        for page in &old_pages {
            self.export.queue_export(&page.id);
        }
        Ok(old_pages.len())
    }

    /// Changes the outline of one atlas page and writes it. Returns the items of the page.
    fn edit(
        &self,
        atlas_id: &Hmtid,
        change: impl FnOnce(&mut Outline) -> AppResult<()>,
    ) -> AppResult<Vec<Item>> {
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
        self.write(&loaded, &outline, now_millis())?;
        self.items.find_by_page_id(atlas_id)
    }

    /// Makes the outline fit the pages of the directory of the atlas. A card whose page is
    /// in another directory goes; one whose page is not known waits for it.
    fn fit(&self, outline: &mut Outline, atlas: &Page) -> AppResult<()> {
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
    fn dir_pages(&self, atlas: &Page) -> AppResult<Vec<Page>> {
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

    fn load(&self, atlas: Page) -> AppResult<Loaded> {
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
    fn write(&self, loaded: &Loaded, outline: &Outline, modified: i64) -> AppResult<()> {
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
    ) -> AppResult<()> {
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
        let now = now_millis();
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
                    let body = ItemBody {
                        text: node.body.clone(),
                        task: false,
                    };
                    let item = Item {
                        modified_date: now,
                        body: node.body.clone(),
                        ast: self.parser.parse_to_ast(&node.body),
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
    ) -> AppResult<bool> {
        let wanted = node.properties();
        let id = node.id.as_str();
        let mut changed = false;
        for key in ITEM_KEYS {
            let have = stored.and_then(|s| s.get(key));
            match wanted.get(key) {
                Some(value) if have != Some(value) => {
                    // The atlas keys are defined when the database is made, and the
                    // definition gives the type; the type sent here is never used
                    PropertyRepository::upsert_scalar_in(tx, ITEM_TARGET, id, key, "text", value)?;
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
            ast: String::new(),
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
fn link_target(target: &str) -> AppResult<String> {
    let target = target.trim();
    let target = atlas::link_target(target).unwrap_or(target).trim();
    if target.is_empty() || target.contains("[[") || target.contains("]]") {
        return Err(AppError::bad_request(format!(
            "Not a link target: {target}"
        )));
    }
    Ok(target.to_string())
}

fn found(found: bool, id: &Hmtid) -> AppResult<()> {
    if found {
        Ok(())
    } else {
        Err(AppError::bad_request(format!("Not on the atlas: {id}")))
    }
}

/// The side and the text of a value of old, `side:text`. A value without a known side lies
/// along the top, all of it as the text.
fn old_side_and_text(value: &str) -> (&str, &str) {
    match value.split_once(':') {
        Some((side, text)) if atlas::SIDES.contains(&side) => (side, text),
        _ => (atlas::SIDES[0], value),
    }
}

/// A callout of old, `x,y,scale,side:text` or `x,y,side:text`, as its bounds, side and text.
fn old_callout(value: &str) -> Option<(String, &str, &str)> {
    let (head, text) = value.split_once(':')?;
    let parts: Vec<&str> = head.split(',').map(str::trim).collect();
    let (x, y, scale, side) = match parts[..] {
        [x, y, side] => (x, y, "1", side),
        [x, y, scale, side] => (x, y, scale, side),
        _ => return None,
    };
    let scale: f64 = scale
        .parse()
        .ok()
        .filter(|s: &f64| s.is_finite() && *s > 0.0)?;
    let height = (CARD_HEIGHT * scale.max(MIN_SCALE) * 100.0).round() / 100.0;
    let bounds = normalize_bounds(&format!("{x},{y},0,{height}")).ok()?;
    (atlas::SIDES.contains(&side) && !text.trim().is_empty()).then_some((bounds, side, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 旧形式のコールアウトは、倍率を高さに読み替える。
    #[test]
    fn a_callout_of_old_takes_its_scale_as_the_height() {
        assert_eq!(
            old_callout("120,-40,1.5,top:hello"),
            Some(("120,-40,0,60".to_string(), "top", "hello"))
        );
        assert_eq!(
            old_callout("1,2,left:a: b"),
            Some(("1,2,0,40".to_string(), "left", "a: b"))
        );
        assert_eq!(old_callout("1,2,middle:a"), None);
    }

    // 旧形式のタイトルは、辺の指定がなければ上辺に置かれる。
    #[test]
    fn a_title_of_old_without_a_side_lies_along_the_top() {
        assert_eq!(old_side_and_text("left:Tools"), ("left", "Tools"));
        assert_eq!(old_side_and_text("Tools: more"), ("top", "Tools: more"));
    }
}
