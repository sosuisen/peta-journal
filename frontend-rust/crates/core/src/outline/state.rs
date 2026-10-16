//! The items of the outline and what the user is doing with them, ported from
//! `outline-state.service.ts`. The visible list is recomputed after every change, so
//! the readers never pay for it.

use crate::model::{Item, ItemPosition, ItemRef, TranscludedTrees};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A heading of the page, for the table of contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub id: String,
    pub text: String,
    pub level: u8,
}

/// The cursor an editor should open with, set once and consumed by the first editor that opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusRequest {
    /// Grows with every request, so the shell knows which one it already applied.
    pub seq: u64,
    pub item_id: String,
    pub cursor: Option<usize>,
    /// True when the edit started with a mouse press, so a drag may grow into a selection.
    pub mouse_drag: bool,
}

#[derive(Debug, Default)]
pub struct OutlineState {
    items: Vec<ItemRef>,
    own_item_ids: HashSet<String>,
    visible: Vec<ItemRef>,
    focused_item_id: Option<String>,
    editing_item_ids: HashSet<String>,
    selected_item_ids: HashSet<String>,
    collapsed_item_ids: HashSet<String>,
    item_selection_mode: bool,
    focus_request: Option<FocusRequest>,
    focus_seq: u64,
    transcluded_trees: HashMap<String, Vec<ItemRef>>,
}

impl OutlineState {
    // ---- Reading ----

    /// The own items: the items of the pages shown in the outline, in order.
    pub fn items(&self) -> &[ItemRef] {
        &self.items
    }

    /// The own items plus transcluded descendants, minus collapsed subtrees.
    pub fn visible_items(&self) -> &[ItemRef] {
        &self.visible
    }

    /// IDs of the items that belong to the pages shown in this outline. Transcluded
    /// descendants are shown too, but they belong to their source page and are not in this set.
    pub fn own_item_ids(&self) -> &HashSet<String> {
        &self.own_item_ids
    }

    pub fn transcluded_trees(&self) -> &HashMap<String, Vec<ItemRef>> {
        &self.transcluded_trees
    }

    pub fn focused_item_id(&self) -> Option<&str> {
        self.focused_item_id.as_deref()
    }

    pub fn editing_item_ids(&self) -> &HashSet<String> {
        &self.editing_item_ids
    }

    pub fn is_editing(&self, item_id: &str) -> bool {
        self.editing_item_ids.contains(item_id)
    }

    pub fn selected_item_ids(&self) -> &HashSet<String> {
        &self.selected_item_ids
    }

    pub fn item_selection_mode(&self) -> bool {
        self.item_selection_mode
    }

    pub fn focus_request(&self) -> Option<&FocusRequest> {
        self.focus_request.as_ref()
    }

    pub fn is_collapsed(&self, item_id: &str) -> bool {
        self.collapsed_item_ids.contains(item_id)
    }

    pub fn is_own_item(&self, item_id: &str) -> bool {
        self.own_item_ids.contains(item_id)
    }

    /// Finds an item by its id, in the own items or the transcluded trees.
    pub fn get_item_by_id(&self, item_id: &str) -> Option<ItemRef> {
        if let Some(found) = self.items.iter().find(|b| b.id == item_id) {
            return Some(found.clone());
        }
        self.transcluded_trees
            .values()
            .find_map(|tree| tree.iter().find(|b| b.id == item_id))
            .cloned()
    }

    /// The item as the visible list shows it (a transclusion root shows its source's body).
    pub fn get_visible_item(&self, item_id: &str) -> Option<ItemRef> {
        self.visible.iter().find(|b| b.id == item_id).cloned()
    }

    /// Checks whether the item has any immediate children (next item at a deeper level).
    pub fn has_children(&self, item_id: &str) -> bool {
        if let Some(idx) = self.items.iter().position(|b| b.id == item_id) {
            let item = &self.items[idx];
            if let Some(source_id) = &item.transclude_item_id {
                if self.transcluded_trees.get(source_id).map(|t| t.len() > 1).unwrap_or(false) {
                    return true;
                }
            }
            return self.items.get(idx + 1).map(|next| next.level > item.level).unwrap_or(false);
        }
        for tree in self.transcluded_trees.values() {
            if let Some(idx) = tree.iter().position(|t| t.id == item_id) {
                return tree.get(idx + 1).map(|next| next.level > tree[idx].level).unwrap_or(false);
            }
        }
        false
    }

    /// All descendant items of the given own item.
    pub fn get_children(&self, item_id: &str) -> Vec<ItemRef> {
        let Some(idx) = self.items.iter().position(|b| b.id == item_id) else { return Vec::new() };
        let parent_level = self.items[idx].level;
        self.items[idx + 1..]
            .iter()
            .take_while(|b| b.level > parent_level)
            .cloned()
            .collect()
    }

    /// The item and all its descendants as a group.
    pub fn get_item_with_children(&self, item_id: &str) -> Vec<ItemRef> {
        let Some(item) = self.get_item_by_id(item_id) else { return Vec::new() };
        let mut group = vec![item];
        group.extend(self.get_children(item_id));
        group
    }

    /// The headings of the own items, for the table of contents.
    pub fn headings(&self) -> Vec<Heading> {
        self.items
            .iter()
            .filter(|item| item.has_type("heading"))
            .map(|item| {
                let level = fm_ast::parse(&item.ast).ok().and_then(|ast| fm_ast::heading_level(&ast)).unwrap_or(1);
                let text = item.body.trim_start_matches('#').trim_start().to_string();
                Heading { id: item.id.clone(), text, level }
            })
            .collect()
    }

    // ---- Items ----

    /// Replaces all items and rebuilds the collapsed set from their collapsed flags.
    pub fn set_items(&mut self, items: Vec<Item>) {
        self.items = items.into_iter().map(Arc::new).collect();
        self.collapsed_item_ids = self.items.iter().filter(|b| b.collapsed).map(|b| b.id.clone()).collect();
        self.refresh();
    }

    /// Replaces all items, keeping the collapsed set as it is; for lists built from the
    /// current items (a page reloaded in place, journals appended).
    pub fn set_item_refs(&mut self, items: Vec<ItemRef>) {
        self.items = items;
        self.refresh();
    }

    pub fn add_item(&mut self, item: Item, position: &ItemPosition) {
        self.add_items(vec![item], position);
    }

    pub fn add_items(&mut self, items: Vec<Item>, position: &ItemPosition) {
        let idx = self.resolve_insertion_index(&self.items, position);
        let refs: Vec<ItemRef> = items.into_iter().map(Arc::new).collect();
        self.items.splice(idx..idx, refs);
        self.refresh();
    }

    /// Changes an item in place, in the own items or in a transcluded tree.
    pub fn update_item(&mut self, item_id: &str, change: impl FnOnce(&mut Item)) -> bool {
        if let Some(slot) = self.items.iter_mut().find(|b| b.id == item_id) {
            let mut item = (**slot).clone();
            change(&mut item);
            *slot = Arc::new(item);
            self.refresh();
            return true;
        }
        for tree in self.transcluded_trees.values_mut() {
            if let Some(slot) = tree.iter_mut().find(|b| b.id == item_id) {
                let mut item = (**slot).clone();
                change(&mut item);
                *slot = Arc::new(item);
                self.refresh();
                return true;
            }
        }
        false
    }

    /// Replaces an item with the given one, keeping its place.
    pub fn replace_item(&mut self, item: &Item) -> bool {
        let new = item.clone();
        self.update_item(&item.id, move |slot| *slot = new)
    }

    pub fn delete_item(&mut self, item_id: &str) {
        if self.items.iter().any(|b| b.id == item_id) {
            self.items.retain(|b| b.id != item_id);
        } else {
            for tree in self.transcluded_trees.values_mut() {
                if tree.iter().any(|b| b.id == item_id) {
                    tree.retain(|b| b.id != item_id);
                    break;
                }
            }
        }
        self.refresh();
    }

    pub fn delete_items(&mut self, item_ids: &[String]) {
        let ids: HashSet<&String> = item_ids.iter().collect();
        self.items.retain(|b| !ids.contains(&b.id));
        self.refresh();
    }

    /// Atomically removes old items and inserts new items at the target position.
    pub fn replace_items(&mut self, old_ids: &[String], new_items: Vec<Item>, position: &ItemPosition) {
        let ids: HashSet<&String> = old_ids.iter().collect();
        let mut filtered: Vec<ItemRef> = self.items.iter().filter(|b| !ids.contains(&b.id)).cloned().collect();
        let idx = self.resolve_insertion_index(&filtered, position);
        filtered.splice(idx..idx, new_items.into_iter().map(Arc::new));
        self.items = filtered;
        self.refresh();
    }

    // ---- Transclusion ----

    /// Replaces the transcluded trees with the given ones.
    pub fn set_transcluded_trees(&mut self, trees: TranscludedTrees) {
        self.transcluded_trees.clear();
        self.merge_transcluded_trees(trees);
    }

    /// Adds trees to the ones already held, e.g. when another page is appended to a journal.
    pub fn merge_transcluded_trees(&mut self, trees: TranscludedTrees) {
        for (source_id, items) in trees {
            for item in &items {
                if item.collapsed {
                    self.collapsed_item_ids.insert(item.id.clone());
                } else {
                    self.collapsed_item_ids.remove(&item.id);
                }
            }
            self.transcluded_trees.insert(source_id, items.into_iter().map(Arc::new).collect());
        }
        self.refresh();
    }

    // ---- Editing, focus, selection ----

    pub fn add_editing_item(&mut self, item_id: &str) {
        self.editing_item_ids.insert(item_id.to_string());
    }

    pub fn remove_editing_item(&mut self, item_id: &str) {
        self.editing_item_ids.remove(item_id);
    }

    pub fn remove_all_editing_items(&mut self) {
        self.editing_item_ids.clear();
    }

    /// Focuses an item and records the cursor the next editor of it opens with.
    pub fn set_focused_item(&mut self, item_id: &str, cursor: Option<usize>) {
        self.focused_item_id = Some(item_id.to_string());
        self.focus_seq += 1;
        self.focus_request = Some(FocusRequest {
            seq: self.focus_seq,
            item_id: item_id.to_string(),
            cursor,
            mouse_drag: false,
        });
    }

    /// Marks the pending focus as started by a mouse press.
    pub fn set_pending_mouse_drag(&mut self, value: bool) {
        if let Some(req) = &mut self.focus_request {
            req.mouse_drag = value;
        }
    }

    pub fn toggle_collapsed(&mut self, item_id: &str) {
        if !self.collapsed_item_ids.remove(item_id) {
            self.collapsed_item_ids.insert(item_id.to_string());
        }
        self.refresh();
    }

    pub fn set_selected_item_ids(&mut self, ids: HashSet<String>) {
        self.selected_item_ids = ids;
    }

    pub fn set_item_selection_mode(&mut self, active: bool) {
        self.item_selection_mode = active;
    }

    pub fn clear_selection(&mut self) {
        self.selected_item_ids.clear();
        self.item_selection_mode = false;
    }

    // ---- Internals ----

    fn resolve_insertion_index(&self, items: &[ItemRef], position: &ItemPosition) -> usize {
        match position {
            ItemPosition::First => 0,
            ItemPosition::Last => items.len(),
            ItemPosition::After(after_id) => items
                .iter()
                .position(|b| &b.id == after_id)
                .map(|i| i + 1)
                .unwrap_or(items.len()),
        }
    }

    /// Recomputes the own-id set and the visible list.
    fn refresh(&mut self) {
        self.own_item_ids = self.items.iter().map(|b| b.id.clone()).collect();
        self.visible = compute_visible(&self.items, &self.collapsed_item_ids, &self.transcluded_trees);
    }
}

/// The visible items: collapsed subtrees are skipped, a transclusion root shows the body,
/// types, properties and AST of its source, and the source's descendants follow it with
/// their levels mapped under the root.
fn compute_visible(
    items: &[ItemRef],
    collapsed: &HashSet<String>,
    trees: &HashMap<String, Vec<ItemRef>>,
) -> Vec<ItemRef> {
    let mut result = Vec::with_capacity(items.len());
    let mut skip_level: Option<i32> = None;

    for item in items {
        if let Some(level) = skip_level {
            if item.level > level {
                continue;
            }
        }
        skip_level = None;

        let tree = item.transclude_item_id.as_ref().and_then(|id| trees.get(id));
        match tree.and_then(|t| t.first()) {
            Some(source_root) => {
                let mut shown = (**item).clone();
                shown.body = source_root.body.clone();
                shown.types = source_root.types.clone();
                shown.properties = source_root.properties.clone();
                shown.ast = source_root.ast.clone();
                result.push(Arc::new(shown));
            }
            None => result.push(item.clone()),
        }

        if let Some(tree) = tree {
            if !collapsed.contains(&item.id) {
                if let Some(source_root) = tree.first() {
                    let mut t_skip: Option<i32> = None;
                    for desc in &tree[1..] {
                        let mapped_level = item.level + (desc.level - source_root.level);
                        if let Some(level) = t_skip {
                            if mapped_level > level {
                                continue;
                            }
                        }
                        t_skip = None;
                        let is_collapsed = collapsed.contains(&desc.id);
                        let mut shown = (**desc).clone();
                        shown.level = mapped_level;
                        shown.collapsed = is_collapsed;
                        result.push(Arc::new(shown));
                        if is_collapsed {
                            t_skip = Some(mapped_level);
                        }
                    }
                }
            }
        }

        if collapsed.contains(&item.id) {
            skip_level = Some(item.level);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{item_with, plain_item};

    #[test]
    fn collapsed_subtrees_are_hidden() {
        let mut state = OutlineState::default();
        state.set_items(vec![
            item_with("a", 1, |i| i.collapsed = true),
            plain_item("b", 2, ""),
            plain_item("c", 3, ""),
            plain_item("d", 1, ""),
        ]);
        let shown: Vec<&str> = state.visible_items().iter().map(|b| b.id.as_str()).collect();
        assert_eq!(shown, ["a", "d"]);
        assert!(state.has_children("a"));
        assert!(!state.has_children("d"));
        state.toggle_collapsed("a");
        assert_eq!(state.visible_items().len(), 4);
    }

    #[test]
    fn transclusions_show_the_source_tree() {
        let mut state = OutlineState::default();
        state.set_items(vec![
            item_with("root", 2, |i| i.transclude_item_id = Some("src".into())),
            plain_item("after", 1, ""),
        ]);
        let mut trees = TranscludedTrees::new();
        trees.insert(
            "src".into(),
            vec![plain_item("src", 1, "source body"), plain_item("child", 2, "child"), plain_item("grand", 3, "")],
        );
        state.set_transcluded_trees(trees);
        let shown: Vec<(String, i32, String)> =
            state.visible_items().iter().map(|b| (b.id.clone(), b.level, b.body.clone())).collect();
        assert_eq!(shown[0], ("root".to_string(), 2, "source body".to_string()));
        assert_eq!(shown[1], ("child".to_string(), 3, "child".to_string()));
        assert_eq!(shown[2].1, 4);
        assert_eq!(shown[3].0, "after");
        assert!(state.has_children("root"));
        assert!(state.has_children("child"));
        assert!(!state.is_own_item("child"));
        assert!(state.get_item_by_id("child").is_some());
        state.update_item("child", |i| i.body = "changed".into());
        assert_eq!(state.visible_items()[1].body, "changed");
        state.delete_item("grand");
        assert_eq!(state.visible_items().len(), 3);
    }

    #[test]
    fn insertion_positions() {
        let mut state = OutlineState::default();
        state.set_items(vec![plain_item("a", 1, ""), plain_item("b", 1, "")]);
        state.add_item(plain_item("x", 1, ""), &ItemPosition::after("a"));
        state.add_item(plain_item("y", 1, ""), &ItemPosition::First);
        state.add_item(plain_item("z", 1, ""), &ItemPosition::Last);
        let ids: Vec<&str> = state.items().iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["y", "a", "x", "b", "z"]);
        state.replace_items(&["a".into(), "x".into()], vec![plain_item("a", 1, ""), plain_item("x", 2, "")], &ItemPosition::after("z"));
        let ids: Vec<&str> = state.items().iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["y", "b", "z", "a", "x"]);
        assert_eq!(state.get_item_with_children("a").len(), 2);
    }

    #[test]
    fn focus_requests_are_numbered() {
        let mut state = OutlineState::default();
        state.set_focused_item("a", Some(3));
        let first = state.focus_request().unwrap().seq;
        state.set_focused_item("b", None);
        assert!(state.focus_request().unwrap().seq > first);
        assert_eq!(state.focused_item_id(), Some("b"));
    }
}
