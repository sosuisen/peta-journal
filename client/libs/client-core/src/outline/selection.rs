//! Multi-item selection, ported from `outline-selection.ts`: which items are selected and
//! how the selection grows with Shift+Arrow or a mouse drag. The selected ids live in the
//! outline state; this keeps the anchor and extent. The document-level listeners that the
//! Angular class installed belong to the shell, which forwards the keys as events while
//! the state reports selection mode.

use super::state::OutlineState;
use super::tree::{get_subtree, ids_of_range_with_subtrees, index_of, selection_start_index};
use crate::model::ItemRef;
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct Selection {
    /// The item the selection started from.
    anchor_item_id: Option<String>,
    /// The item the selection was last extended to.
    extent_item_id: Option<String>,
    /// True while the mouse button is held after dragging out of an editor.
    dragging: bool,
}

impl Selection {
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// Starts a keyboard selection from an editor: the item with everything under it, as a
    /// selected item always comes with its descendants.
    pub fn start_from_editor(&mut self, state: &mut OutlineState, item_id: &str) {
        state.remove_all_editing_items();
        let ids = state.selection_group(item_id);
        self.enter(state, item_id, ids);
    }

    /// Selects an item together with its subtree, e.g. the item the page was opened on.
    pub fn select_subtree(&mut self, state: &mut OutlineState, item_id: &str) {
        let ids = state.selection_group(item_id);
        self.enter(state, item_id, ids);
    }

    /// Starts a mouse-drag selection from an editor: the item and its children are selected,
    /// and the selection follows the mouse until the button is released.
    pub fn start_drag(&mut self, state: &mut OutlineState, item_id: &str) {
        state.remove_all_editing_items();
        state.set_item_selection_mode(true);
        self.anchor_item_id = Some(item_id.to_string());
        self.dragging = true;
        let group = state.selection_group(item_id);
        state.set_selected_item_ids(group);
    }

    /// Extends a drag selection from the anchor to the item the mouse entered.
    pub fn extend_drag_to(&mut self, state: &mut OutlineState, item_id: &str) {
        if !self.dragging {
            return;
        }
        self.extent_item_id = Some(item_id.to_string());
        let items = state.visible_items();
        let (Some(anchor_idx), Some(current_idx)) = (
            self.anchor_item_id
                .as_deref()
                .and_then(|id| index_of(items, id)),
            index_of(items, item_id),
        ) else {
            return;
        };
        let start = selection_start_index(items, anchor_idx, current_idx);
        let end = anchor_idx.max(current_idx);
        self.select_range(state, start, end);
    }

    /// The mouse button was released: the selection stays, the drag ends.
    pub fn end_drag(&mut self) {
        self.dragging = false;
    }

    /// Ends the selection.
    pub fn exit(&mut self, state: &mut OutlineState) {
        state.clear_selection();
        self.anchor_item_id = None;
        self.extent_item_id = None;
        self.dragging = false;
    }

    pub fn extend_up(&mut self, state: &mut OutlineState) {
        let items = state.visible_items();
        let (Some(mut extent_idx), Some(anchor_idx)) = (
            self.extent_item_id
                .as_deref()
                .and_then(|id| index_of(items, id)),
            self.anchor_item_id
                .as_deref()
                .and_then(|id| index_of(items, id)),
        ) else {
            return;
        };
        if extent_idx == 0 {
            return;
        }
        // When expanding upward, skip past items already in the selection
        if extent_idx <= anchor_idx {
            let selected = state.selected_item_ids();
            while extent_idx >= 1 && selected.contains(&items[extent_idx - 1].id) {
                extent_idx -= 1;
            }
            if extent_idx == 0 {
                return;
            }
        }
        self.extent_item_id = Some(items[extent_idx - 1].id.clone());
        self.select_from_anchor_to_extent(state);
    }

    pub fn extend_down(&mut self, state: &mut OutlineState) {
        let items = state.visible_items();
        let (Some(mut extent_idx), Some(anchor_idx)) = (
            self.extent_item_id
                .as_deref()
                .and_then(|id| index_of(items, id)),
            self.anchor_item_id
                .as_deref()
                .and_then(|id| index_of(items, id)),
        ) else {
            return;
        };
        if extent_idx + 1 >= items.len() {
            return;
        }
        if extent_idx >= anchor_idx {
            // A selection from a child stays under its parent (ADR 049)
            let last = last_under_parent(items, anchor_idx);
            // When expanding downward, skip past items already in the selection
            let selected = state.selected_item_ids();
            while extent_idx + 1 <= last && selected.contains(&items[extent_idx + 1].id) {
                extent_idx += 1;
            }
            if extent_idx + 1 > last {
                return;
            }
        } else {
            // When shrinking from above, the extent is an ancestor that was pulled in with its
            // subtree; step past the subtree, or the next selection would pull it in again
            let subtree_end = extent_idx + get_subtree(items, &items[extent_idx].id).len() - 1;
            extent_idx = subtree_end.min(anchor_idx - 1);
        }
        self.extent_item_id = Some(items[extent_idx + 1].id.clone());
        self.select_from_anchor_to_extent(state);
    }

    fn enter(&mut self, state: &mut OutlineState, anchor_item_id: &str, selected: HashSet<String>) {
        state.set_selected_item_ids(selected);
        state.set_item_selection_mode(true);
        self.anchor_item_id = Some(anchor_item_id.to_string());
        self.extent_item_id = Some(anchor_item_id.to_string());
    }

    fn select_from_anchor_to_extent(&mut self, state: &mut OutlineState) {
        let items = state.visible_items();
        let (Some(anchor_idx), Some(extent_idx)) = (
            self.anchor_item_id
                .as_deref()
                .and_then(|id| index_of(items, id)),
            self.extent_item_id
                .as_deref()
                .and_then(|id| index_of(items, id)),
        ) else {
            return;
        };
        let start = selection_start_index(items, anchor_idx, extent_idx);
        // When selecting upward into a subtree, the extent moves to the ancestor that was
        // pulled in, so the next Shift+Up continues from the correct position
        if extent_idx < anchor_idx && start < extent_idx {
            self.extent_item_id = Some(items[start].id.clone());
        }
        let end = anchor_idx.max(extent_idx);
        self.select_range(state, start, end);
    }

    /// Selects the items from start to end (inclusive) together with their subtrees.
    fn select_range(&self, state: &mut OutlineState, start: usize, end: usize) {
        let ids = ids_of_range_with_subtrees(state.visible_items(), state.items(), start, end);
        state.set_selected_item_ids(ids);
    }
}

/// The index of the last item that a selection from `items[idx]` can reach downward: the
/// last descendant of its parent, or the last item when it has no parent (ADR 049).
fn last_under_parent(items: &[ItemRef], idx: usize) -> usize {
    let level = items[idx].level;
    match items[..idx].iter().rposition(|item| item.level < level) {
        Some(parent) => parent + get_subtree(items, &items[parent].id).len() - 1,
        None => items.len() - 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TranscludedTrees;
    use crate::testing::{item_with, plain_item};

    fn state() -> OutlineState {
        let mut state = OutlineState::default();
        state.set_items(vec![
            plain_item("a", 1, ""),
            plain_item("b", 2, ""),
            plain_item("c", 2, ""),
            plain_item("d", 1, ""),
            plain_item("e", 1, ""),
        ]);
        state
    }

    fn selected(state: &OutlineState) -> Vec<String> {
        let mut ids: Vec<String> = state.selected_item_ids().iter().cloned().collect();
        ids.sort();
        ids
    }

    // エディタから始めた選択を上に伸ばすと上の項目がその部分木ごと含まれ、下に伸ばすと再び縮み、終了すると選択モードが解除される。
    #[test]
    fn shift_up_pulls_in_whole_subtrees() {
        let mut state = state();
        let mut sel = Selection::default();
        sel.start_from_editor(&mut state, "d");
        assert!(state.item_selection_mode());
        assert_eq!(selected(&state), ["d"]);
        sel.extend_up(&mut state);
        assert_eq!(selected(&state), ["a", "b", "c", "d"]);
        sel.extend_down(&mut state);
        assert_eq!(selected(&state), ["d"]);
        sel.extend_down(&mut state);
        assert_eq!(selected(&state), ["d", "e"]);
        sel.exit(&mut state);
        assert!(!state.item_selection_mode());
        assert!(state.selected_item_ids().is_empty());
    }

    // ドラッグ選択は開始項目の部分木から始まり、マウスが入った項目まで広がる。ボタンを離した後はマウスが動いても選択が変わらない。
    #[test]
    fn drag_selection_follows_the_mouse() {
        let mut state = state();
        let mut sel = Selection::default();
        sel.start_drag(&mut state, "a");
        assert_eq!(selected(&state), ["a", "b", "c"]);
        sel.extend_drag_to(&mut state, "e");
        assert_eq!(selected(&state), ["a", "b", "c", "d", "e"]);
        sel.end_drag();
        sel.extend_drag_to(&mut state, "a");
        assert_eq!(selected(&state).len(), 5);
    }

    // 子の項目から始めた選択は、Shift+Downで親の子孫の終わりまで広がり、そこで止まる。別の木へは広がらない。
    #[test]
    fn shift_down_from_a_child_stops_at_the_end_of_its_parent() {
        let mut state = state();
        let mut sel = Selection::default();
        sel.start_from_editor(&mut state, "b");
        sel.extend_down(&mut state);
        assert_eq!(selected(&state), ["b", "c"]);
        sel.extend_down(&mut state);
        assert_eq!(selected(&state), ["b", "c"]);
    }

    // 孫の項目から始めた選択は、Shift+Downで親の兄弟へ広がらない。
    #[test]
    fn shift_down_from_a_grandchild_stays_under_its_parent() {
        let mut state = OutlineState::default();
        state.set_items(vec![
            plain_item("a", 1, ""),
            plain_item("b", 2, ""),
            plain_item("g", 3, ""),
            plain_item("c", 2, ""),
        ]);
        let mut sel = Selection::default();
        sel.start_from_editor(&mut state, "g");
        sel.extend_down(&mut state);
        assert_eq!(selected(&state), ["g"]);
    }

    // 子の項目から始めたドラッグは、マウスが親の外へ出ても、親の子孫の終わりで止まる。
    #[test]
    fn a_drag_from_a_child_stops_at_the_end_of_its_parent() {
        let mut state = state();
        let mut sel = Selection::default();
        sel.start_drag(&mut state, "b");
        sel.extend_drag_to(&mut state, "e");
        assert_eq!(selected(&state), ["b", "c"]);
    }

    // エディタから始めた選択も、その項目の子孫をすべて含む。
    #[test]
    fn a_selection_from_an_editor_takes_the_children_too() {
        let mut state = state();
        let mut sel = Selection::default();
        sel.start_from_editor(&mut state, "a");
        assert_eq!(selected(&state), ["a", "b", "c"]);
    }

    /// "t" transcludes "src", which shows its child "c" under "t". "t" also has its own
    /// child "own", which is collapsed and so hidden, with its child "hidden".
    fn state_with_transclusion() -> OutlineState {
        let mut state = OutlineState::default();
        state.set_items(vec![
            plain_item("x", 1, ""),
            item_with("t", 1, |i| i.transclude_item_id = Some("src".into())),
            item_with("own", 2, |i| i.collapsed = true),
            plain_item("hidden", 3, ""),
            plain_item("y", 1, ""),
        ]);
        let mut trees = TranscludedTrees::new();
        trees.insert(
            "src".into(),
            vec![plain_item("src", 1, ""), plain_item("c", 2, "")],
        );
        state.set_transcluded_trees(trees);
        state
    }

    // トランスクルージョンを選ぶと、その下に見える項目と、折りたたまれた自分の子も選ばれる。
    // エディタから始めても、範囲を広げても、ドラッグでも同じである。
    #[test]
    fn selecting_a_transclusion_takes_everything_under_it() {
        let under_t = ["c", "hidden", "own", "t"];

        let mut state = state_with_transclusion();
        let mut sel = Selection::default();
        sel.start_from_editor(&mut state, "t");
        assert_eq!(selected(&state), under_t);

        let mut state = state_with_transclusion();
        let mut sel = Selection::default();
        sel.start_from_editor(&mut state, "x");
        sel.extend_down(&mut state);
        assert_eq!(selected(&state), ["c", "hidden", "own", "t", "x"]);
        sel.extend_down(&mut state);
        assert_eq!(selected(&state), ["c", "hidden", "own", "t", "x", "y"]);

        let mut state = state_with_transclusion();
        let mut sel = Selection::default();
        sel.start_drag(&mut state, "t");
        assert_eq!(selected(&state), under_t);
    }
}
