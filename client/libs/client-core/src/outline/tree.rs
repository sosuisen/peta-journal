//! Pure functions over the outline tree: a flat list of items whose nesting is given by
//! `level`. Ported from `outline-tree.ts` and `move-utils.ts`.
//!
//! Two lists appear throughout:
//! - the own items: the items of the pages shown in the outline, in order;
//! - the visible items: the own items plus transcluded descendants, minus collapsed subtrees.

use crate::model::{Item, ItemPosition, ItemRef};
use crate::outline::state::OwnItemIds;
use std::collections::{HashMap, HashSet};

/// Where an edit of an item shown in the outline is really stored.
/// A transclusion root shows the body of its source item, and a transcluded
/// descendant belongs to the source page, so both are saved there and the
/// levels they show are offset from the levels stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditContext {
    /// The page new items are created on.
    pub page_id: String,
    /// Shown level minus stored level; 0 outside a transclusion.
    pub level_offset: i32,
    /// True for a transclusion root or a transcluded descendant.
    pub in_transclusion: bool,
}

/// The transclusion root an item is shown under, with the level offset of that tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransclusionContext {
    pub root_item_id: String,
    pub root_display_level: i32,
    pub level_offset: i32,
}

pub fn index_of(items: &[ItemRef], item_id: &str) -> Option<usize> {
    items.iter().position(|b| b.id == item_id)
}

/// Gets an item and all its descendants from a flat ordered item list.
pub fn get_subtree(items: &[ItemRef], item_id: &str) -> Vec<ItemRef> {
    let Some(idx) = index_of(items, item_id) else {
        return Vec::new();
    };
    let level = items[idx].level;
    let mut result = vec![items[idx].clone()];
    for b in &items[idx + 1..] {
        if b.level <= level {
            break;
        }
        result.push(b.clone());
    }
    result
}

// ---- Placing new items ----

/// True when a sibling at the same level follows the item within the same parent.
pub fn has_sibling_below(items: &[ItemRef], item_id: &str) -> bool {
    let Some(idx) = index_of(items, item_id) else {
        return false;
    };
    let level = items[idx].level;
    for b in &items[idx + 1..] {
        if b.level < level {
            break;
        }
        if b.level == level {
            return true;
        }
    }
    false
}

/// The position right before an item: after the item above it, or first.
pub fn position_before(items: &[ItemRef], item_id: &str) -> ItemPosition {
    match index_of(items, item_id) {
        Some(idx) if idx > 0 => ItemPosition::after(items[idx - 1].id.clone()),
        _ => ItemPosition::First,
    }
}

/// Finds the position an item restored by undo goes back to: after the last item of its
/// page whose fractional index sorts before the restored item's index.
pub fn position_by_index(items: &[ItemRef], item: &Item) -> ItemPosition {
    let mut best: Option<&str> = None;
    for b in items {
        if b.page_id == item.page_id && b.idx < item.idx {
            best = Some(&b.id);
        }
    }
    match best {
        Some(id) => ItemPosition::after(id),
        None => ItemPosition::First,
    }
}

/// True when the index of an item does not sort between the items of its page above and
/// below it. The server gives the items around a new or moved item new indices when an index
/// grows too long, and answers only the item it wrote, so the indices held here are old.
pub fn is_out_of_place(items: &[ItemRef], item_id: &str) -> bool {
    let Some(at) = index_of(items, item_id) else {
        return false;
    };
    let item = &items[at];
    let same_page = |other: &&ItemRef| other.page_id == item.page_id;
    let above = items[..at].iter().rev().find(same_page);
    let below = items[at + 1..].iter().find(same_page);
    above.is_some_and(|a| a.idx >= item.idx) || below.is_some_and(|b| b.idx <= item.idx)
}

// ---- Indent and outdent ----

/// Whether an item at the given level can follow `prev_item`.
/// The item above must be a sibling (same level) or the parent (one level up);
/// with nothing above, only level 1 fits.
pub fn can_place(item_level: i32, prev_item: Option<&Item>) -> bool {
    match prev_item {
        None => item_level == 1,
        Some(prev) => prev.level == item_level || prev.level == item_level - 1,
    }
}

/// Whether an item can be indented by one level: the item above must accept the new level,
/// or a sibling at the item's current level must exist above it within the same parent.
pub fn can_indent(items: &[ItemRef], item_id: &str) -> bool {
    let Some(idx) = index_of(items, item_id) else {
        return false;
    };
    if idx == 0 {
        return false;
    }
    let item = &items[idx];
    if can_place(item.level + 1, Some(&items[idx - 1])) {
        return true;
    }
    // Exception: a sibling above at the same level makes the indent a nesting under that sibling
    for b in items[..idx].iter().rev() {
        if b.level < item.level {
            break;
        }
        if b.level == item.level {
            return true;
        }
    }
    false
}

/// The index of the parent of the item at idx: the nearest item above it one level up.
pub fn parent_index_of(items: &[ItemRef], idx: usize) -> Option<usize> {
    let level = items[idx].level;
    (0..idx).rev().find(|&i| items[i].level == level - 1)
}

/// The id of the last descendant of a parent that is not among the excluded ids,
/// or the parent's own id when there is none. Used to place an outdented group
/// after the parent's remaining children.
pub fn last_descendant_outside(
    items: &[ItemRef],
    parent_id: &str,
    excluded: &HashSet<String>,
) -> String {
    let mut last_id = parent_id.to_string();
    for child in get_subtree(items, parent_id).iter().skip(1) {
        if !excluded.contains(&child.id) {
            last_id = child.id.clone();
        }
    }
    last_id
}

// ---- Selection ----

/// The first index of a selection between the anchor and another item. When the
/// selection reaches into a subtree nested deeper than the anchor, it starts at the
/// ancestor on the anchor's level so that whole subtrees are selected.
pub fn selection_start_index(items: &[ItemRef], anchor_idx: usize, other_idx: usize) -> usize {
    let mut start = anchor_idx.min(other_idx);
    let anchor_level = items[anchor_idx].level;
    while start > 0 && items[start].level > anchor_level {
        start -= 1;
    }
    start
}

/// The ids of the visible items from start to end (inclusive), each with its selection group.
pub fn ids_of_range_with_subtrees(
    visible: &[ItemRef],
    own: &[ItemRef],
    start: usize,
    end: usize,
) -> HashSet<String> {
    let mut ids = HashSet::new();
    for item in &visible[start..=end.min(visible.len() - 1)] {
        ids.extend(selection_group(visible, own, &item.id));
    }
    ids
}

/// The ids of an item and of everything under it, as a selection takes them. The own items
/// give the collapsed children too. The visible items give what a transclusion shows under
/// it, which is not in the own items.
pub fn selection_group(visible: &[ItemRef], own: &[ItemRef], item_id: &str) -> HashSet<String> {
    let mut ids = HashSet::from([item_id.to_string()]);
    for b in get_subtree(own, item_id)
        .iter()
        .chain(get_subtree(visible, item_id).iter())
    {
        ids.insert(b.id.clone());
    }
    ids
}

/// The selected items whose parent is not selected, in outline order.
pub fn selected_root_items(items: &[ItemRef], selected: &HashSet<String>) -> Vec<ItemRef> {
    let mut roots = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        if !selected.contains(&item.id) {
            continue;
        }
        let parent_selected = items[..idx]
            .iter()
            .rev()
            .find(|b| b.level < item.level)
            .map(|parent| selected.contains(&parent.id))
            .unwrap_or(false);
        if !parent_selected {
            roots.push(item.clone());
        }
    }
    roots
}

// ---- Transclusion ----

/// Resolves the page, the level offset and the transclusion state for editing an item.
pub fn edit_context_of(
    item: &Item,
    own_ids: &OwnItemIds,
    trees: &HashMap<String, Vec<ItemRef>>,
) -> EditContext {
    let is_own = own_ids.contains(&item.id);
    if is_own {
        if let Some(source_id) = &item.transclude_item_id {
            // Transclusion root: edits go to the source page
            if let Some(root) = trees.get(source_id).and_then(|t| t.first()) {
                return EditContext {
                    page_id: root.page_id.clone(),
                    level_offset: item.level - root.level,
                    in_transclusion: true,
                };
            }
            return EditContext {
                page_id: item.page_id.clone(),
                level_offset: 0,
                in_transclusion: true,
            };
        }
        return EditContext {
            page_id: item.page_id.clone(),
            level_offset: 0,
            in_transclusion: false,
        };
    }
    // Transcluded descendant: it already belongs to the source page
    let level_offset = trees
        .values()
        .find_map(|tree| tree.iter().find(|t| t.id == item.id))
        .map(|found| item.level - found.level)
        .unwrap_or(0);
    EditContext {
        page_id: item.page_id.clone(),
        level_offset,
        in_transclusion: true,
    }
}

/// The transclusion root an item belongs to, or None outside a transclusion.
/// A transclusion root is its own context; a transcluded descendant gets the context
/// of the root that shows it.
pub fn transclusion_context_of(
    item: &Item,
    own_items: &[ItemRef],
    trees: &HashMap<String, Vec<ItemRef>>,
) -> Option<TransclusionContext> {
    let is_own = own_items.iter().any(|b| b.id == item.id);

    if is_own {
        let source_id = item.transclude_item_id.as_ref()?;
        let root = trees.get(source_id)?.first()?;
        return Some(TransclusionContext {
            root_display_level: item.level,
            level_offset: item.level - root.level,
            root_item_id: item.id.clone(),
        });
    }

    for (source_id, tree) in trees {
        if tree.iter().any(|t| t.id == item.id) {
            if let (Some(source_root), Some(root)) = (
                tree.first(),
                own_items
                    .iter()
                    .find(|b| b.transclude_item_id.as_deref() == Some(source_id)),
            ) {
                return Some(TransclusionContext {
                    root_display_level: root.level,
                    level_offset: root.level - source_root.level,
                    root_item_id: root.id.clone(),
                });
            }
        }
    }
    None
}

/// The visible descendants of a transclusion root, without the root itself.
pub fn transclusion_scoped_items(visible: &[ItemRef], ctx: &TransclusionContext) -> Vec<ItemRef> {
    let Some(root_idx) = index_of(visible, &ctx.root_item_id) else {
        return Vec::new();
    };
    visible[root_idx + 1..]
        .iter()
        .take_while(|b| b.level > ctx.root_display_level)
        .cloned()
        .collect()
}

// ---- Journal date groups ----

/// True when the visible item at the index is the first item of a journal day, so a date
/// header goes above it. Transcluded descendants belong to another page and never start
/// a group, and are skipped when looking for the item above.
pub fn starts_date_group(visible: &[ItemRef], own_ids: &OwnItemIds, index: usize) -> bool {
    if index == 0 {
        return true;
    }
    let current = &visible[index];
    if !own_ids.contains(&current.id) {
        return false;
    }
    for b in visible[..index].iter().rev() {
        if own_ids.contains(&b.id) {
            return current.name != b.name;
        }
    }
    true
}

// ---- Moving ----

/// Determines the target position for moving an item down.
///
/// Scans items below the moving item and finds the first item matching:
/// - 2+ levels shallower: movement not allowed.
/// - One level shallower: move to just under that item.
/// - Same level: move after that item's subtree.
pub fn find_move_down_target(items: &[ItemRef], item_id: &str) -> Option<ItemPosition> {
    let idx = index_of(items, item_id)?;
    let level = items[idx].level;
    for candidate in &items[idx + 1..] {
        if candidate.level == level {
            let subtree = get_subtree(items, &candidate.id);
            return subtree
                .last()
                .map(|last| ItemPosition::after(last.id.clone()));
        }
        if candidate.level == level - 1 {
            return Some(ItemPosition::after(candidate.id.clone()));
        }
        if candidate.level < level - 1 {
            return None;
        }
    }
    None
}

/// Determines the target position for moving an item up.
///
/// 1. If the item directly above is one level shallower, check one more above.
///    If that item's level >= item level - 1, move under it. Otherwise movement is not allowed.
/// 2. Scan upward for a same-level item. Move above it.
pub fn find_move_up_target(items: &[ItemRef], item_id: &str) -> Option<ItemPosition> {
    let idx = index_of(items, item_id)?;
    let level = items[idx].level;

    if idx >= 1 && items[idx - 1].level == level - 1 {
        if idx >= 2 {
            let two_above = &items[idx - 2];
            if two_above.level >= level - 1 {
                return Some(ItemPosition::after(two_above.id.clone()));
            }
        }
        return None;
    }

    for i in (0..idx).rev() {
        if items[i].level == level {
            return Some(if i == 0 {
                ItemPosition::First
            } else {
                ItemPosition::after(items[i - 1].id.clone())
            });
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanDirection {
    FromEnd,
    FromStart,
}

/// Finds a valid placement position for an item with the given level in a target page's
/// item list. The placement rule is: level <= above item's level + 1.
pub fn find_cross_page_placement(
    target_items: &[ItemRef],
    level: i32,
    direction: ScanDirection,
) -> Option<ItemPosition> {
    match direction {
        ScanDirection::FromEnd => {
            for b in target_items.iter().rev() {
                if level <= b.level + 1 {
                    return Some(ItemPosition::after(b.id.clone()));
                }
            }
            (level == 1).then_some(ItemPosition::First)
        }
        ScanDirection::FromStart => {
            if level == 1 {
                return Some(ItemPosition::First);
            }
            target_items
                .iter()
                .find(|b| level <= b.level + 1)
                .map(|b| ItemPosition::after(b.id.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::item;

    fn list(spec: &[(&str, i32)]) -> Vec<ItemRef> {
        spec.iter().map(|(id, level)| item(id, *level)).collect()
    }

    // 平坦な項目リストから部分木・下のきょうだいの有無・直前の位置が求められる。
    #[test]
    fn subtree_and_siblings() {
        let items = list(&[("a", 1), ("b", 2), ("c", 3), ("d", 2), ("e", 1)]);
        let subtree = get_subtree(&items, "a");
        let ids: Vec<&str> = subtree.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c", "d"]);
        assert!(has_sibling_below(&items, "b"));
        assert!(!has_sibling_below(&items, "d"));
        assert_eq!(position_before(&items, "a"), ItemPosition::First);
        assert_eq!(position_before(&items, "d"), ItemPosition::after("c"));
    }

    // 親の直下の最初の子はインデントできず、上に同レベルのきょうだいがある項目はインデントできる。親のインデックスは一段浅い直近の項目である。
    #[test]
    fn indent_rules() {
        let items = list(&[("a", 1), ("b", 2), ("c", 2), ("d", 1)]);
        assert!(!can_indent(&items, "a"));
        assert!(!can_indent(&items, "b")); // a is the parent, not a sibling
        assert!(can_indent(&items, "c"));
        assert!(can_indent(&items, "d"));
        assert_eq!(parent_index_of(&items, 2), Some(0));
        assert_eq!(parent_index_of(&items, 0), None);
    }

    // 上下移動の移動先は同レベルの項目の部分木の後ろや一段浅い項目の直下として求められ、移動できないときや空のリストでは None になる。
    #[test]
    fn move_targets() {
        let items = list(&[("a", 1), ("b", 2), ("c", 2), ("d", 1), ("e", 2)]);
        assert_eq!(
            find_move_down_target(&items, "a"),
            Some(ItemPosition::after("e"))
        );
        assert_eq!(
            find_move_down_target(&items, "c"),
            Some(ItemPosition::after("d"))
        );
        assert_eq!(find_move_up_target(&items, "d"), Some(ItemPosition::First));
        assert_eq!(
            find_move_up_target(&items, "c"),
            Some(ItemPosition::after("a"))
        );
        assert_eq!(find_move_up_target(&items, "b"), None);
        assert_eq!(
            find_move_up_target(&items, "e"),
            Some(ItemPosition::after("c"))
        );
        assert_eq!(
            find_cross_page_placement(&items, 2, ScanDirection::FromStart),
            Some(ItemPosition::after("a"))
        );
        assert_eq!(
            find_cross_page_placement(&[], 2, ScanDirection::FromEnd),
            None
        );
    }

    // 選択の開始位置はアンカーのレベルの先祖まで戻り、範囲は部分木ごと ID に展開され、親が選択済みの項目はルート項目に含まれない。
    #[test]
    fn selection_rules() {
        let items = list(&[("a", 1), ("b", 2), ("c", 2), ("d", 1)]);
        assert_eq!(selection_start_index(&items, 3, 1), 0);
        let ids = ids_of_range_with_subtrees(&items, &items, 0, 0);
        assert_eq!(ids.len(), 3);
        let selected: HashSet<String> = ["a", "b", "d"].iter().map(|s| s.to_string()).collect();
        let root_items = selected_root_items(&items, &selected);
        let roots: Vec<&str> = root_items.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(roots, ["a", "d"]);
    }

    // 先頭の項目と、上の項目と名前の日付が異なる項目は、日付グループの先頭になる。
    #[test]
    fn date_groups() {
        let mut a = item("a", 1);
        Arc::make_mut(&mut a).name = "2026-01-02".into();
        let mut b = item("b", 1);
        Arc::make_mut(&mut b).name = "2026-01-01".into();
        let visible = vec![a, b];
        let own: OwnItemIds = ["a", "b"].into_iter().collect();
        assert!(starts_date_group(&visible, &own, 0));
        assert!(starts_date_group(&visible, &own, 1));
    }

    use std::sync::Arc;
}
