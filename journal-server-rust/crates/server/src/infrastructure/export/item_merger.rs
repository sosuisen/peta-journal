//! Merges the items read from a markdown file into the items a page has in the database.
//! Items are matched by id; unchanged ones are left alone, including their index.
//! Items missing from the file are deleted, since the file is the newer version of the page.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use hmtid::Hmtid;

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
    /// The file lacks an id or idx, or its idx comments disagree with the line order.
    pub needs_write_back: bool,
    /// The hashtags of the page's items before and after the merge, so the caller can keep
    /// the tag pages in step once the file is fully imported.
    pub tags_before: Vec<String>,
    pub tags_after: Vec<String>,
}

pub struct ItemMerger {
    items: ItemRepository,
    properties: PropertyRepository,
    parser: MarkdownParser,
    search: Arc<dyn SearchIndex>,
    ids: Arc<IdGenerator>,
}

impl ItemMerger {
    pub fn new(
        items: ItemRepository,
        properties: PropertyRepository,
        parser: MarkdownParser,
        search: Arc<dyn SearchIndex>,
        ids: Arc<IdGenerator>,
    ) -> ItemMerger {
        ItemMerger {
            items,
            properties,
            parser,
            search,
            ids,
        }
    }

    /// Makes the page's items match the parsed ones.
    pub fn merge(
        &self,
        page: &Page,
        parsed_items: &[ParsedItem],
        now: i64,
    ) -> AppResult<MergeResult> {
        let page_id = &page.id;
        let page_rel_path = path_mapper::to_rel_path_of(page);
        let title = page.title();
        let existing = self.items.find_by_page_id(page_id)?;
        let by_id: BTreeMap<&Hmtid, &Item> = existing.iter().map(|i| (&i.id, i)).collect();
        let tags_before = self.tags_of(existing.iter().map(|i| i.id.to_string()).collect())?;

        // Resolve an id for every parsed item
        let n = parsed_items.len();
        let mut ids: Vec<Hmtid> = Vec::with_capacity(n);
        let mut seen: BTreeSet<Hmtid> = BTreeSet::new();
        let mut any_missing_id = false;
        for pi in parsed_items {
            let mut id = parse_id(pi.id.as_deref());
            if let Some(candidate) = &id {
                if !seen.contains(candidate)
                    && !by_id.contains_key(candidate)
                    && self.items.find_by_id(candidate)?.is_some()
                {
                    // Belongs to another page (copy-pasted between files): don't steal it
                    id = None;
                }
            }
            let id = match id {
                Some(i) if !seen.contains(&i) => i,
                _ => {
                    any_missing_id = true;
                    self.ids.generate_item_id()
                }
            };
            seen.insert(id.clone());
            ids.push(id);
        }

        // Decide ordering. The idx written in the file is the writer's DB value and is taken as-is,
        // so every device ends up with the same idx. The line order in the file still wins: idx
        // comments that disagree with it are recomputed.
        let wanted_idx: Vec<Option<String>> = (0..n)
            .map(|i| {
                parsed_items[i]
                    .idx
                    .clone()
                    .or_else(|| by_id.get(&ids[i]).map(|old| old.idx.clone()))
            })
            .collect();
        let idxs = match order_planner::plan(&wanted_idx) {
            Ok(v) => v,
            Err(_) => {
                tracing::warn!(
                    "Fractional index exhausted on page {page_id}, reindexing all items"
                );
                order_planner::reindex_all(n)
            }
        };
        let mut needs_write_back = any_missing_id;
        for i in 0..n {
            if Some(&idxs[i]) != parsed_items[i].idx.as_ref() {
                needs_write_back = true;
            }
        }

        // Items no longer in the file go first, so their idx values are free for the survivors
        let existing_ids: Vec<String> = existing.iter().map(|i| i.id.to_string()).collect();
        let scalars = self
            .properties
            .find_scalar_by_targets(ITEM_TARGET, &existing_ids)?;
        let lists = self
            .properties
            .find_list_by_targets(ITEM_TARGET, &existing_ids)?;
        let mut changed = false;
        let mut removed_ids: Vec<String> = Vec::new();
        for old in &existing {
            if seen.contains(&old.id) {
                continue;
            }
            self.items.delete(&old.id)?;
            removed_ids.push(old.id.to_string());
            changed = true;
        }
        self.properties
            .delete_all_for_targets(ITEM_TARGET, &removed_ids)?;
        self.search.remove_items(&removed_ids); // one index commit for all of them

        // (page_id, idx) is unique: if a surviving item is about to take another survivor's current
        // idx, park every survivor on a temporary idx first.
        let idx_owner: BTreeMap<&str, &Hmtid> = existing
            .iter()
            .filter(|old| seen.contains(&old.id))
            .map(|old| (old.idx.as_str(), &old.id))
            .collect();
        let collision = (0..n)
            .any(|i| matches!(idx_owner.get(idxs[i].as_str()), Some(owner) if **owner != ids[i]));
        if collision {
            for old in &existing {
                if seen.contains(&old.id) {
                    self.items
                        .update_index(&old.id, &format!("\u{7f}{}", old.idx))?;
                }
            }
        }

        let defs = self.properties.find_all_definitions()?;
        let mut to_index: Vec<Item> = Vec::new(); // indexed in one commit at the end

        for i in 0..n {
            let pi = &parsed_items[i];
            let id = &ids[i];
            let idx = &idxs[i];
            let old = by_id.get(id).copied();

            // The file holds relative Markdown links; the app stores wikilinks and hashtags
            let body = page_link_converter::to_wikilinks(&pi.body, &page_rel_path);
            let mut transclude = parse_id(pi.transclude_item_id.as_deref());
            if transclude.is_none() {
                if let Some(o) = old {
                    if o.transclude_item_id.is_some() && body == o.body {
                        transclude = o.transclude_item_id.clone(); // file written before transclude comments existed
                    }
                }
            }
            let types = pi.types.clone();

            let same = match old {
                Some(o) => {
                    o.body == body
                        && o.level == pi.level
                        && o.collapsed == pi.collapsed
                        && same_types(&o.types, &types)
                        && o.transclude_item_id == transclude
                        && o.idx == *idx
                        && !collision
                }
                None => false,
            };

            if !same {
                let body_changed = old.is_none_or(|o| o.body != body);
                let ast = if body_changed {
                    self.parser.parse_to_ast(&body)
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
                    modified_date: if body_changed || old.is_none() {
                        now
                    } else {
                        old.map(|o| o.modified_date).unwrap_or(now)
                    },
                    idx: idx.clone(),
                    level: pi.level,
                    body: body.clone(),
                    ast: ast.clone(),
                    collapsed: pi.collapsed,
                    types,
                    transclude_item_id: transclude,
                };
                self.items.save(&item)?;
                if body_changed {
                    self.properties.sync_tags(
                        ITEM_TARGET,
                        id.as_str(),
                        &hashtag::extract_hashtags(&ast),
                    )?;
                }
                to_index.push(item);
                changed = true;
            }

            let have = scalars.get(id.as_str()).cloned().unwrap_or_default();
            if self.sync_scalar_properties(id.as_str(), &pi.properties, &have, &defs)? {
                changed = true;
            }
            let have = lists.get(id.as_str()).cloned().unwrap_or_default();
            if self.sync_list_properties(id.as_str(), &pi.properties, &have, &defs)? {
                changed = true;
            }
        }

        self.search.index_items(&to_index, &title);
        if changed {
            self.items.recalculate_parent_ids(page_id)?;
        }
        let tags_after = self.tags_of(ids.iter().map(|id| id.to_string()).collect())?;
        Ok(MergeResult {
            changed,
            needs_write_back,
            tags_before,
            tags_after,
        })
    }

    /// The hashtags of the given items, each once.
    fn tags_of(&self, item_ids: Vec<String>) -> AppResult<Vec<String>> {
        let lists = self
            .properties
            .find_list_by_targets(ITEM_TARGET, &item_ids)?;
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
                match self.properties.upsert_scalar(ITEM_TARGET, item_id, k, t, v) {
                    Ok(_) => changed = true,
                    Err(e) => tracing::warn!("Skipping property {k}={v} on item {item_id}: {e}"),
                }
            }
        }
        for key in have.keys() {
            if !want.iter().any(|(k, _)| *k == key.as_str()) {
                self.properties.delete_property(ITEM_TARGET, item_id, key)?;
                changed = true;
            }
        }
        Ok(changed)
    }

    /// Makes the list properties of an item match the `key="a,b"` pairs of its metadata line.
    /// The tags are not among them: they come from the hashtags of the body.
    fn sync_list_properties(
        &self,
        item_id: &str,
        file_properties: &[(String, String)],
        have: &BTreeMap<String, Vec<String>>,
        defs: &BTreeMap<String, String>,
    ) -> AppResult<bool> {
        let mut changed = false;
        for (k, v) in file_properties {
            if defs.get(k).map(String::as_str) != Some("list") {
                continue;
            }
            let values = list_values(v);
            if have.get(k) != Some(&values) {
                self.properties
                    .upsert_list(ITEM_TARGET, item_id, k, "list", &values)?;
                changed = true;
            }
        }
        Ok(changed)
    }
}

fn same_types(a: &[String], b: &[String]) -> bool {
    let sa: BTreeSet<&String> = a.iter().collect();
    let sb: BTreeSet<&String> = b.iter().collect();
    sa == sb
}
