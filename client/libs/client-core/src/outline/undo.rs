//! The undo and redo stacks, ported from `undo.service.ts`.

use crate::model::Item;

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum UndoOp {
    Update { before: Item, after: Item },
    Create { item: Item },
    Delete { item: Item },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorState {
    pub item_id: String,
    pub position: usize,
}

impl CursorState {
    pub fn new(item_id: impl Into<String>, position: usize) -> CursorState {
        CursorState {
            item_id: item_id.into(),
            position,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UndoEntry {
    pub ops: Vec<UndoOp>,
    pub cursor_before: Option<CursorState>,
    pub cursor_after: Option<CursorState>,
    pub timestamp_ms: u64,
    pub label: String,
}

const MAX_STACK_SIZE: usize = 100;
const BODY_GROUP_INTERVAL_MS: u64 = 500;

#[derive(Debug, Default)]
pub struct UndoStacks {
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    pending_ops: Vec<UndoOp>,
    pending_label: String,
    pending_cursor_before: Option<CursorState>,
    pending: bool,
}

impl UndoStacks {
    pub fn begin(&mut self, label: &str, cursor_before: Option<CursorState>) {
        self.pending_ops.clear();
        self.pending_label = label.to_string();
        self.pending_cursor_before = cursor_before;
        self.pending = true;
    }

    /// Records one entry that changes an item, in one step.
    pub fn record_update(&mut self, label: &str, before: &Item, after: &Item, now_ms: u64) {
        self.record_updates(label, [(before, after)], now_ms);
    }

    /// Records one entry that changes several items, in one step.
    pub fn record_updates<'a>(
        &mut self,
        label: &str,
        changes: impl IntoIterator<Item = (&'a Item, &'a Item)>,
        now_ms: u64,
    ) {
        self.begin(label, None);
        for (before, after) in changes {
            self.push_update(before, after);
        }
        self.commit(None, now_ms);
    }

    pub fn push_update(&mut self, before: &Item, after: &Item) {
        self.pending_ops.push(UndoOp::Update {
            before: before.clone(),
            after: after.clone(),
        });
    }

    pub fn push_create(&mut self, item: &Item) {
        self.pending_ops.push(UndoOp::Create { item: item.clone() });
    }

    pub fn push_delete(&mut self, item: &Item) {
        self.pending_ops.push(UndoOp::Delete { item: item.clone() });
    }

    /// Closes the entry begun with `begin`. Body edits of the same item within half a
    /// second are merged into one entry.
    pub fn commit(&mut self, cursor_after: Option<CursorState>, now_ms: u64) {
        if !self.pending || self.pending_ops.is_empty() {
            self.pending = false;
            return;
        }

        if self.pending_label == "body" {
            if let Some(top) = self.undo_stack.last_mut() {
                let same_item = match (top.ops.as_slice(), self.pending_ops.as_slice()) {
                    ([UndoOp::Update { after, .. }], [UndoOp::Update { before, .. }]) => {
                        after.id == before.id
                    }
                    _ => false,
                };
                if top.label == "body"
                    && now_ms.saturating_sub(top.timestamp_ms) < BODY_GROUP_INTERVAL_MS
                    && same_item
                {
                    if let (
                        Some(UndoOp::Update {
                            after: top_after, ..
                        }),
                        Some(UndoOp::Update {
                            after: new_after, ..
                        }),
                    ) = (top.ops.first_mut(), self.pending_ops.pop())
                    {
                        *top_after = new_after;
                    }
                    top.cursor_after = cursor_after;
                    top.timestamp_ms = now_ms;
                    self.pending_ops.clear();
                    self.pending = false;
                    self.redo_stack.clear();
                    return;
                }
            }
        }

        let entry = UndoEntry {
            ops: std::mem::take(&mut self.pending_ops),
            cursor_before: self.pending_cursor_before.take(),
            cursor_after,
            timestamp_ms: now_ms,
            label: std::mem::take(&mut self.pending_label),
        };
        self.push_capped(entry, true);
        self.redo_stack.clear();
        self.pending = false;
    }

    pub fn abort(&mut self) {
        self.pending_ops.clear();
        self.pending = false;
    }

    pub fn is_pending(&self) -> bool {
        self.pending
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn pop_undo(&mut self) -> Option<UndoEntry> {
        self.undo_stack.pop()
    }

    pub fn pop_redo(&mut self) -> Option<UndoEntry> {
        self.redo_stack.pop()
    }

    pub fn push_redo(&mut self, entry: UndoEntry) {
        self.push_capped(entry, false);
    }

    pub fn push_undo(&mut self, entry: UndoEntry) {
        self.push_capped(entry, true);
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.pending_ops.clear();
        self.pending = false;
    }

    fn push_capped(&mut self, entry: UndoEntry, undo: bool) {
        let stack = if undo {
            &mut self.undo_stack
        } else {
            &mut self.redo_stack
        };
        stack.push(entry);
        if stack.len() > MAX_STACK_SIZE {
            stack.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::plain_item;

    // 同じ項目への本文編集が短い間隔で続くと、一つの undo エントリにまとめられる。
    #[test]
    fn groups_quick_body_edits() {
        let mut undo = UndoStacks::default();
        let a0 = plain_item("a", 1, "");
        let a1 = plain_item("a", 1, "h");
        let a2 = plain_item("a", 1, "hi");
        undo.begin("body", None);
        undo.push_update(&a0, &a1);
        undo.commit(None, 1000);
        undo.begin("body", None);
        undo.push_update(&a1, &a2);
        undo.commit(None, 1200);
        let entry = undo.pop_undo().unwrap();
        assert!(undo.pop_undo().is_none());
        match &entry.ops[0] {
            UndoOp::Update { before, after } => {
                assert_eq!(before.body, "");
                assert_eq!(after.body, "hi");
            }
            _ => panic!(),
        }
    }

    // 本文編集の間隔が結合の間隔を超えると、別々の undo エントリになる。
    #[test]
    fn separate_entries_after_the_interval() {
        let mut undo = UndoStacks::default();
        let a0 = plain_item("a", 1, "");
        let a1 = plain_item("a", 1, "h");
        undo.begin("body", None);
        undo.push_update(&a0, &a1);
        undo.commit(None, 1000);
        undo.begin("body", None);
        undo.push_update(&a1, &a0);
        undo.commit(None, 2000);
        assert!(undo.pop_undo().is_some());
        assert!(undo.pop_undo().is_some());
        assert!(!undo.can_undo());
    }

    // 操作のないコミットは undo スタックに積まれず、保留状態も残らない。
    #[test]
    fn empty_commit_is_dropped() {
        let mut undo = UndoStacks::default();
        undo.begin("x", None);
        undo.commit(None, 0);
        assert!(!undo.can_undo());
        assert!(!undo.is_pending());
    }
}
