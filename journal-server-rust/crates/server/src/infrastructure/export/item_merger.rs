//! Merges the items read from a markdown file into the items a page has in the database.
//! Items are matched by id; unchanged ones are left alone, including their index. The later
//! change of each item stays, so edits of different items on two devices are both kept.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use hmtid::Hmtid;
use rusqlite::Connection;

use crate::application::markdown_parser::MarkdownParser;
use crate::domain::hashtag;
use crate::domain::id_generator::IdGenerator;
use crate::domain::model::{Item, Page};
use crate::domain::search::SearchIndex;
use crate::error::AppResult;
use crate::infrastructure::persistence::property_repository::list_values;
use crate::infrastructure::persistence::{ItemRepository, PropertyRepository};

use super::order_planner;
use super::outline_parser::ParsedItem;
use super::page_link_converter;
use super::page_matcher::parse_id;
use super::path_mapper;

const RESERVED_KEYS: [&str; 5] = ["level", "collapsed", "transclude", "tags", "idx"];
const ITEM_TARGET: &str = "item";

/// Outcome of a merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeResult {
    /// Any item was added, changed or removed.
    pub changed: bool,
    /// The file lacks an id or idx, its idx comments disagree with the line order, or the
    /// page now holds what the file does not.
    pub needs_write_back: bool,
    /// The page now holds a change that no file holds yet, so it is newer than the file.
    pub changed_here: bool,
    /// A change in the file that was not seen here lost, so the file is worth keeping.
    pub file_lost: bool,
    /// An item was edited in the file by hand, and got a date the file does not have yet.
    pub restamped: bool,
    /// The hashtags of the page's items before and after the merge, so the caller can keep
    /// the tag pages in step once the file is fully imported.
    pub tags_before: Vec<String>,
    pub tags_after: Vec<String>,
}

pub struct ItemMerger {
    items: ItemRepository,
    parser: MarkdownParser,
    search: Arc<dyn SearchIndex>,
    ids: Arc<IdGenerator>,
}

impl ItemMerger {
    pub fn new(
        items: ItemRepository,
        parser: MarkdownParser,
        search: Arc<dyn SearchIndex>,
        ids: Arc<IdGenerator>,
    ) -> ItemMerger {
        ItemMerger {
            items,
            parser,
            search,
            ids,
        }
    }

    /// Makes the page's items the result of merging the file, in one transaction. The search
    /// index follows once the commit is through.
    pub fn merge(
        &self,
        page: &Page,
        parsed_items: &[ParsedItem],
        file_modified: i64,
        mode: MergeMode,
        now: i64,
    ) -> AppResult<MergeResult> {
        let page_id = &page.id;
        let page_rel_path = path_mapper::to_rel_path_of(page);
        let title = page.title();
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let existing = ItemRepository::find_by_page_id_in(&tx, page_id)?;
        let by_id: BTreeMap<&Hmtid, &Item> = existing.iter().map(|i| (&i.id, i)).collect();
        let existing_ids: Vec<String> = existing.iter().map(|i| i.id.to_string()).collect();
        let tags_before = self.tags_of(&tx, existing_ids.clone())?;
        let scalars =
            PropertyRepository::find_scalar_by_targets_in(&tx, ITEM_TARGET, &existing_ids)?;
        let lists = PropertyRepository::find_list_by_targets_in(&tx, ITEM_TARGET, &existing_ids)?;
        let defs = PropertyRepository::find_all_definitions_in(&tx)?;

        let (file_items, any_missing_id) =
            self.read_file_items(&tx, parsed_items, &by_id, &page_rel_path)?;
        let plan = plan_merge(
            &file_items,
            &existing,
            &MergeFacts {
                by_id: &by_id,
                page_modified: page.modified_date,
                file_modified,
                mode,
                scalars: &scalars,
                lists: &lists,
                defs: &defs,
            },
        );
        let entries = &plan.entries;
        let n = entries.len();

        // Decide ordering. The idx written in the file is the writer's DB value and is taken as-is,
        // so every device ends up with the same idx. The line order in the file still wins: idx
        // comments that disagree with it are recomputed.
        let wanted_idx: Vec<Option<String>> = entries.iter().map(Entry::wanted_idx).collect();
        let idxs = match order_planner::plan(&wanted_idx).and_then(order_planner::shortened) {
            Ok(v) => v,
            Err(_) => {
                tracing::warn!(
                    "Fractional index exhausted on page {page_id}, reindexing all items"
                );
                order_planner::reindex_all(n)
            }
        };
        let mut needs_write_back = any_missing_id || plan.differs_from_file;
        for (entry, idx) in entries.iter().zip(&idxs) {
            if let Source::File(fi) = &entry.source {
                if Some(idx) != fi.parsed.idx.as_ref() {
                    needs_write_back = true;
                }
            }
        }

        // Items that do not stay go first, so their idx values are free for the survivors
        let staying: BTreeSet<&Hmtid> = entries.iter().map(Entry::id).collect();
        let mut changed = false;
        let mut removed_ids: Vec<String> = Vec::new();
        for old in &existing {
            if staying.contains(&old.id) {
                continue;
            }
            ItemRepository::delete_in(&tx, &old.id)?;
            removed_ids.push(old.id.to_string());
            changed = true;
        }
        PropertyRepository::delete_all_for_targets_in(&tx, ITEM_TARGET, &removed_ids)?;

        // (page_id, idx) is unique: if a surviving item is about to take another survivor's current
        // idx, park every survivor on a temporary idx first.
        let idx_owner: BTreeMap<&str, &Hmtid> = existing
            .iter()
            .filter(|old| staying.contains(&old.id))
            .map(|old| (old.idx.as_str(), &old.id))
            .collect();
        let collision = entries.iter().zip(&idxs).any(|(entry, idx)| {
            matches!(idx_owner.get(idx.as_str()), Some(owner) if *owner != entry.id())
        });
        if collision {
            for old in &existing {
                if staying.contains(&old.id) {
                    ItemRepository::update_index_in(&tx, &old.id, &format!("\u{7f}{}", old.idx))?;
                }
            }
        }

        let mut to_index: Vec<Item> = Vec::new(); // indexed in one commit at the end
        let mut restamped = false;

        for (entry, idx) in entries.iter().zip(&idxs) {
            let fi = match &entry.source {
                Source::Kept(old) => {
                    if collision || old.idx != *idx {
                        ItemRepository::update_index_in(&tx, &old.id, idx)?;
                        changed = true;
                    }
                    continue;
                }
                Source::File(fi) => fi,
            };
            let pi = fi.parsed;
            let id = &fi.id;
            let old = by_id.get(id).copied();
            let types = pi.types.clone();

            let same_fields = old.is_some_and(|o| same_fields(o, fi));
            let same = same_fields && old.is_some_and(|o| o.idx == *idx) && !collision;
            let edited_by_hand = matches!(mode, MergeMode::ItemByItem { .. })
                && old.is_some_and(|o| {
                    pi.modified == Some(o.modified_date)
                        && !same_content(o, fi, &scalars, &lists, &defs)
                });
            let modified_date = match old {
                _ if edited_by_hand => now,
                // The same item with a date of the other device, or of no device
                Some(o) if same_fields => o.modified_date.max(pi.modified.unwrap_or(i64::MIN)),
                _ => pi.modified.unwrap_or(file_modified),
            };
            restamped |= edited_by_hand;

            if !same || old.is_some_and(|o| o.modified_date != modified_date) {
                let body_changed = old.is_none_or(|o| o.body != fi.body);
                let ast = if body_changed {
                    self.parser.parse_to_ast(&fi.body)
                } else {
                    old.map(|o| o.ast.clone()).unwrap_or_default()
                };
                let item = Item {
                    id: id.clone(),
                    page_id: page_id.clone(),
                    parent_item_id: None,
                    dir_path: page.dir_path.clone(),
                    name: page.name.clone(),
                    item_type: page.page_type.clone(),
                    created_date: old.map(|o| o.created_date).unwrap_or(now),
                    modified_date,
                    idx: idx.clone(),
                    level: pi.level,
                    body: fi.body.clone(),
                    ast: ast.clone(),
                    collapsed: pi.collapsed,
                    types,
                    transclude_item_id: fi.transclude.clone(),
                };
                ItemRepository::save_in(&tx, &item)?;
                if body_changed {
                    PropertyRepository::sync_tags_in(
                        &tx,
                        ITEM_TARGET,
                        id.as_str(),
                        &hashtag::extract_hashtags(&ast),
                    )?;
                }
                if !same {
                    to_index.push(item);
                    changed = true;
                }
            }

            let have = scalars.get(id.as_str()).cloned().unwrap_or_default();
            if self.sync_scalar_properties(&tx, id.as_str(), &pi.properties, &have, &defs)? {
                changed = true;
            }
            let have = lists.get(id.as_str()).cloned().unwrap_or_default();
            if self.sync_list_properties(&tx, id.as_str(), &pi.properties, &have, &defs)? {
                changed = true;
            }
        }

        if changed {
            ItemRepository::recalculate_parent_ids_in(&tx, page_id)?;
        }
        let staying_ids: Vec<String> = staying.iter().map(|id| id.to_string()).collect();
        let tags_after = self.tags_of(&tx, staying_ids)?;
        tx.commit()?;
        // The search index lives outside the database, so it follows the commit
        self.search.remove_items(&removed_ids); // one index commit for all of them
        self.search.index_items(&to_index, &title);
        Ok(MergeResult {
            changed,
            needs_write_back: needs_write_back || restamped,
            changed_here: plan.differs_from_file || restamped,
            file_lost: plan.file_lost,
            restamped,
            tags_before,
            tags_after,
        })
    }

    /// The items of the file with an id each and their body as the app stores it, and whether
    /// any item had to be given a new id.
    fn read_file_items<'a>(
        &self,
        tx: &Connection,
        parsed_items: &'a [ParsedItem],
        by_id: &BTreeMap<&Hmtid, &Item>,
        page_rel_path: &str,
    ) -> AppResult<(Vec<FileItem<'a>>, bool)> {
        let mut file_items = Vec::with_capacity(parsed_items.len());
        let mut seen: BTreeSet<Hmtid> = BTreeSet::new();
        let mut any_missing_id = false;
        for pi in parsed_items {
            let mut id = parse_id(pi.id.as_deref());
            if let Some(candidate) = &id {
                if !seen.contains(candidate)
                    && !by_id.contains_key(candidate)
                    && ItemRepository::find_by_id_in(tx, candidate)?.is_some()
                {
                    // Belongs to another page (copy-pasted between files): don't steal it
                    id = None;
                }
            }
            let (id, has_own_id) = match id {
                Some(i) if !seen.contains(&i) => (i, true),
                _ => {
                    any_missing_id = true;
                    (self.ids.generate_item_id(), false)
                }
            };
            seen.insert(id.clone());
            // The file holds relative Markdown links; the app stores wikilinks and hashtags
            let body = page_link_converter::to_wikilinks(&pi.body, page_rel_path);
            let mut transclude = parse_id(pi.transclude_item_id.as_deref());
            if transclude.is_none() {
                if let Some(o) = by_id.get(&id) {
                    if o.transclude_item_id.is_some() && body == o.body {
                        transclude = o.transclude_item_id.clone(); // file written before transclude comments existed
                    }
                }
            }
            file_items.push(FileItem {
                parsed: pi,
                id,
                has_own_id,
                body,
                transclude,
            });
        }
        Ok((file_items, any_missing_id))
    }

    /// The hashtags of the given items, each once.
    fn tags_of(&self, conn: &Connection, item_ids: Vec<String>) -> AppResult<Vec<String>> {
        let lists = PropertyRepository::find_list_by_targets_in(conn, ITEM_TARGET, &item_ids)?;
        let tags: BTreeSet<String> = lists
            .into_values()
            .filter_map(|mut l| l.remove("tags"))
            .flatten()
            .collect();
        Ok(tags.into_iter().collect())
    }

    /// Makes the scalar properties of an item match the key="value" pairs of its metadata line.
    /// Reserved keys and list-typed keys are not properties here. Returns true if anything changed.
    fn sync_scalar_properties(
        &self,
        conn: &Connection,
        item_id: &str,
        file_properties: &[(String, String)],
        have: &BTreeMap<String, String>,
        defs: &BTreeMap<String, String>,
    ) -> AppResult<bool> {
        let mut want: Vec<(&str, &str)> = Vec::new();
        for (k, v) in file_properties {
            if RESERVED_KEYS.contains(&k.as_str()) {
                continue;
            }
            let t = defs.get(k).map(String::as_str).unwrap_or("text");
            if t == "list" || t == "tags" {
                continue;
            }
            if let Some(entry) = want.iter_mut().find(|(wk, _)| *wk == k.as_str()) {
                entry.1 = v;
            } else {
                want.push((k, v));
            }
        }
        let mut changed = false;
        for (k, v) in &want {
            if have.get(*k).map(String::as_str) != Some(*v) {
                let t = defs.get(*k).map(String::as_str).unwrap_or("text");
                match PropertyRepository::upsert_scalar_in(conn, ITEM_TARGET, item_id, k, t, v) {
                    Ok(_) => changed = true,
                    Err(e) => tracing::warn!("Skipping property {k}={v} on item {item_id}: {e}"),
                }
            }
        }
        for key in have.keys() {
            if !want.iter().any(|(k, _)| *k == key.as_str()) {
                PropertyRepository::delete_property_in(conn, ITEM_TARGET, item_id, key)?;
                changed = true;
            }
        }
        Ok(changed)
    }

    /// Makes the list properties of an item match the `key="a,b"` pairs of its metadata line.
    /// The tags are not among them: they come from the hashtags of the body.
    fn sync_list_properties(
        &self,
        conn: &Connection,
        item_id: &str,
        file_properties: &[(String, String)],
        have: &BTreeMap<String, Vec<String>>,
        defs: &BTreeMap<String, String>,
    ) -> AppResult<bool> {
        let mut changed = false;
        let mut want: Vec<&str> = Vec::new();
        for (k, v) in file_properties {
            if defs.get(k).map(String::as_str) != Some("list") {
                continue;
            }
            want.push(k);
            let values = list_values(v);
            if have.get(k) != Some(&values) {
                PropertyRepository::upsert_list_in(conn, ITEM_TARGET, item_id, k, "list", &values)?;
                changed = true;
            }
        }
        for key in have.keys() {
            if key != "tags" && !want.contains(&key.as_str()) {
                PropertyRepository::delete_property_in(conn, ITEM_TARGET, item_id, key)?;
                changed = true;
            }
        }
        Ok(changed)
    }
}

/// How a file is taken in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMode {
    /// The file is the page: nothing here was made after what the file holds.
    WholeFile,
    /// The later change of each item stays. `last_synced` is when this device last wrote or
    /// took in a file of the page: a change in the file after it was not seen here yet.
    ItemByItem { last_synced: i64 },
}

/// An item of the file with its id and its body as the app stores it.
struct FileItem<'a> {
    parsed: &'a ParsedItem,
    id: Hmtid,
    /// False when the file gave no id, or one that cannot be this item's.
    has_own_id: bool,
    body: String,
    transclude: Option<Hmtid>,
}

/// Where an item that stays on the page comes from.
enum Source<'a> {
    File(&'a FileItem<'a>),
    Kept(&'a Item),
}

struct Entry<'a> {
    source: Source<'a>,
    /// The index of the item before the file came, for an item of the file.
    old_idx: Option<&'a str>,
}

impl Entry<'_> {
    fn id(&self) -> &Hmtid {
        match &self.source {
            Source::File(fi) => &fi.id,
            Source::Kept(old) => &old.id,
        }
    }

    fn wanted_idx(&self) -> Option<String> {
        match &self.source {
            Source::File(fi) => fi
                .parsed
                .idx
                .clone()
                .or_else(|| self.old_idx.map(str::to_string)),
            Source::Kept(old) => Some(old.idx.clone()),
        }
    }
}

struct MergeFacts<'a> {
    by_id: &'a BTreeMap<&'a Hmtid, &'a Item>,
    page_modified: i64,
    file_modified: i64,
    mode: MergeMode,
    scalars: &'a BTreeMap<String, BTreeMap<String, String>>,
    lists: &'a BTreeMap<String, BTreeMap<String, Vec<String>>>,
    defs: &'a BTreeMap<String, String>,
}

struct MergePlan<'a> {
    /// The items that stay, in page order.
    entries: Vec<Entry<'a>>,
    differs_from_file: bool,
    file_lost: bool,
}

/// Decides for every item which side it comes from, by the rules in
/// docs/folder-sync-future.md: the later change of an item stays; an item only here stays when
/// it changed after the file was written; an item only in the file comes in when it changed
/// after the page here, or when the file as a whole is not older than the page here.
fn plan_merge<'a>(
    file_items: &'a [FileItem<'a>],
    existing: &'a [Item],
    facts: &MergeFacts<'a>,
) -> MergePlan<'a> {
    let mut from_file: Vec<Entry<'a>> = Vec::with_capacity(file_items.len());
    let mut kept: Vec<&'a Item> = Vec::new();
    let mut differs_from_file = false;
    let mut file_lost = false;
    let (by_item, last_synced) = match facts.mode {
        MergeMode::WholeFile => (false, i64::MAX),
        MergeMode::ItemByItem { last_synced } => (true, last_synced),
    };

    for fi in file_items {
        let stamp = fi.parsed.modified.unwrap_or(facts.file_modified);
        let old = facts.by_id.get(&fi.id).copied();
        match old {
            Some(o) if by_item && o.modified_date > stamp => {
                let same = same_content(o, fi, facts.scalars, facts.lists, facts.defs);
                // A file not newer than the last sync is no file of the other device: it was
                // edited by hand from what was here before, so the edit is worth keeping too
                file_lost |= !same && (stamp > last_synced || facts.file_modified <= last_synced);
                differs_from_file |= !same || fi.parsed.idx.as_deref() != Some(o.idx.as_str());
                kept.push(o);
            }
            None if by_item
                && fi.has_own_id
                && stamp <= facts.page_modified
                && facts.file_modified < facts.page_modified =>
            {
                file_lost |= stamp > last_synced;
                differs_from_file = true;
            }
            _ => from_file.push(Entry {
                source: Source::File(fi),
                old_idx: old.map(|o| o.idx.as_str()),
            }),
        }
    }
    for old in existing {
        let in_file = file_items.iter().any(|fi| fi.id == old.id);
        if by_item && !in_file && old.modified_date > facts.file_modified {
            differs_from_file = true;
            kept.push(old);
        }
    }

    // The file keeps its line order, which may differ from its indexes when edited by hand.
    // A kept item goes in before the first item that sorts after it.
    let mut entries = from_file;
    for old in kept {
        let at = entries
            .iter()
            .position(|e| e.wanted_idx().is_some_and(|idx| idx > old.idx))
            .unwrap_or(entries.len());
        entries.insert(
            at,
            Entry {
                source: Source::Kept(old),
                old_idx: None,
            },
        );
    }
    MergePlan {
        entries,
        differs_from_file,
        file_lost,
    }
}

/// The fields of the item itself match; the properties and the index are not looked at.
fn same_fields(old: &Item, fi: &FileItem) -> bool {
    old.body == fi.body
        && old.level == fi.parsed.level
        && old.collapsed == fi.parsed.collapsed
        && same_types(&old.types, &fi.parsed.types)
        && old.transclude_item_id == fi.transclude
}

/// The item says the same in the file and here, its properties included.
fn same_content(
    old: &Item,
    fi: &FileItem,
    scalars: &BTreeMap<String, BTreeMap<String, String>>,
    lists: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
    defs: &BTreeMap<String, String>,
) -> bool {
    if !same_fields(old, fi) {
        return false;
    }
    let id = old.id.as_str();
    let mut here: BTreeMap<&str, String> = BTreeMap::new();
    for (k, v) in scalars.get(id).into_iter().flatten() {
        here.insert(k, v.clone());
    }
    for (k, v) in lists.get(id).into_iter().flatten() {
        if k != "tags" {
            here.insert(k, v.join(","));
        }
    }
    let in_file: BTreeMap<&str, String> = fi
        .parsed
        .properties
        .iter()
        .filter(|(k, _)| !RESERVED_KEYS.contains(&k.as_str()))
        .filter(|(k, _)| defs.get(k).map(String::as_str) != Some("tags"))
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    here == in_file
}

fn same_types(a: &[String], b: &[String]) -> bool {
    let sa: BTreeSet<&String> = a.iter().collect();
    let sb: BTreeSet<&String> = b.iter().collect();
    sa == sb
}
