//! Item use cases.

use std::collections::BTreeSet;
use std::sync::Arc;

use hmtid::Hmtid;
use parking_lot::Mutex;

use crate::domain::fractional_index;
use crate::domain::hashtag;
use crate::domain::id_generator::IdGenerator;
use crate::domain::item_body::ItemBody;
use crate::domain::model::{Item, ItemPatch, ItemPlacement, ItemPosition, Page};
use crate::domain::search::SearchIndex;
use crate::error::{AppError, AppResult};
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::export::page_link_converter;
use crate::infrastructure::persistence::db::now_millis;
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

use super::markdown_parser::MarkdownParser;
use super::page_service::PageService;
use super::tree_markdown;

const ITEM_TARGET: &str = "item";
const PAGE_TARGET: &str = "page";

/// Result of exporting items as tree markdown.
#[derive(Debug, Clone)]
pub struct TreeMarkdownExport {
    pub tree_markdown: String,
    pub ast: String,
    pub flat_markdown: String,
}

pub struct ItemService {
    items: ItemRepository,
    pages: PageRepository,
    parser: MarkdownParser,
    search: Arc<dyn SearchIndex>,
    export: Arc<MarkdownExportService>,
    properties: PropertyRepository,
    ids: Arc<IdGenerator>,
    /// Keeps the tag pages in step with the hashtags of the bodies.
    page_service: Arc<PageService>,
    /// Two moves must not plan their indices from the same snapshot of a page.
    move_lock: Mutex<()>,
}

/// Formats the current time like `Instant.now().truncatedTo(SECONDS).toString()`.
fn now_in_seconds() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

impl ItemService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        items: ItemRepository,
        pages: PageRepository,
        parser: MarkdownParser,
        search: Arc<dyn SearchIndex>,
        export: Arc<MarkdownExportService>,
        properties: PropertyRepository,
        ids: Arc<IdGenerator>,
        page_service: Arc<PageService>,
    ) -> ItemService {
        ItemService {
            items,
            pages,
            parser,
            search,
            export,
            properties,
            ids,
            page_service,
            move_lock: Mutex::new(()),
        }
    }

    /// Creates a new item on the specified page.
    pub fn create_item(
        &self,
        page_id: &Hmtid,
        position: &ItemPosition,
        level: i32,
        body: Option<&str>,
        client_id: Option<Hmtid>,
    ) -> AppResult<Item> {
        let id = client_id.unwrap_or_else(|| self.ids.generate_item_id());
        let now = now_millis();
        let parsed = ItemBody::parse(body);
        let ast = self.parser.parse_to_ast(&parsed.text);

        let mut existing = self.items.find_by_page_id(page_id)?;

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
                self.reindex_page(page_id, &existing)?;
                existing = self.items.find_by_page_id(page_id)?;
                calculate_insertion_index(&existing, position).map_err(AppError::internal)?
            }
        };

        let types = parsed.all_types();
        let page = self.page_service.require_page(page_id)?;

        let item = Item {
            idx,
            level,
            body: parsed.text.clone(),
            ast: ast.clone(),
            types,
            ..Item::new_on(id.clone(), &page, now)
        };
        let item = self.items.save(&item)?;
        self.reparent_and_export(page_id)?;
        // The new item may have got a parent
        let item = self.items.find_by_id(&id)?.unwrap_or(item);

        // Sync hashtags as tag properties
        let hashtags = hashtag::extract_hashtags(&ast);
        if !hashtags.is_empty() {
            self.properties
                .sync_tags(ITEM_TARGET, id.as_str(), &hashtags)?;
            self.page_service.update_tag_pages(&[], &hashtags);
        }

        self.search.index_item(&item, &page.title());
        Ok(item)
    }

    /// Updates an existing item. Only the given fields are written.
    pub fn update_item(
        &self,
        item_id: &Hmtid,
        body: Option<&str>,
        level: Option<i32>,
        collapsed: Option<bool>,
    ) -> AppResult<Item> {
        let existing = self.get_item_by_id(item_id)?;

        let parsed = body.map(|b| ItemBody::parse(Some(b)));
        let new_body = parsed.as_ref().map(|p| p.text.clone());
        let new_ast = new_body.as_deref().map(|b| self.parser.parse_to_ast(b));
        let body_types = parsed.as_ref().map(|p| p.text_types());
        // A task stays a task when its body changes; only the prefix can add the type here
        let task = match &parsed {
            Some(p) if p.task => Some(true),
            _ => None,
        };

        let patch = ItemPatch::new(
            new_body.clone(),
            new_ast.clone(),
            body_types,
            task,
            level,
            collapsed,
            now_millis(),
        )
        .map_err(AppError::bad_request)?;
        self.items.patch(item_id, &patch)?;

        if let Some(ast) = &new_ast {
            let before = self.properties.find_tags(ITEM_TARGET, item_id.as_str())?;
            let hashtags = hashtag::extract_hashtags(ast);
            self.properties
                .sync_tags(ITEM_TARGET, item_id.as_str(), &hashtags)?;
            self.page_service.update_tag_pages(&before, &hashtags);
        }
        if level.is_some() {
            self.items.recalculate_parent_ids(&existing.page_id)?;
        }
        let updated = self.get_item_by_id(item_id)?;
        self.touch_and_export(&existing.page_id)?;

        // The search index holds the body only; level and collapsed do not change it
        if new_body.is_some() {
            if let Some(page) = self.pages.find_by_id(&existing.page_id)? {
                self.search.index_item(&updated, &page.title());
            }
            self.sync_transcluders(&updated)?;
        }
        Ok(updated)
    }

    /// Copies the body of a changed source item into every item that transcludes it, and
    /// queues their pages for export. The UI resolves the source at read time, but the
    /// exported Markdown is built from the stored copy, so the copy must follow the source.
    fn sync_transcluders(&self, source: &Item) -> AppResult<()> {
        for transcluder in self.items.find_transcluders(&source.id)? {
            if transcluder.body != source.body {
                self.items.save(&Item {
                    modified_date: now_millis(),
                    body: source.body.clone(),
                    ast: source.ast.clone(),
                    ..transcluder.clone()
                })?;
            }
            self.touch_and_export(&transcluder.page_id)?;
        }
        Ok(())
    }

    /// Adds delta to the level of every given item in one step.
    pub fn shift_levels(&self, item_ids: &[Hmtid], delta: i32) -> AppResult<Vec<Item>> {
        if item_ids.is_empty() {
            return Err(AppError::bad_request("itemIds must not be empty"));
        }
        let mut page_ids: Vec<Hmtid> = Vec::new();
        for id in item_ids {
            let page_id = self.get_item_by_id(id)?.page_id;
            if !page_ids.contains(&page_id) {
                page_ids.push(page_id);
            }
        }
        self.items.shift_levels(item_ids, delta, now_millis())?;
        for page_id in &page_ids {
            self.reparent_and_export(page_id)?;
        }
        self.find_all(item_ids)
    }

    /// Moves items to a target page at the specified position. All or nothing.
    pub fn move_items(
        &self,
        item_ids: &[Hmtid],
        target_page_id: &Hmtid,
        position: &ItemPosition,
        level_delta: Option<i32>,
    ) -> AppResult<Vec<Item>> {
        let delta = level_delta.unwrap_or(0);
        let mut touched: Vec<Hmtid> = vec![target_page_id.clone()];
        {
            let _guard = self.move_lock.lock();
            self.pages.find_by_id(target_page_id)?.ok_or_else(|| {
                AppError::bad_request(format!("Target page not found: {target_page_id}"))
            })?;
            let moving = self.find_all(item_ids)?;
            for item in &moving {
                if !touched.contains(&item.page_id) {
                    touched.push(item.page_id.clone());
                }
            }
            let mut staying = self.items_staying_on(target_page_id, item_ids)?;
            validate_levels(&moving, delta, find_anchor(&staying, position)?)?;

            let new_idxs = match plan_indices(&staying, position, moving.len()) {
                Ok(v) => v,
                Err(PlanError::Invalid(msg)) => return Err(AppError::bad_request(msg)),
                Err(PlanError::NoGap) => {
                    // No room left in the gap: reindex the page and plan again
                    let all = self.items.find_by_page_id(target_page_id)?;
                    self.reindex_page(target_page_id, &all)?;
                    staying = self.items_staying_on(target_page_id, item_ids)?;
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
            self.items
                .move_items(target_page_id, &placements, delta, now_millis())?;
        }
        // The pages the items left need new parents and a new export too
        for page_id in &touched {
            self.reparent_and_export(page_id)?;
        }
        self.find_all(item_ids)
    }

    fn items_staying_on(&self, page_id: &Hmtid, moving_ids: &[Hmtid]) -> AppResult<Vec<Item>> {
        let moving: BTreeSet<&Hmtid> = moving_ids.iter().collect();
        Ok(self
            .items
            .find_by_page_id(page_id)?
            .into_iter()
            .filter(|item| !moving.contains(&item.id))
            .collect())
    }

    /// Deletes an item together with its properties and removes it from the search index.
    pub fn delete_item(&self, item_id: &Hmtid) -> AppResult<()> {
        let item = self.items.find_by_id(item_id)?;
        // Deleting the source clears `transclude_item_id` on these items (FK ON DELETE SET
        // NULL), so their files must be rewritten without the transclude attribute
        let transcluders = self.items.find_transcluders(item_id)?;
        let tags = self.properties.find_tags(ITEM_TARGET, item_id.as_str())?;
        self.items.delete(item_id)?;
        self.search.remove_item(item_id.as_str());
        self.properties
            .delete_all_for_targets(ITEM_TARGET, &[item_id.to_string()])?;
        self.page_service.delete_unused_tag_pages(&tags);
        if let Some(b) = item {
            self.reparent_and_export(&b.page_id)?;
        }
        for t in transcluders {
            self.touch_and_export(&t.page_id)?;
        }
        Ok(())
    }

    /// Sets a transclusion on an item, making it mirror a source item.
    pub fn set_transclusion(&self, item_id: &Hmtid, source_item_id: &Hmtid) -> AppResult<Item> {
        let item = self.get_item_by_id(item_id)?;
        let source = self.items.find_by_id(source_item_id)?.ok_or_else(|| {
            AppError::bad_request(format!("Source item not found: {source_item_id}"))
        })?;
        if source.transclude_item_id.is_some() {
            return Err(AppError::bad_request(
                "Cannot transclude a transclusion item. Please select the original source item.",
            ));
        }
        self.save_body_and_transclusion(&item, &source.body, Some(source_item_id.clone()))
    }

    /// Removes a transclusion from an item, clearing its body.
    pub fn remove_transclusion(&self, item_id: &Hmtid) -> AppResult<Item> {
        let item = self.get_item_by_id(item_id)?;
        self.save_body_and_transclusion(&item, "", None)
    }

    fn save_body_and_transclusion(
        &self,
        item: &Item,
        body: &str,
        transclude: Option<Hmtid>,
    ) -> AppResult<Item> {
        let updated = Item {
            modified_date: now_millis(),
            body: body.to_string(),
            ast: self.parser.parse_to_ast(body),
            transclude_item_id: transclude,
            ..item.clone()
        };
        let updated = self.items.save(&updated)?;
        self.touch_and_export(&item.page_id)?;
        Ok(updated)
    }

    pub fn get_item_by_id(&self, item_id: &Hmtid) -> AppResult<Item> {
        self.items
            .find_by_id(item_id)?
            .ok_or_else(|| AppError::bad_request(format!("Item not found: {item_id}")))
    }

    /// The items with the given IDs, in that order.
    fn find_all(&self, item_ids: &[Hmtid]) -> AppResult<Vec<Item>> {
        item_ids.iter().map(|id| self.get_item_by_id(id)).collect()
    }

    pub fn get_items_by_types(&self, types: &[String]) -> AppResult<Vec<Item>> {
        self.items.find_by_types(types)
    }

    /// The page ID that contains the given item, or None if the item is not found.
    pub fn get_item_page_id(&self, item_id: &Hmtid) -> AppResult<Option<String>> {
        Ok(self
            .items
            .find_by_id(item_id)?
            .map(|b| b.page_id.to_string()))
    }

    /// Cycles the task state of an item:
    /// not a task -> task; task without begin -> begin = now; begun -> end = now;
    /// begun and ended -> back to a normal item.
    pub fn cycle_task(&self, item_id: &Hmtid) -> AppResult<Item> {
        let existing = self.get_item_by_id(item_id)?;
        let id = item_id.to_string();
        let scalars = self
            .properties
            .find_scalar_by_targets(ITEM_TARGET, std::slice::from_ref(&id))?;
        let scalars = scalars.get(&id).cloned().unwrap_or_default();
        let is_task = existing.is_task();
        let has_begin = scalars.contains_key("begin");
        let has_end = scalars.contains_key("end");

        let result = if !is_task {
            let r = self.patch_task(&existing, true)?;
            self.clear_task_times(&id)?;
            r
        } else if !has_begin && !has_end {
            self.properties.upsert_scalar(
                ITEM_TARGET,
                &id,
                "begin",
                "datetime",
                &now_in_seconds(),
            )?;
            existing.clone()
        } else if has_begin && !has_end {
            self.properties.upsert_scalar(
                ITEM_TARGET,
                &id,
                "end",
                "datetime",
                &now_in_seconds(),
            )?;
            existing.clone()
        } else {
            let r = self.patch_task(&existing, false)?;
            self.clear_task_times(&id)?;
            r
        };
        self.touch_and_export(&existing.page_id)?;
        Ok(result)
    }

    /// Sets a scalar property of an item. The page file is rewritten, as for any other
    /// change of the item.
    pub fn set_property(
        &self,
        item_id: &Hmtid,
        key: &str,
        value_type: &str,
        value: &str,
    ) -> AppResult<Item> {
        let existing = self.get_item_by_id(item_id)?;
        self.properties
            .upsert_scalar(ITEM_TARGET, item_id.as_str(), key, value_type, value)?;
        self.touch_and_export(&existing.page_id)?;
        Ok(existing)
    }

    /// Removes a property of an item. The page file is rewritten.
    pub fn delete_property(&self, item_id: &Hmtid, key: &str) -> AppResult<Item> {
        let existing = self.get_item_by_id(item_id)?;
        self.properties
            .delete_property(ITEM_TARGET, item_id.as_str(), key)?;
        self.touch_and_export(&existing.page_id)?;
        Ok(existing)
    }

    fn clear_task_times(&self, item_id: &str) -> AppResult<()> {
        self.properties
            .delete_property(ITEM_TARGET, item_id, "begin")?;
        self.properties
            .delete_property(ITEM_TARGET, item_id, "end")?;
        Ok(())
    }

    /// Adds or removes the task type without writing any other field of the item.
    fn patch_task(&self, existing: &Item, task: bool) -> AppResult<Item> {
        let patch = ItemPatch::new(None, None, None, Some(task), None, None, now_millis())
            .map_err(AppError::internal)?;
        self.items.patch(&existing.id, &patch)?;
        Ok(self
            .items
            .find_by_id(&existing.id)?
            .unwrap_or_else(|| existing.clone()))
    }

    pub fn export_page_markdown(&self, page_id: &Hmtid) -> AppResult<TreeMarkdownExport> {
        self.export(&self.items.find_by_page_id(page_id)?)
    }

    pub fn export_subtree_markdown(&self, item_id: &Hmtid) -> AppResult<TreeMarkdownExport> {
        self.export(&self.items.find_item_with_descendants(item_id)?)
    }

    /// Exports items as hierarchical tree markdown, flat markdown, and AST. Unknown IDs are skipped.
    pub fn export_tree_markdown(&self, item_ids: &[Hmtid]) -> AppResult<TreeMarkdownExport> {
        let mut items = Vec::new();
        for id in item_ids {
            if let Some(item) = self.items.find_by_id(id)? {
                items.push(item);
            }
        }
        self.export(&items)
    }

    fn export(&self, items: &[Item]) -> AppResult<TreeMarkdownExport> {
        let ids: Vec<String> = items.iter().map(|i| i.id.to_string()).collect();
        let scalar_props = self.properties.find_scalar_by_targets(ITEM_TARGET, &ids)?;
        let list_props = self.properties.find_list_by_targets(ITEM_TARGET, &ids)?;
        let tree_md = tree_markdown::items_to_tree_markdown(items);
        let ast = self.parser.parse_to_ast(&tree_md);
        let flat_md = tree_markdown::items_to_flat_markdown(items, &scalar_props, &list_props);
        Ok(TreeMarkdownExport {
            tree_markdown: tree_md,
            ast,
            flat_markdown: flat_md,
        })
    }

    /// Converts tree markdown (indented with tabs or spaces) to flat markdown.
    pub fn import_tree_markdown(&self, tree_markdown: &str) -> String {
        tree_markdown::tree_markdown_to_flat_markdown(tree_markdown)
    }

    /// Re-assigns fresh fractional indices to all items on a page.
    fn reindex_page(&self, page_id: &Hmtid, items: &[Item]) -> AppResult<()> {
        tracing::warn!(
            "REINDEX: Reassigning fractional indices for page {page_id} ({} items). \
             This may cause conflicts with CRDT-based sync.",
            items.len()
        );
        let new_indices = fractional_index::generate_indices(items.len());
        for (item, idx) in items.iter().zip(new_indices) {
            self.items.update_index(&item.id, &idx)?;
        }
        Ok(())
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
    ) -> AppResult<(Page, Vec<Hmtid>)> {
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
            .update_page(page_id, title, dir_path, overwrite)?;
        let changed = match rename {
            Some((old, new)) => self.rename_tag(&old, &new)?,
            None => Vec::new(),
        };
        Ok((page, changed))
    }

    /// Renames the tag `old`, and the tags under it, in the tags of the pages and in the
    /// hashtags of the bodies. Returns the pages that changed.
    fn rename_tag(&self, old: &str, new: &str) -> AppResult<Vec<Hmtid>> {
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

    /// Marks a page as changed and queues its file: what every change of an item does to its
    /// page. A page the item has lost is not written.
    fn touch_and_export(&self, page_id: &Hmtid) -> AppResult<()> {
        if let Some(page) = self.pages.find_by_id(page_id)? {
            let updated = Page {
                modified_date: now_millis(),
                ..page
            };
            self.pages.save(&updated)?;
        }
        self.export.queue_export(page_id);
        Ok(())
    }

    /// `touch_and_export` for a change that can give the items of the page other parents: an
    /// item added, removed, moved or moved to another level.
    fn reparent_and_export(&self, page_id: &Hmtid) -> AppResult<()> {
        self.items.recalculate_parent_ids(page_id)?;
        self.touch_and_export(page_id)
    }
}

enum PlanError {
    Invalid(String),
    NoGap,
}

/// The item the group is placed under, or None when the group goes to the top of the page.
fn find_anchor<'a>(staying: &'a [Item], position: &ItemPosition) -> AppResult<Option<&'a Item>> {
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

fn validate_levels(moving: &[Item], delta: i32, anchor: Option<&Item>) -> AppResult<()> {
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

/// The fractional index for inserting an item at the specified position.
fn calculate_insertion_index(existing: &[Item], position: &ItemPosition) -> Result<String, String> {
    if existing.is_empty() {
        return Ok(fractional_index::initial());
    }
    Ok(match position {
        ItemPosition::First => {
            fractional_index::between(None, existing.first().map(|i| i.idx.as_str()))
        }
        ItemPosition::Last => {
            fractional_index::between(existing.last().map(|i| i.idx.as_str()), None)
        }
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
            fractional_index::between(after_idx, next_idx)
        }
    })
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
