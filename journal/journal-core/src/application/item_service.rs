//! Item use cases.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use hmtid::Hmtid;
use rusqlite::{Connection, Transaction};

use crate::domain::fractional_index;
use crate::domain::hashtag;
use crate::domain::id_generator::IdGenerator;
use crate::domain::item_body::{self, ItemBody};
use crate::domain::model::{Item, ItemPatch, ItemPlacement, ItemPosition, Page};
use crate::domain::nfc::to_nfc;
use crate::domain::property_key::{ListKey, PropertyKey};
use crate::domain::search::SearchIndex;
use crate::error::AppError;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::export::page_link_converter;
use crate::infrastructure::persistence::item_repository::parked_idx;
use crate::infrastructure::persistence::property_repository::list_values;
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

use super::extension_runtime::ItemRequest;
use super::flat_markdown_splitter::FlatMarkdownSplitter;
use super::markdown_parser::MarkdownParser;
use super::page_service::PageService;
use super::paste::{self, PasteMode};
use super::split;
use super::tree_markdown;

const ITEM_TARGET: &str = "item";
const PAGE_TARGET: &str = "page";

/// Result of exporting items as tree markdown.
#[derive(Debug, Clone)]
pub struct TreeMarkdownExport {
    pub tree_markdown: String,
    pub flat_markdown: String,
}

/// The task state of an item, with the times and the plan that go with it.
#[derive(Debug, Clone)]
pub struct TaskState {
    pub task: bool,
    pub begin: Option<String>,
    pub end: Option<String>,
    pub plan: Option<String>,
}

/// What a paste changed: the item pasted into, then the items it created, and where the
/// cursor is after the pasted text.
#[derive(Debug, Clone)]
pub struct Pasted {
    pub items: Vec<Item>,
    pub cursor_item_id: Hmtid,
    pub cursor: usize,
}

/// What a split wrote: the item whose text was cut, which is the source item when a
/// transclusion was split, and the new item.
#[derive(Debug, Clone)]
pub struct Split {
    pub item: Item,
    pub created: Item,
    /// The pages the split changed.
    pub page_ids: BTreeSet<Hmtid>,
}

/// What sending an item to the stash wrote: the heading the items went under, whether it was
/// made for it, and the moved items as they are now.
#[derive(Debug, Clone)]
pub struct Stashed {
    pub heading: Item,
    pub created: bool,
    pub items: Vec<Item>,
    /// The pages the stash changed.
    pub page_ids: BTreeSet<Hmtid>,
}

/// The heading the items sent to the stash go under.
const STASH_HEADING: &str = "# Stash";

/// What a move wrote: the moved items as they are now.
#[derive(Debug, Clone)]
pub struct Moved {
    pub items: Vec<Item>,
    /// The pages the move changed.
    pub page_ids: BTreeSet<Hmtid>,
}

/// A move of items as a key in an editor does it.
#[derive(Debug, Clone, Copy)]
pub struct MoveRequest<'a> {
    pub item_ids: &'a [Hmtid],
    pub target_page_id: &'a Hmtid,
    pub position: &'a ItemPosition,
    /// How many levels deeper the items go; 0 keeps their levels.
    pub level_delta: i32,
    /// The text the editor of the first item has now, when it is not saved yet. It is saved
    /// to that item (or to the source item of a transclusion) before the move.
    pub body: Option<&'a str>,
}

/// One operation of a batch.
#[derive(Debug, Clone)]
pub enum BatchOp {
    /// Makes an item with the given id, as the undo of a deletion makes the item again.
    Create {
        id: Hmtid,
        page_id: Hmtid,
        position: ItemPosition,
        level: i32,
        body: String,
        /// The source item the item shows, for a transclusion made again.
        transclude: Option<Hmtid>,
        /// The properties of the item made again: one value for a scalar key, the values of
        /// a list key.
        properties: Vec<(PropertyKey, Vec<String>)>,
    },
    /// Writes the given fields of an item. A task state is written whole, with its times.
    Update {
        id: Hmtid,
        body: Option<String>,
        level: Option<i32>,
        collapsed: Option<bool>,
        task: Option<TaskState>,
        /// The properties to write, by their keys: one value for a scalar key, the values of
        /// a list key. The other properties stay.
        properties: Vec<(String, Vec<String>)>,
        /// The keys of the properties to take away.
        dropped_properties: Vec<String>,
    },
    /// Moves items, in their order, to a position on a page: the undo of a move.
    Move {
        ids: Vec<Hmtid>,
        page_id: Hmtid,
        position: ItemPosition,
    },
    /// Deletes an item. The deletes of a batch go last, as one group.
    Delete { id: Hmtid },
}

/// What a batch came to.
#[derive(Debug, Clone)]
pub struct BatchApplied {
    /// The items written, as they are after the batch.
    pub items: Vec<Item>,
    /// The empty items made on the pages the batch left without an item.
    pub created: Vec<Item>,
    /// The pages the batch changed.
    pub page_ids: BTreeSet<Hmtid>,
}

/// What an operation does outside the database, once its transaction is committed: the
/// search index, the tag pages and the export of the pages it touched.
#[derive(Default)]
pub(crate) struct AfterCommit {
    removed_ids: Vec<String>,
    /// Each item to index anew, with the title of its page.
    indexed: Vec<(Item, String)>,
    /// The tags of an item before and after, for the tag pages.
    tag_changes: Vec<(Vec<String>, Vec<String>)>,
    pages: BTreeSet<Hmtid>,
}

impl AfterCommit {
    /// The pages the operation changed, to tell the clients of.
    pub(crate) fn pages(&self) -> &BTreeSet<Hmtid> {
        &self.pages
    }
}

pub struct ItemService {
    items: ItemRepository,
    parser: MarkdownParser,
    search: Arc<dyn SearchIndex>,
    export: Arc<MarkdownExportService>,
    properties: PropertyRepository,
    ids: Arc<IdGenerator>,
    /// Keeps the tag pages in step with the hashtags of the bodies.
    page_service: Arc<PageService>,
}

/// Formats the current time like `Instant.now().truncatedTo(SECONDS).toString()`.
fn now_in_seconds() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

impl ItemService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        items: ItemRepository,
        parser: MarkdownParser,
        search: Arc<dyn SearchIndex>,
        export: Arc<MarkdownExportService>,
        properties: PropertyRepository,
        ids: Arc<IdGenerator>,
        page_service: Arc<PageService>,
    ) -> ItemService {
        ItemService {
            items,
            parser,
            search,
            export,
            properties,
            ids,
            page_service,
        }
    }

    /// Creates a new item on the specified page.
    pub fn create_item(
        &self,
        page_id: &Hmtid,
        position: &ItemPosition,
        level: i32,
        body: &str,
        client_id: Option<Hmtid>,
    ) -> Result<Item, AppError> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let item =
            self.create_item_in(&tx, page_id, position, level, body, client_id, &mut effects)?;
        tx.commit()?;
        self.finish(effects);
        Ok(item)
    }

    /// Creates an item in the transaction. The page gets a new modified date; what the item
    /// changes outside the database goes into `effects`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_item_in(
        &self,
        tx: &Connection,
        page_id: &Hmtid,
        position: &ItemPosition,
        level: i32,
        body: &str,
        client_id: Option<Hmtid>,
        effects: &mut AfterCommit,
    ) -> Result<Item, AppError> {
        let id = match client_id {
            Some(id) => {
                // The undo of a deletion makes the item again with its id: it is not a
                // deleted item any more, or the import of its file would take it away
                ItemRepository::forget_deleted_in(tx, &id)?;
                id
            }
            None => self.ids.generate_item_id(),
        };
        let now = self.items.db().now_millis();
        let parsed = ItemBody::accept(body).map_err(AppError::bad_request)?;

        let mut existing = ItemRepository::find_by_page_id_in(tx, page_id)?;

        // Validate level constraint
        match find_item_above(&existing, position) {
            None => {
                if level != 1 {
                    return Err(AppError::bad_request(
                        "Level must be 1 when there is no item above the insertion position",
                    ));
                }
            }
            Some(above) => {
                if level > above.level + 1 {
                    return Err(AppError::bad_request(format!(
                        "Level must be at most {} (one more than the item above), but was {level}",
                        above.level + 1
                    )));
                }
            }
        }

        let idx = match calculate_insertion_index(&existing, position) {
            Ok(idx) => idx,
            Err(_) => {
                // Fractional index exhausted: reindex and retry
                reindex_page(tx, page_id, &existing)?;
                existing = ItemRepository::find_by_page_id_in(tx, page_id)?;
                calculate_insertion_index(&existing, position).map_err(AppError::internal)?
            }
        };

        let types = parsed.all_types();
        let page = PageRepository::find_by_id_in(tx, page_id)?
            .ok_or_else(|| AppError::bad_request(format!("Page not found: {page_id}")))?;

        let item = Item {
            idx,
            level,
            body: parsed.text.clone(),
            types,
            ..Item::new_on(id.clone(), &page, now)
        };
        ItemRepository::save_in(tx, &item)?;
        if item.idx.len() > fractional_index::MAX_LEN {
            shorten_indices_in(tx, page_id)?;
        }
        ItemRepository::recalculate_parent_ids_in(tx, page_id)?;
        PageRepository::touch_in(tx, page_id, now)?;
        // The new item may have got a parent
        let item = ItemRepository::find_by_id_in(tx, &id)?.unwrap_or(item);

        effects.pages.insert(page_id.clone());
        let hashtags = hashtag::extract_hashtags(&self.parser.parse(&item.body));
        if !hashtags.is_empty() {
            PropertyRepository::sync_tags_in(tx, ITEM_TARGET, item.id.as_str(), &hashtags)?;
            effects.tag_changes.push((Vec::new(), hashtags));
        }
        effects.indexed.push((item.clone(), page.title()));
        Ok(item)
    }

    /// Updates an existing item. Only the given fields are written.
    pub fn update_item(
        &self,
        item_id: &Hmtid,
        body: Option<&str>,
        level: Option<i32>,
        collapsed: Option<bool>,
    ) -> Result<Item, AppError> {
        let existing = self.get_item_by_id(item_id)?;
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let updated = self.update_item_in(&tx, &existing, body, level, collapsed, &mut effects)?;
        tx.commit()?;
        self.finish(effects);
        Ok(updated)
    }

    /// Writes the items an extension asked for on the page, in one transaction: all of them or
    /// none. An item placed last on a page whose only item is empty takes the place of that
    /// item, so a template does not start below an empty line. Its id is then the empty item's,
    /// and a later item placed after it goes after that one.
    pub fn apply_extension_items(
        &self,
        page_id: &Hmtid,
        requests: &[ItemRequest],
    ) -> Result<(), AppError> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let mut taken_ids: HashMap<Hmtid, Hmtid> = HashMap::new();
        for request in requests {
            let position = match &request.position {
                ItemPosition::After(id) => {
                    ItemPosition::After(taken_ids.get(id).unwrap_or(id).clone())
                }
                other => other.clone(),
            };
            let empty = match position {
                ItemPosition::Last => lone_empty_item_in(&tx, page_id)?,
                _ => None,
            };
            let written = match empty {
                Some(empty) => {
                    taken_ids.insert(request.id.clone(), empty.id.clone());
                    self.update_item_in(
                        &tx,
                        &empty,
                        Some(&request.body),
                        Some(1),
                        None,
                        &mut effects,
                    )?
                }
                None => {
                    let level = level_that_fits_in(&tx, page_id, &position, request.level)?;
                    self.create_item_in(
                        &tx,
                        page_id,
                        &position,
                        level,
                        &request.body,
                        Some(request.id.clone()),
                        &mut effects,
                    )?
                }
            };
            if let Some(source_id) = &request.transclude {
                self.set_transclusion_in(&tx, &written, &Hmtid::parse(source_id)?)?;
            }
        }
        tx.commit()?;
        self.finish(effects);
        Ok(())
    }

    /// Makes `item` mirror the source item, in the transaction.
    fn set_transclusion_in(
        &self,
        tx: &Connection,
        item: &Item,
        source_item_id: &Hmtid,
    ) -> Result<Item, AppError> {
        let source = ItemRepository::find_by_id_in(tx, source_item_id)?.ok_or_else(|| {
            AppError::bad_request(format!("Source item not found: {source_item_id}"))
        })?;
        if source.transclude_item_id.is_some() {
            return Err(AppError::bad_request(
                "Cannot transclude a transclusion item. Please select the original source item.",
            ));
        }
        let updated = Item {
            modified_date: self.items.db().now_millis(),
            body: source.body.clone(),
            transclude_item_id: Some(source_item_id.clone()),
            ..item.clone()
        };
        ItemRepository::save_in(tx, &updated)?;
        Ok(updated)
    }

    /// Updates an existing item in the transaction. Only the given fields are written.
    pub(crate) fn update_item_in(
        &self,
        tx: &Connection,
        existing: &Item,
        body: Option<&str>,
        level: Option<i32>,
        collapsed: Option<bool>,
        effects: &mut AfterCommit,
    ) -> Result<Item, AppError> {
        let item_id = &existing.id;
        let parsed = body
            .map(ItemBody::accept)
            .transpose()
            .map_err(AppError::bad_request)?;
        let new_body = parsed.as_ref().map(|p| p.text.clone());
        let body_types = parsed.as_ref().map(|p| p.text_types());
        // A task stays a task when its body changes; only the prefix can add the type here
        let task = match &parsed {
            Some(p) if p.task => Some(true),
            _ => None,
        };

        let now = self.items.db().now_millis();
        let patch = ItemPatch::new(new_body.clone(), body_types, task, level, collapsed, now)
            .map_err(AppError::bad_request)?;
        ItemRepository::patch_in(tx, item_id, &patch)?;

        if let Some(body) = &new_body {
            let before = PropertyRepository::find_tags_in(tx, ITEM_TARGET, item_id.as_str())?;
            let hashtags = hashtag::extract_hashtags(&self.parser.parse(body));
            PropertyRepository::sync_tags_in(tx, ITEM_TARGET, item_id.as_str(), &hashtags)?;
            effects.tag_changes.push((before, hashtags));
        }
        if level.is_some() {
            ItemRepository::recalculate_parent_ids_in(tx, &existing.page_id)?;
        }
        let updated = ItemRepository::find_by_id_in(tx, item_id)?
            .ok_or_else(|| AppError::not_found(format!("Item not found: {item_id}")))?;
        PageRepository::touch_in(tx, &existing.page_id, now)?;
        effects.pages.insert(existing.page_id.clone());

        // The search index holds the body only; level and collapsed do not change it
        if new_body.is_some() {
            let page = self.page_service.require_page(&existing.page_id)?;
            effects.indexed.push((updated.clone(), page.title()));
            sync_transcluders_in(tx, &updated, now, effects)?;
        }
        Ok(updated)
    }

    /// Adds delta to the level of every given item in one step.
    pub fn shift_levels(&self, item_ids: &[Hmtid], delta: i32) -> Result<Vec<Item>, AppError> {
        if item_ids.is_empty() {
            return Err(AppError::bad_request("itemIds must not be empty"));
        }
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        effects.pages.extend(
            find_all_in(&tx, item_ids)?
                .into_iter()
                .map(|item| item.page_id),
        );
        let now = self.items.db().now_millis();
        ItemRepository::shift_levels_in(&tx, item_ids, delta, now)?;
        // The items may have other parents now
        for page_id in &effects.pages {
            ItemRepository::recalculate_parent_ids_in(&tx, page_id)?;
            PageRepository::touch_in(&tx, page_id, now)?;
        }
        let shifted = find_all_in(&tx, item_ids)?;
        tx.commit()?;
        self.finish(effects);
        Ok(shifted)
    }

    /// Sends an item with its descendants to the stash of its page: right under the
    /// `# Stash` heading, as its children, in one transaction (ADR 011). The heading is made
    /// at the end of the page when the page has none. `body` is the text the editor of the
    /// item has now, when it is not saved yet: it is saved with the move.
    pub fn stash(&self, item_id: &Hmtid, body: Option<&str>) -> Result<Stashed, AppError> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let item = ItemRepository::find_by_id_in(&tx, item_id)?
            .ok_or_else(|| AppError::not_found(format!("Item not found: {item_id}")))?;
        if is_stash_heading(&item) {
            return Err(AppError::bad_request(
                "The stash heading itself cannot be sent to the stash",
            ));
        }
        let on_page = ItemRepository::find_by_page_id_in(&tx, &item.page_id)?;
        let (heading, created) = match on_page.iter().find(|it| is_stash_heading(it)) {
            Some(heading) => (heading.clone(), false),
            None => {
                let heading = self.create_item_in(
                    &tx,
                    &item.page_id,
                    &ItemPosition::Last,
                    1,
                    STASH_HEADING,
                    None,
                    &mut effects,
                )?;
                (heading, true)
            }
        };
        let group: Vec<Hmtid> = ItemRepository::find_item_with_descendants_in(&tx, &item.id)?
            .into_iter()
            .map(|it| it.id)
            .collect();
        // The newest comes first, right under the heading
        let request = MoveRequest {
            item_ids: &group,
            target_page_id: &item.page_id,
            position: &ItemPosition::After(heading.id.clone()),
            level_delta: (heading.level + 1) - item.level,
            body,
        };
        let items = self.move_edited_items_in(&tx, &request, &mut effects)?;
        tx.commit()?;
        let page_ids = effects.pages.clone();
        self.finish(effects);
        Ok(Stashed {
            heading,
            created,
            items,
            page_ids,
        })
    }

    /// Moves items to a target page at the specified position. All or nothing.
    pub fn move_items(
        &self,
        item_ids: &[Hmtid],
        target_page_id: &Hmtid,
        position: &ItemPosition,
        level_delta: Option<i32>,
    ) -> Result<Vec<Item>, AppError> {
        let moved = self.move_edited_items(&MoveRequest {
            item_ids,
            target_page_id,
            position,
            level_delta: level_delta.unwrap_or(0),
            body: None,
        });
        Ok(moved?.items)
    }

    /// Moves items as a key in an editor does: with the body of the request saved in the
    /// same transaction (ADR 011). A page the items leave empty keeps one empty item.
    pub fn move_edited_items(&self, request: &MoveRequest<'_>) -> Result<Moved, AppError> {
        // The write lock of the transaction keeps two moves from planning their indices from
        // the same snapshot of a page
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let items = self.move_edited_items_in(&tx, request, &mut effects)?;
        self.keep_pages_non_empty_in(&tx, &mut effects)?;
        tx.commit()?;
        let page_ids = effects.pages.clone();
        self.finish(effects);
        Ok(Moved { items, page_ids })
    }

    /// `move_edited_items` inside a transaction, with the pages to tell and to export in
    /// `effects`. Returns the moved items.
    pub(crate) fn move_edited_items_in(
        &self,
        tx: &Connection,
        request: &MoveRequest<'_>,
        effects: &mut AfterCommit,
    ) -> Result<Vec<Item>, AppError> {
        let MoveRequest {
            item_ids,
            target_page_id,
            position,
            level_delta,
            body,
        } = *request;
        if let (Some(body), Some(first)) = (body, item_ids.first()) {
            let edited = find_all_in(tx, std::slice::from_ref(first))?.remove(0);
            // A transclusion shows its source item. What is typed into it is text of the
            // source, and stays on the page of the source
            let edited = match &edited.transclude_item_id {
                Some(source_id) => find_all_in(tx, std::slice::from_ref(source_id))?.remove(0),
                None => edited,
            };
            self.update_item_in(tx, &edited, Some(body), None, None, effects)?;
        }
        self.move_items_in(tx, item_ids, target_page_id, position, level_delta, effects)?;
        find_all_in(tx, item_ids)
    }

    /// Moves items inside a transaction, with the pages to tell and to export in `effects`.
    fn move_items_in(
        &self,
        tx: &Connection,
        item_ids: &[Hmtid],
        target_page_id: &Hmtid,
        position: &ItemPosition,
        delta: i32,
        effects: &mut AfterCommit,
    ) -> Result<(), AppError> {
        PageRepository::find_by_id_in(tx, target_page_id)?.ok_or_else(|| {
            AppError::bad_request(format!("Target page not found: {target_page_id}"))
        })?;
        let moving = find_all_in(tx, item_ids)?;
        effects.pages.insert(target_page_id.clone());
        effects
            .pages
            .extend(moving.iter().map(|item| item.page_id.clone()));
        let mut staying = items_staying_on_in(tx, target_page_id, item_ids)?;
        validate_levels(&moving, delta, find_anchor(&staying, position)?)?;

        let new_idxs = match plan_indices(&staying, position, moving.len()) {
            Ok(v) => v,
            Err(PlanError::Invalid(msg)) => return Err(AppError::bad_request(msg)),
            Err(PlanError::NoGap) => {
                // No room left in the gap: reindex the page and plan again
                let all = ItemRepository::find_by_page_id_in(tx, target_page_id)?;
                reindex_page(tx, target_page_id, &all)?;
                staying = items_staying_on_in(tx, target_page_id, item_ids)?;
                plan_indices(&staying, position, moving.len()).map_err(|e| match e {
                    PlanError::Invalid(msg) => AppError::bad_request(msg),
                    PlanError::NoGap => AppError::internal("No index available"),
                })?
            }
        };
        let placements: Vec<ItemPlacement> = moving
            .iter()
            .zip(new_idxs)
            .map(|(item, idx)| ItemPlacement {
                item_id: item.id.clone(),
                idx,
            })
            .collect();
        let now = self.items.db().now_millis();
        ItemRepository::move_items_in(tx, target_page_id, &placements, delta, now)?;
        if placements
            .iter()
            .any(|p| p.idx.len() > fractional_index::MAX_LEN)
        {
            shorten_indices_in(tx, target_page_id)?;
        }
        // The pages the items left need new parents and a new export too
        for page_id in &effects.pages {
            ItemRepository::recalculate_parent_ids_in(tx, page_id)?;
            PageRepository::touch_in(tx, page_id, now)?;
        }
        Ok(())
    }

    /// Deletes an item together with its properties and removes it from the search index.
    /// Deletes one item. An item with children is refused: a deletion never leaves an item
    /// without its parent. The children go first, in one group, or they are merged away.
    pub fn delete_item(&self, item_id: &Hmtid) -> Result<(), AppError> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        self.remove_group_in(&tx, std::slice::from_ref(item_id), &mut effects)?;
        self.keep_pages_non_empty_in(&tx, &mut effects)?;
        tx.commit()?;
        self.finish(effects);
        Ok(())
    }

    /// Deletes items as one group, in one transaction: all of them or none. Every descendant
    /// of a deleted item must be in the group, so that no item is left without its parent.
    /// A page that has no item left gets an empty one in the same transaction. Returns the
    /// items made that way.
    pub fn delete_items(&self, item_ids: &[Hmtid]) -> Result<Vec<Item>, AppError> {
        if item_ids.is_empty() {
            return Err(AppError::bad_request("itemIds must not be empty"));
        }
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        self.remove_group_in(&tx, item_ids, &mut effects)?;
        let created = self.keep_pages_non_empty_in(&tx, &mut effects)?;
        tx.commit()?;
        self.finish(effects);
        Ok(created)
    }

    /// Keeps every page the transaction touched non-empty. Returns the items made.
    fn keep_pages_non_empty_in(
        &self,
        tx: &Connection,
        effects: &mut AfterCommit,
    ) -> Result<Vec<Item>, AppError> {
        let mut created = Vec::new();
        for page_id in effects.pages().clone() {
            created.extend(self.keep_page_non_empty_in(tx, &page_id, effects)?);
        }
        Ok(created)
    }

    /// A page keeps at least one item: an empty one is made when the page has none left.
    /// Returns the item made.
    pub(crate) fn keep_page_non_empty_in(
        &self,
        tx: &Connection,
        page_id: &Hmtid,
        effects: &mut AfterCommit,
    ) -> Result<Option<Item>, AppError> {
        if !ItemRepository::find_by_page_id_in(tx, page_id)?.is_empty() {
            return Ok(None);
        }
        self.create_item_in(tx, page_id, &ItemPosition::First, 1, "", None, effects)
            .map(Some)
    }

    /// Removes a group of items inside a transaction. The group is refused when it leaves a
    /// descendant of one of its items behind.
    pub(crate) fn remove_group_in(
        &self,
        tx: &Connection,
        item_ids: &[Hmtid],
        effects: &mut AfterCommit,
    ) -> Result<(), AppError> {
        let mut items = Vec::new();
        for id in item_ids {
            let item = ItemRepository::find_by_id_in(tx, id)?
                .ok_or_else(|| AppError::not_found(format!("Item not found: {id}")))?;
            items.push(item);
        }
        let group: BTreeSet<&str> = item_ids.iter().map(Hmtid::as_str).collect();
        for item in &items {
            for descendant in ItemRepository::find_item_with_descendants_in(tx, &item.id)? {
                if !group.contains(descendant.id.as_str()) {
                    return Err(AppError::bad_request(format!(
                        "Item {} has a descendant that is not deleted: {}",
                        item.id, descendant.id
                    )));
                }
            }
        }
        self.remove_items(tx, &items, effects)
    }

    /// Removes items inside a transaction: their copies go to their transcluders first, then
    /// the items go, children before parents, with their properties. The pages touched get
    /// their parents recalculated. What happens outside the database is returned for after
    /// the commit.
    fn remove_items(
        &self,
        tx: &Connection,
        items: &[Item],
        effects: &mut AfterCommit,
    ) -> Result<(), AppError> {
        // A transcluder keeps what it showed, so the descendants are copied while they exist
        for item in items {
            let transcluders = ItemRepository::find_transcluders_in(tx, &item.id)?;
            self.copy_descendants_to_transcluders(tx, item, &transcluders, effects)?;
            effects
                .pages
                .extend(transcluders.iter().map(|t| t.page_id.clone()));
        }
        let mut items: Vec<&Item> = items.iter().collect();
        items.sort_by_key(|item| std::cmp::Reverse(item.level));
        let mut removed_ids = Vec::new();
        for item in items {
            let tags = PropertyRepository::find_tags_in(tx, ITEM_TARGET, item.id.as_str())?;
            effects.tag_changes.push((tags, Vec::new()));
            ItemRepository::delete_in(tx, &item.id)?;
            ItemRepository::record_deleted_in(tx, &item.id, self.items.db().now_millis())?;
            removed_ids.push(item.id.to_string());
            effects.pages.insert(item.page_id.clone());
        }
        PropertyRepository::delete_all_for_targets_in(tx, ITEM_TARGET, &removed_ids)?;
        effects.removed_ids.extend(removed_ids);
        let now = self.items.db().now_millis();
        for page_id in &effects.pages {
            ItemRepository::recalculate_parent_ids_in(tx, page_id)?;
            PageRepository::touch_in(tx, page_id, now)?;
        }
        Ok(())
    }

    /// Gives an item a new body inside a transaction: its types, its hashtags and the stored
    /// copies in its transcluders follow. Returns the item as it is now.
    fn set_body_in(
        &self,
        tx: &Connection,
        item: &Item,
        body: &str,
        effects: &mut AfterCommit,
    ) -> Result<Item, AppError> {
        let parsed = ItemBody::accept(body).map_err(AppError::bad_request)?;
        let now = self.items.db().now_millis();
        // A task stays a task when its body changes; only the prefix can add the type here
        let patch = ItemPatch::new(
            Some(parsed.text.clone()),
            Some(parsed.text_types()),
            parsed.task.then_some(true),
            None,
            None,
            now,
        )
        .map_err(AppError::bad_request)?;
        ItemRepository::patch_in(tx, &item.id, &patch)?;
        let before = PropertyRepository::find_tags_in(tx, ITEM_TARGET, item.id.as_str())?;
        let hashtags = hashtag::extract_hashtags(&self.parser.parse(&parsed.text));
        PropertyRepository::sync_tags_in(tx, ITEM_TARGET, item.id.as_str(), &hashtags)?;
        effects.tag_changes.push((before, hashtags));
        let updated = ItemRepository::find_by_id_in(tx, &item.id)?
            .ok_or_else(|| AppError::not_found(format!("Item not found: {}", item.id)))?;
        let page = self.page_service.require_page(&item.page_id)?;
        effects.indexed.push((updated.clone(), page.title()));
        PageRepository::touch_in(tx, &item.page_id, now)?;
        effects.pages.insert(item.page_id.clone());
        sync_transcluders_in(tx, &updated, now, effects)?;
        Ok(updated)
    }

    /// The work that lives outside the database, done once the commit is through.
    pub(crate) fn finish(&self, effects: AfterCommit) {
        for id in &effects.removed_ids {
            self.search.remove_item(id);
        }
        for (item, title) in &effects.indexed {
            self.search.index_item(item, title);
        }
        for (before, after) in &effects.tag_changes {
            self.page_service.update_tag_pages(before, after);
        }
        for page_id in &effects.pages {
            self.export.queue_export(page_id);
        }
    }

    /// Before a source item goes, its descendants are copied below each transcluder, so the
    /// transcluder keeps showing what it showed. Its own body is already a stored copy.
    fn copy_descendants_to_transcluders(
        &self,
        tx: &Connection,
        source: &Item,
        transcluders: &[Item],
        effects: &mut AfterCommit,
    ) -> Result<(), AppError> {
        let descendants: Vec<Item> = ItemRepository::find_item_with_descendants_in(tx, &source.id)?
            .into_iter()
            .filter(|d| d.id != source.id)
            .collect();
        for transcluder in transcluders {
            let page = self.page_service.require_page(&transcluder.page_id)?;
            let mut above = transcluder.id.clone();
            for descendant in &descendants {
                let existing = ItemRepository::find_by_page_id_in(tx, &transcluder.page_id)?;
                let idx = calculate_insertion_index(&existing, &ItemPosition::After(above.clone()))
                    .map_err(AppError::internal)?;
                let copy = Item {
                    idx,
                    level: transcluder.level + descendant.level - source.level,
                    body: descendant.body.clone(),
                    collapsed: descendant.collapsed,
                    types: descendant.types.clone(),
                    transclude_item_id: descendant.transclude_item_id.clone(),
                    ..Item::new_on(
                        self.ids.generate_item_id(),
                        &page,
                        self.items.db().now_millis(),
                    )
                };
                ItemRepository::save_in(tx, &copy)?;
                let hashtags = hashtag::extract_hashtags(&self.parser.parse(&copy.body));
                if !hashtags.is_empty() {
                    PropertyRepository::sync_tags_in(tx, ITEM_TARGET, copy.id.as_str(), &hashtags)?;
                    effects.tag_changes.push((Vec::new(), hashtags));
                }
                above = copy.id.clone();
                effects.indexed.push((copy, page.title()));
            }
        }
        Ok(())
    }

    /// Joins an item into another: its body goes to the end of `into`, its children keep
    /// their level and get their parent recalculated, and the item itself is deleted.
    ///
    /// `body` and `into_body` are the bodies an editor has now, for an item with text that
    /// was typed and not saved yet. The stored body is joined where none is given.
    pub fn merge_item(
        &self,
        item_id: &Hmtid,
        into_id: &Hmtid,
        body: Option<&str>,
        into_body: Option<&str>,
    ) -> Result<Item, AppError> {
        let item = self.get_item_by_id(item_id)?;
        let into = self.get_item_by_id(into_id)?;
        // A transclusion shows its source: its body is not its own to give or to take
        if item.transclude_item_id.is_some() || into.transclude_item_id.is_some() {
            return Err(AppError::bad_request("A transclusion cannot be merged"));
        }
        let merged = format!(
            "{}{}",
            into_body.unwrap_or(&into.body),
            body.unwrap_or(&item.body)
        );
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let into = self.set_body_in(&tx, &into, &merged, &mut effects)?;
        // The children stay, with the receiving item as their parent: no group is needed
        self.remove_items(&tx, std::slice::from_ref(&item), &mut effects)?;
        tx.commit()?;
        self.finish(effects);
        Ok(into)
    }

    /// Pastes text into an item at the cursor of the body an editor has now. The text is split
    /// into items the way a markdown file is: the first piece joins the text before the cursor,
    /// the last piece takes the text after it, and the pieces after the first become new items
    /// below. All of it is written in one transaction.
    pub fn paste(
        &self,
        item_id: &Hmtid,
        body: &str,
        cursor: usize,
        text: &str,
        mode: PasteMode,
    ) -> Result<Pasted, AppError> {
        let item = self.get_item_by_id(item_id)?;
        // A transclusion shows its source item. What is pasted into it goes into the source,
        // as what is typed into it does
        let item = match &item.transclude_item_id {
            Some(source_id) => self.get_item_by_id(source_id)?,
            None => item,
        };
        let splitter = FlatMarkdownSplitter::new(self.parser.clone());
        let plan = paste::plan(&splitter, body, cursor, text, mode)?;

        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let first = &plan.pieces[0].body;
        let mut items =
            vec![self.update_item_in(&tx, &item, Some(first), None, None, &mut effects)?];
        for piece in &plan.pieces[1..] {
            let above = ItemPosition::After(items[items.len() - 1].id.clone());
            items.push(self.create_item_in(
                &tx,
                &item.page_id,
                &above,
                item.level + piece.level,
                &piece.body,
                None,
                &mut effects,
            )?);
        }
        tx.commit()?;
        self.finish(effects);
        Ok(Pasted {
            cursor_item_id: items[items.len() - 1].id.clone(),
            cursor: plan.cursor,
            items,
        })
    }

    /// Splits an item at the cursor of the body an editor has now, as Enter does: the text
    /// before the cursor stays, and the text after it goes into a new item below. Both are
    /// written in one transaction (ADR 011).
    ///
    /// `root_id` names the transclusion that shows the item, when the item is one of
    /// another page that a transclusion shows below itself.
    pub fn split(
        &self,
        item_id: &Hmtid,
        root_id: Option<&Hmtid>,
        body: &str,
        cursor: usize,
    ) -> Result<Split, AppError> {
        // The two parts are closed wherever the body is split. The body that comes in is
        // checked too, as every body that comes in is
        item_body::check_closed(body).map_err(AppError::bad_request)?;
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let item = find_all_in(&tx, std::slice::from_ref(item_id))?.remove(0);
        let shown_by = root_id
            .map(|root_id| {
                let root = find_all_in(&tx, std::slice::from_ref(root_id))?.remove(0);
                let shown = transclusion_in(&tx, root)?;
                if !shown.has_below(&item) {
                    return Err(AppError::bad_request(format!(
                        "The transclusion {root_id} does not show the item {item_id}"
                    )));
                }
                Ok(shown)
            })
            .transpose()?;
        let below = shown_by
            .as_ref()
            .and_then(|shown| split::plan_below(shown, &item, body, cursor));
        let (written, plan) = match (below, &item.transclude_item_id) {
            (Some(plan), _) => (item, plan),
            // A transclusion shows its source item: the text that is cut is the text of
            // the source
            (None, Some(_)) => {
                let shown = transclusion_in(&tx, item)?;
                let plan = split::plan_of_transclusion(&shown, body, cursor);
                (shown.source, plan)
            }
            (None, None) => {
                let on_page = ItemRepository::find_by_page_id_in(&tx, &item.page_id)?;
                let plan = split::plan(&on_page, &item, body, cursor);
                (item, plan)
            }
        };
        let updated =
            self.update_item_in(&tx, &written, Some(&plan.body), None, None, &mut effects)?;
        let created = self.create_item_in(
            &tx,
            &plan.page_id,
            &plan.position,
            plan.level,
            &plan.new_body,
            None,
            &mut effects,
        )?;
        tx.commit()?;
        let page_ids = effects.pages.clone();
        self.finish(effects);
        Ok(Split {
            item: updated,
            created,
            page_ids,
        })
    }

    /// Applies operations in their order, in one transaction: all of them or none. An undo
    /// or a redo sends what it replays this way (ADR 011).
    pub fn apply_batch(&self, ops: &[BatchOp]) -> Result<BatchApplied, AppError> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let mut written = Vec::new();
        let mut deleted = Vec::new();
        let mut transcluder_pages = BTreeSet::new();
        let mut transclusions = Vec::new();
        for op in ops {
            match op {
                BatchOp::Create {
                    id,
                    page_id,
                    position,
                    level,
                    body,
                    transclude,
                    properties,
                } => {
                    let created = self.create_item_in(
                        &tx,
                        page_id,
                        position,
                        *level,
                        body,
                        Some(id.clone()),
                        &mut effects,
                    )?;
                    if let Some(source) = transclude {
                        transclusions.push((created.id.clone(), source.clone()));
                    }
                    put_properties_in(&tx, id, properties)?;
                    written.push(id.clone());
                }
                BatchOp::Update {
                    id,
                    body,
                    level,
                    collapsed,
                    task,
                    properties,
                    dropped_properties,
                } => {
                    let existing = find_all_in(&tx, std::slice::from_ref(id))?.remove(0);
                    self.update_item_in(
                        &tx,
                        &existing,
                        body.as_deref(),
                        *level,
                        *collapsed,
                        &mut effects,
                    )?;
                    if let Some(state) = task {
                        put_task_in(&tx, &existing, state, self.items.db().now_millis())?;
                    }
                    for (key, value) in properties {
                        write_property_in(&tx, id, key, value)?;
                    }
                    for key in dropped_properties {
                        PropertyRepository::delete_property_in(&tx, ITEM_TARGET, id.as_str(), key)?;
                    }
                    if !properties.is_empty() || !dropped_properties.is_empty() {
                        ItemRepository::touch_in(&tx, id, self.items.db().now_millis())?;
                        effects.pages.insert(existing.page_id.clone());
                    }
                    // A transcluder shows the item as it is stored, so its page changed too
                    transcluder_pages.extend(
                        ItemRepository::find_transcluders_in(&tx, id)?
                            .into_iter()
                            .map(|transcluder| transcluder.page_id),
                    );
                    written.push(id.clone());
                }
                BatchOp::Move {
                    ids,
                    page_id,
                    position,
                } => {
                    self.move_items_in(&tx, ids, page_id, position, 0, &mut effects)?;
                    written.extend(ids.iter().cloned());
                }
                BatchOp::Delete { id } => deleted.push(id.clone()),
            }
        }
        self.transclude_made_again_in(&tx, &transclusions)?;
        // A parent deleted on its own is refused, so the deletes go together
        if !deleted.is_empty() {
            self.remove_group_in(&tx, &deleted, &mut effects)?;
        }
        let mut answered: Vec<Hmtid> = Vec::new();
        for id in written {
            if !deleted.contains(&id) && !answered.contains(&id) {
                answered.push(id);
            }
        }
        let items = find_all_in(&tx, &answered)?;
        let created = self.keep_pages_non_empty_in(&tx, &mut effects)?;
        tx.commit()?;
        let mut page_ids = effects.pages.clone();
        page_ids.extend(transcluder_pages);
        self.finish(effects);
        Ok(BatchApplied {
            items,
            created,
            page_ids,
        })
    }

    /// Makes the items of a batch show their sources again, once every item of the batch is
    /// made: an undo makes a source and the items that show it together, in any order. An
    /// item whose source is gone keeps the text it had, so that the undo is not refused.
    fn transclude_made_again_in(
        &self,
        tx: &Connection,
        transclusions: &[(Hmtid, Hmtid)],
    ) -> Result<(), AppError> {
        for (id, source) in transclusions {
            if ItemRepository::find_by_id_in(tx, source)?.is_none() {
                continue;
            }
            let item = find_all_in(tx, std::slice::from_ref(id))?.remove(0);
            self.set_transclusion_in(tx, &item, source)?;
        }
        Ok(())
    }

    /// Sets a transclusion on an item, making it mirror a source item.
    pub fn set_transclusion(
        &self,
        item_id: &Hmtid,
        source_item_id: &Hmtid,
    ) -> Result<Item, AppError> {
        let item = self.get_item_by_id(item_id)?;
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let updated = self.set_transclusion_in(&tx, &item, source_item_id)?;
        self.commit_page(tx, &item.page_id, updated.modified_date)?;
        Ok(updated)
    }

    /// Removes a transclusion from an item, clearing its body.
    pub fn remove_transclusion(&self, item_id: &Hmtid) -> Result<Item, AppError> {
        let item = self.get_item_by_id(item_id)?;
        self.save_body_and_transclusion(&item, "", None)
    }

    fn save_body_and_transclusion(
        &self,
        item: &Item,
        body: &str,
        transclude: Option<Hmtid>,
    ) -> Result<Item, AppError> {
        let now = self.items.db().now_millis();
        let updated = Item {
            modified_date: now,
            body: body.to_string(),
            transclude_item_id: transclude,
            ..item.clone()
        };
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        ItemRepository::save_in(&tx, &updated)?;
        self.commit_page(tx, &item.page_id, now)?;
        Ok(updated)
    }

    pub fn get_item_by_id(&self, item_id: &Hmtid) -> Result<Item, AppError> {
        self.items
            .find_by_id(item_id)?
            .ok_or_else(|| AppError::bad_request(format!("Item not found: {item_id}")))
    }

    /// The items of the given types. With `without_property`, the items that have a value for
    /// that property are left out.
    pub fn get_items_by_types(
        &self,
        types: &[String],
        without_property: Option<&str>,
    ) -> Result<Vec<Item>, AppError> {
        self.items.find_by_types(types, without_property)
    }

    /// The page ID that contains the given item, or None if the item is not found.
    pub fn get_item_page_id(&self, item_id: &Hmtid) -> Result<Option<Hmtid>, AppError> {
        Ok(self.items.find_by_id(item_id)?.map(|b| b.page_id))
    }

    /// The pages that show the items: the pages the items are on, and the pages with an item
    /// that transcludes one of them. A change of the items is told to these pages. Before a
    /// deletion they are found first, as the items are gone after it.
    pub fn pages_showing(&self, item_ids: &[Hmtid]) -> Result<BTreeSet<Hmtid>, AppError> {
        let mut pages = BTreeSet::new();
        for id in item_ids {
            pages.extend(self.get_item_page_id(id)?);
            for t in self.items.find_transcluders(id)? {
                pages.insert(t.page_id);
            }
        }
        Ok(pages)
    }

    /// Cycles the task state of an item:
    /// not a task -> task; task without begin -> begin = now; begun -> end = now;
    /// begun and ended -> back to a normal item.
    /// An item of the journal of a day that becomes a task is planned for that day, in the
    /// same transaction.
    pub fn cycle_task(&self, item_id: &Hmtid) -> Result<Item, AppError> {
        let existing = self.get_item_by_id(item_id)?;
        let id = item_id.to_string();
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let scalars = PropertyRepository::find_scalar_by_targets_in(
            &tx,
            ITEM_TARGET,
            std::slice::from_ref(&id),
        )?;
        let scalars = scalars.get(&id).cloned().unwrap_or_default();
        let is_task = existing.is_task();
        let has_begin = scalars.contains_key("begin");
        let has_end = scalars.contains_key("end");
        let now = self.items.db().now_millis();

        let result = if !is_task {
            let r = patch_task_in(&tx, &existing, true, now)?;
            clear_task_times_in(&tx, &id)?;
            // A task made on the journal of a day is planned for that day
            if let Some(day) = existing.day() {
                PropertyRepository::upsert_scalar_in(&tx, ITEM_TARGET, &id, "plan", &day)?;
            }
            r
        } else if !has_begin && !has_end {
            PropertyRepository::upsert_scalar_in(
                &tx,
                ITEM_TARGET,
                &id,
                "begin",
                &now_in_seconds(),
            )?;
            touched_in(&tx, &existing, now)?
        } else if has_begin && !has_end {
            PropertyRepository::upsert_scalar_in(&tx, ITEM_TARGET, &id, "end", &now_in_seconds())?;
            touched_in(&tx, &existing, now)?
        } else {
            let r = patch_task_in(&tx, &existing, false, now)?;
            clear_task_times_in(&tx, &id)?;
            r
        };
        self.commit_page(tx, &existing.page_id, now)?;
        Ok(result)
    }

    /// Sets a property of an item. A key of a list type takes the values comma-separated.
    /// The page file is rewritten, as for any other change of the item.
    pub fn set_property(
        &self,
        item_id: &Hmtid,
        key: &str,
        _value_type: &str,
        value: &str,
    ) -> Result<Item, AppError> {
        let existing = self.get_item_by_id(item_id)?;
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        // A type is chosen only when a key is defined (ADR 047), so a key with no definition
        // takes no value
        let value_type = PropertyRepository::find_definition_in(&tx, key)?
            .map(|def| def.value_type)
            .ok_or_else(|| AppError::bad_request(format!("Property '{key}' is not defined")))?;
        if value_type == "tags" || value_type == "list" {
            let values = list_values(value);
            PropertyRepository::upsert_list_in(&tx, ITEM_TARGET, item_id.as_str(), key, &values)?;
        } else {
            PropertyRepository::upsert_scalar_in(&tx, ITEM_TARGET, item_id.as_str(), key, value)?;
        }
        let now = self.items.db().now_millis();
        ItemRepository::touch_in(&tx, item_id, now)?;
        self.commit_page(tx, &existing.page_id, now)?;
        Ok(existing)
    }

    /// Removes a property of an item. The page file is rewritten.
    pub fn delete_property(&self, item_id: &Hmtid, key: &str) -> Result<Item, AppError> {
        let existing = self.get_item_by_id(item_id)?;
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        PropertyRepository::delete_property_in(&tx, ITEM_TARGET, item_id.as_str(), key)?;
        let now = self.items.db().now_millis();
        ItemRepository::touch_in(&tx, item_id, now)?;
        self.commit_page(tx, &existing.page_id, now)?;
        Ok(existing)
    }

    pub fn export_page_markdown(&self, page_id: &Hmtid) -> Result<TreeMarkdownExport, AppError> {
        self.export(&self.items.find_by_page_id(page_id)?)
    }

    pub fn export_subtree_markdown(&self, item_id: &Hmtid) -> Result<TreeMarkdownExport, AppError> {
        self.export(&self.items.find_item_with_descendants(item_id)?)
    }

    /// Exports items as hierarchical tree markdown, flat markdown, and AST. Unknown IDs are skipped.
    pub fn export_tree_markdown(&self, item_ids: &[Hmtid]) -> Result<TreeMarkdownExport, AppError> {
        let mut items = Vec::new();
        for id in item_ids {
            if let Some(item) = self.items.find_by_id(id)? {
                items.push(item);
            }
        }
        self.export(&items)
    }

    fn export(&self, items: &[Item]) -> Result<TreeMarkdownExport, AppError> {
        let ids: Vec<String> = items.iter().map(|i| i.id.to_string()).collect();
        let scalar_props = self.properties.find_scalar_by_targets(ITEM_TARGET, &ids)?;
        let list_props = self.properties.find_list_by_targets(ITEM_TARGET, &ids)?;
        let tree_md = tree_markdown::items_to_tree_markdown(items);
        let flat_md = tree_markdown::items_to_flat_markdown(items, &scalar_props, &list_props);
        Ok(TreeMarkdownExport {
            tree_markdown: tree_md,
            flat_markdown: flat_md,
        })
    }

    /// Converts tree markdown (indented with tabs or spaces) to flat markdown.
    pub fn import_tree_markdown(&self, tree_markdown: &str) -> String {
        tree_markdown::tree_markdown_to_flat_markdown(tree_markdown)
    }

    /// Updates a page's title or folder, as `PageService::update_page` does, which also says
    /// what `overwrite` does. A tag page
    /// names its tag, so renaming one renames the tag on every page and item that has it,
    /// and the tags under it too. Returns the page and the other pages that changed.
    pub fn update_page(
        &self,
        page_id: &Hmtid,
        title: Option<&str>,
        dir_path: Option<&str>,
        overwrite: bool,
    ) -> Result<(Page, Vec<Hmtid>), AppError> {
        self.rename_page(page_id, title, dir_path, overwrite, false)
    }

    /// Demotes the index page of a directory, `a/b/index`, to the page `a/b`, in one
    /// transaction with the deletion of the atlas page of the directory. Any other page in
    /// the directory refuses it. `overwrite` is what it is for [`Self::update_page`].
    /// Returns the page and the other pages that changed.
    pub fn demote_index_page(
        &self,
        page_id: &Hmtid,
        overwrite: bool,
    ) -> Result<(Page, Vec<Hmtid>), AppError> {
        let existing = self.page_service.require_page(page_id)?;
        let dir = existing.dir_path.trim_end_matches('/');
        if existing.name != "index" || dir.is_empty() {
            return Err(AppError::bad_request(
                "Only the index page of a directory can be demoted",
            ));
        }
        let (parent_dir, name) = match dir.rsplit_once('/') {
            Some((parent, name)) => (format!("{parent}/"), name),
            None => (String::new(), dir),
        };
        self.rename_page(page_id, Some(name), Some(&parent_dir), overwrite, true)
    }

    fn rename_page(
        &self,
        page_id: &Hmtid,
        title: Option<&str>,
        dir_path: Option<&str>,
        overwrite: bool,
        drop_atlas: bool,
    ) -> Result<(Page, Vec<Hmtid>), AppError> {
        let title = title.map(to_nfc);
        let title = title.as_deref();
        let dir_path = dir_path.map(to_nfc);
        let dir_path = dir_path.as_deref();
        let existing = self.page_service.require_page(page_id)?;
        let moved = Page {
            name: title.map_or_else(|| existing.name.clone(), str::to_string),
            dir_path: dir_path.map_or_else(|| existing.dir_path.clone(), str::to_string),
            ..existing.clone()
        };
        let rename = match (
            self.page_service.tag_of_page(&existing),
            self.page_service.tag_of_page(&moved),
        ) {
            (Some(old), Some(new)) if old != new => {
                // A title the bodies cannot write as a hashtag would break the tag
                if !hashtag::is_valid(&new) {
                    return Err(AppError::bad_request(format!(
                        "'{new}' cannot be a tag: use only letters, digits, '_' and '-'"
                    )));
                }
                Some((old, new))
            }
            _ => None,
        };
        let page = self
            .page_service
            .rename_page(page_id, title, dir_path, overwrite, drop_atlas)?;
        let changed = match rename {
            Some((old, new)) => self.rename_tag(&old, &new)?,
            None => Vec::new(),
        };
        Ok((page, changed))
    }

    /// Renames the tag `old`, and the tags under it, in the tags of the pages and in the
    /// hashtags of the bodies. Returns the pages that changed.
    fn rename_tag(&self, old: &str, new: &str) -> Result<Vec<Hmtid>, AppError> {
        let mut page_ids = BTreeSet::new();
        let mut item_ids = BTreeSet::new();
        for value in self.properties.find_all_tag_values()? {
            if hashtag::renamed(&value, old, new).is_none() {
                continue;
            }
            page_ids.extend(
                self.properties
                    .find_page_ids_by_tag_value_ignoring_case(&value)?,
            );
            item_ids.extend(
                self.properties
                    .find_item_ids_by_tag_value_ignoring_case(&value, i64::MAX)?,
            );
        }

        let mut changed = BTreeSet::new();
        for id in page_ids {
            let Ok(page_id) = Hmtid::parse(&id) else {
                continue;
            };
            let mut tags: Vec<String> = Vec::new();
            for tag in self.properties.find_tags(PAGE_TARGET, &id)? {
                let tag = hashtag::renamed(&tag, old, new).unwrap_or(tag);
                if !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
            self.page_service
                .set_property(&page_id, "tags", "tags", &tags.join(","))?;
            changed.insert(page_id);
        }
        for id in item_ids {
            let Ok(item_id) = Hmtid::parse(&id) else {
                continue;
            };
            let item = self.get_item_by_id(&item_id)?;
            let body = page_link_converter::rename_hashtags(&item.body, old, new);
            if body == item.body {
                tracing::warn!("The tag '{old}' of item {item_id} was not found in its body");
                continue;
            }
            self.update_item(&item_id, Some(&body), None, None)?;
            changed.insert(item.page_id);
        }
        Ok(changed.into_iter().collect())
    }

    /// Commits a transaction that changed the items of one page, marking the page as changed
    /// inside it and queuing its file after it.
    fn commit_page(&self, tx: Transaction<'_>, page_id: &Hmtid, now: i64) -> Result<(), AppError> {
        PageRepository::touch_in(&tx, page_id, now)?;
        tx.commit()?;
        let mut effects = AfterCommit::default();
        effects.pages.insert(page_id.clone());
        self.finish(effects);
        Ok(())
    }
}

fn clear_task_times_in(tx: &Connection, item_id: &str) -> Result<(), AppError> {
    PropertyRepository::delete_property_in(tx, ITEM_TARGET, item_id, "begin")?;
    PropertyRepository::delete_property_in(tx, ITEM_TARGET, item_id, "end")?;
    Ok(())
}

fn set_or_clear_in(
    tx: &Connection,
    item_id: &str,
    key: &str,
    value: Option<&str>,
) -> Result<(), AppError> {
    match value {
        Some(value) => PropertyRepository::upsert_scalar_in(tx, ITEM_TARGET, item_id, key, value)?,
        None => PropertyRepository::delete_property_in(tx, ITEM_TARGET, item_id, key)?,
    };
    Ok(())
}

/// Adds or removes the task type without writing any other field of the item.
/// The item with a new date, as a change of its properties is a change of the item: another
/// device takes the later item when it syncs.
fn touched_in(tx: &Connection, existing: &Item, now: i64) -> Result<Item, AppError> {
    ItemRepository::touch_in(tx, &existing.id, now)?;
    Ok(Item {
        modified_date: now,
        ..existing.clone()
    })
}

/// Writes a property of an item, as a client sets it: the key must be defined, and the value
/// is taken by the type of the definition (ADR 047).
fn write_property_in(
    tx: &Connection,
    item_id: &Hmtid,
    key: &str,
    values: &[String],
) -> Result<(), AppError> {
    let value_type = PropertyRepository::find_definition_in(tx, key)?
        .map(|def| def.value_type)
        .ok_or_else(|| AppError::bad_request(format!("Property '{key}' is not defined")))?;
    let id = item_id.as_str();
    if value_type == "tags" || value_type == "list" {
        PropertyRepository::upsert_list_in(tx, ITEM_TARGET, id, key, values)
    } else {
        PropertyRepository::upsert_scalar_in(tx, ITEM_TARGET, id, key, &values.join(","))
    }
}

/// Writes the properties of an item made again, as the file of its page would bring them
/// in: a key that is no longer defined gets a provisional definition, so that the undo of a
/// deletion is not refused (ADR 046). The tags come from the hashtags of the body.
fn put_properties_in(
    tx: &Connection,
    item_id: &Hmtid,
    properties: &[(PropertyKey, Vec<String>)],
) -> Result<(), AppError> {
    for (key, values) in properties {
        match key {
            PropertyKey::List(ListKey::Tags) => {}
            PropertyKey::List(ListKey::User(_)) => PropertyRepository::take_in_list_in(
                tx,
                ITEM_TARGET,
                item_id.as_str(),
                key.as_str(),
                values,
            )?,
            PropertyKey::Scalar(_) => PropertyRepository::take_in_scalar_in(
                tx,
                ITEM_TARGET,
                item_id.as_str(),
                key.as_str(),
                &values.join(","),
            )?,
        }
    }
    Ok(())
}

/// Puts an item in a task state in one step, with the times and the plan that go with it. The
/// undo of a task cycle needs this: cycling back would write new times over the ones to
/// restore.
fn put_task_in(
    tx: &Connection,
    existing: &Item,
    state: &TaskState,
    now: i64,
) -> Result<(), AppError> {
    if !state.task && (state.begin.is_some() || state.end.is_some()) {
        return Err(AppError::bad_request("A normal item has no task times"));
    }
    let id = existing.id.to_string();
    patch_task_in(tx, existing, state.task, now)?;
    set_or_clear_in(tx, &id, "begin", state.begin.as_deref())?;
    set_or_clear_in(tx, &id, "end", state.end.as_deref())?;
    set_or_clear_in(tx, &id, "plan", state.plan.as_deref())?;
    Ok(())
}

fn patch_task_in(tx: &Connection, existing: &Item, task: bool, now: i64) -> Result<Item, AppError> {
    let patch =
        ItemPatch::new(None, None, Some(task), None, None, now).map_err(AppError::internal)?;
    ItemRepository::patch_in(tx, &existing.id, &patch)?;
    Ok(ItemRepository::find_by_id_in(tx, &existing.id)?.unwrap_or_else(|| existing.clone()))
}

enum PlanError {
    Invalid(String),
    NoGap,
}

/// The item the group is placed under, or None when the group goes to the top of the page.
fn find_anchor<'a>(
    staying: &'a [Item],
    position: &ItemPosition,
) -> Result<Option<&'a Item>, AppError> {
    Ok(match position {
        ItemPosition::First => None,
        ItemPosition::Last => staying.last(),
        ItemPosition::After(anchor_id) => Some(
            staying
                .iter()
                .find(|item| &item.id == anchor_id)
                .ok_or_else(|| {
                    AppError::bad_request(format!(
                        "Position item is not on the target page or is being moved: {anchor_id}"
                    ))
                })?,
        ),
    })
}

fn validate_levels(moving: &[Item], delta: i32, anchor: Option<&Item>) -> Result<(), AppError> {
    let mut above_level: Option<i32> = anchor.map(|a| a.level);
    for item in moving {
        let new_level = (item.level + delta).max(1);
        match above_level {
            None => {
                if new_level != 1 {
                    return Err(AppError::bad_request(
                        "Level must be 1 when there is no item above the insertion position",
                    ));
                }
            }
            Some(above) => {
                if new_level > above + 1 {
                    return Err(AppError::bad_request(format!(
                        "Level must be at most {} (one more than the item above), but was {new_level}",
                        above + 1
                    )));
                }
            }
        }
        // For subsequent items, the item just placed is the one above
        above_level = Some(new_level);
    }
    Ok(())
}

/// Plans indices for the whole group inside the gap at the position, so none reaches the next item.
fn plan_indices(
    staying: &[Item],
    position: &ItemPosition,
    count: usize,
) -> Result<Vec<String>, PlanError> {
    let anchor = match find_anchor(staying, position) {
        Ok(a) => a,
        Err(e) => return Err(PlanError::Invalid(e.to_string())),
    };
    let next_pos = match anchor {
        None => 0,
        Some(a) => staying
            .iter()
            .position(|i| i.id == a.id)
            .map(|p| p + 1)
            .unwrap_or(0),
    };
    let after_idx = anchor.map(|a| a.idx.as_str());
    let next_idx = staying.get(next_pos).map(|i| i.idx.as_str());
    if after_idx.is_none() && next_idx.is_none() {
        return Ok(fractional_index::generate_indices(count));
    }
    fractional_index::between_many(after_idx, next_idx, count).map_err(|_| PlanError::NoGap)
}

/// Copies the body of a changed source item into every item that transcludes it, on the
/// given connection, and marks their pages for export. The UI resolves the source at read
/// time, but the exported Markdown is built from the stored copy, so the copy must follow
/// the source.
fn sync_transcluders_in(
    tx: &Connection,
    source: &Item,
    now: i64,
    effects: &mut AfterCommit,
) -> Result<(), AppError> {
    for transcluder in ItemRepository::find_transcluders_in(tx, &source.id)? {
        if transcluder.body != source.body {
            ItemRepository::save_in(
                tx,
                &Item {
                    modified_date: now,
                    body: source.body.clone(),
                    ..transcluder.clone()
                },
            )?;
        }
        PageRepository::touch_in(tx, &transcluder.page_id, now)?;
        effects.pages.insert(transcluder.page_id.clone());
    }
    Ok(())
}

/// A transclusion with what a split of an item it shows is planned from, on the given
/// connection.
fn transclusion_in(conn: &Connection, root: Item) -> Result<split::Transclusion, AppError> {
    let source_id = root
        .transclude_item_id
        .clone()
        .ok_or_else(|| AppError::bad_request(format!("Not a transclusion: {}", root.id)))?;
    let source = find_all_in(conn, std::slice::from_ref(&source_id))?.remove(0);
    Ok(split::Transclusion {
        root_page: ItemRepository::find_by_page_id_in(conn, &root.page_id)?,
        source_page: ItemRepository::find_by_page_id_in(conn, &source.page_id)?,
        root,
        source,
    })
}

/// The items with the given IDs, in that order, on the given connection.
fn find_all_in(conn: &Connection, item_ids: &[Hmtid]) -> Result<Vec<Item>, AppError> {
    item_ids
        .iter()
        .map(|id| {
            ItemRepository::find_by_id_in(conn, id)?
                .ok_or_else(|| AppError::bad_request(format!("Item not found: {id}")))
        })
        .collect()
}

/// The items of a page that are not among the moving ones, on the given connection.
fn items_staying_on_in(
    conn: &Connection,
    page_id: &Hmtid,
    moving_ids: &[Hmtid],
) -> Result<Vec<Item>, AppError> {
    let moving: BTreeSet<&Hmtid> = moving_ids.iter().collect();
    Ok(ItemRepository::find_by_page_id_in(conn, page_id)?
        .into_iter()
        .filter(|item| !moving.contains(&item.id))
        .collect())
}

/// Re-assigns fresh fractional indices to all items on a page, on the given connection.
fn reindex_page(conn: &Connection, page_id: &Hmtid, items: &[Item]) -> Result<(), AppError> {
    tracing::info!(
        "Reindexing the {} items of page {page_id}: the index space is used up",
        items.len()
    );
    let new_indices = fractional_index::generate_indices(items.len());
    let changes: Vec<(&Item, String)> = items.iter().zip(new_indices).collect();
    assign_indices_in(conn, &changes)
}

/// Reindexes the items around the indices grown past the limit, so the page stays short.
/// Only the indices change: the items keep their modified date, as the order is the same.
/// Falls back to the whole page when an index of the old scheme is in the way.
fn shorten_indices_in(conn: &Connection, page_id: &Hmtid) -> Result<(), AppError> {
    let items = ItemRepository::find_by_page_id_in(conn, page_id)?;
    let indices: Vec<&str> = items.iter().map(|i| i.idx.as_str()).collect();
    let Ok(changes) = fractional_index::shorten(&indices) else {
        return reindex_page(conn, page_id, &items);
    };
    let changes: Vec<(&Item, String)> = changes
        .into_iter()
        .map(|(position, idx)| (&items[position], idx))
        .collect();
    assign_indices_in(conn, &changes)
}

/// Writes new indices. The items are parked first: (page_id, idx) is unique and checked per
/// statement, so an item cannot take an index another still holds.
fn assign_indices_in(conn: &Connection, changes: &[(&Item, String)]) -> Result<(), AppError> {
    for (item, _) in changes {
        ItemRepository::update_index_in(conn, &item.id, &parked_idx(&item.id))?;
    }
    for (item, idx) in changes {
        ItemRepository::update_index_in(conn, &item.id, idx)?;
    }
    Ok(())
}

/// The fractional index for inserting an item at the specified position.
fn calculate_insertion_index(existing: &[Item], position: &ItemPosition) -> Result<String, String> {
    if existing.is_empty() {
        return Ok(fractional_index::initial());
    }
    let (before, after) = match position {
        ItemPosition::First => (None, existing.first().map(|i| i.idx.as_str())),
        ItemPosition::Last => (existing.last().map(|i| i.idx.as_str()), None),
        ItemPosition::After(item_id) => {
            let mut after_idx = None;
            let mut next_idx = None;
            for (i, item) in existing.iter().enumerate() {
                if &item.id == item_id {
                    after_idx = Some(item.idx.as_str());
                    next_idx = existing.get(i + 1).map(|n| n.idx.as_str());
                    break;
                }
            }
            (after_idx, next_idx)
        }
    };
    fractional_index::between(before, after).map_err(|e| e.0)
}

/// The item directly above the insertion position; None if the list is empty or position is FIRST.
fn find_item_above<'a>(existing: &'a [Item], position: &ItemPosition) -> Option<&'a Item> {
    if existing.is_empty() {
        return None;
    }
    match position {
        ItemPosition::First => None,
        ItemPosition::Last => existing.last(),
        ItemPosition::After(item_id) => existing.iter().find(|i| &i.id == item_id),
    }
}

/// `level`, lowered until an item can sit at `position`: at most one deeper than the item
/// above, and 1 with no item above.
fn level_that_fits_in(
    tx: &Connection,
    page_id: &Hmtid,
    position: &ItemPosition,
    level: i32,
) -> Result<i32, AppError> {
    let existing = ItemRepository::find_by_page_id_in(tx, page_id)?;
    let deepest = find_item_above(&existing, position).map_or(1, |above| above.level + 1);
    Ok(level.min(deepest))
}

/// The only item of the page when it is empty: no text and no transclusion.
fn lone_empty_item_in(tx: &Connection, page_id: &Hmtid) -> Result<Option<Item>, AppError> {
    let items = ItemRepository::find_by_page_id_in(tx, page_id)?;
    Ok(match items.as_slice() {
        [only] if only.body.trim().is_empty() && only.transclude_item_id.is_none() => {
            Some(only.clone())
        }
        _ => None,
    })
}

/// True for the heading the stash of a page is under.
fn is_stash_heading(item: &Item) -> bool {
    item.body.trim() == STASH_HEADING
}
