//! Replays the entries recorded in the undo stacks: applies them to the outline state,
//! restores the cursor, and then brings the server up to date with one request (ADR 011). The
//! screen shows the replayed state at once; when the server refuses the request, the page is
//! read again.

use crate::app::{done, log_error, run, Cmd, Event, Model};
use crate::model::{BatchOp, Item};
use crate::ops::edit::discard_pending_edit_of_focused_item;
use crate::ops::edit::{focus_editor, save_target_id, show_parsed, Handover};
use crate::ops::file_reload::reload_shown_page;
use crate::ops::outline_events::ensure_at_least_one_item;
use crate::outline::tree::position_by_index;
use crate::outline::{CursorState, UndoEntry, UndoOp};
use crux_core::Command;
use std::collections::{HashMap, HashSet};

/// Which sync of an undo or a redo wrote each item last. The answers can come back in another
/// order than the requests went out, so the answer of an older sync is not shown over what a
/// later sync wrote.
#[derive(Debug, Default)]
pub struct UndoSyncs {
    sent: u64,
    last_of_item: HashMap<String, u64>,
}

impl UndoSyncs {
    /// Numbers a sync that is about to be sent, and notes the items it writes.
    fn begin(&mut self, ops: &[BatchOp]) -> u64 {
        self.sent += 1;
        for op in ops {
            let ids = match op {
                BatchOp::Create { id, .. }
                | BatchOp::Update { id, .. }
                | BatchOp::Delete { id } => std::slice::from_ref(id),
                BatchOp::Move { ids, .. } => ids.as_slice(),
            };
            for id in ids {
                self.last_of_item.insert(id.clone(), self.sent);
            }
        }
        self.sent
    }

    /// True when no sync after the given one wrote the item.
    fn wrote_last(&self, sync: u64, item_id: &str) -> bool {
        self.last_of_item.get(item_id) == Some(&sync)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Undo,
    Redo,
}

/// Undoes or redoes the most recent entry, then makes sure the page keeps one item.
pub fn perform(model: &mut Model, dir: Direction) -> Cmd {
    if model.undo.is_pending() {
        return done();
    }
    let entry = match dir {
        Direction::Undo => model.undo.pop_undo(),
        Direction::Redo => model.undo.pop_redo(),
    };
    let Some(entry) = entry else { return done() };
    // Only now: with nothing to replay, the typed text must keep its pending save
    let discard = discard_pending_edit_of_focused_item(model);

    // Undo applies the inverse ops in reverse order; redo applies the ops as recorded
    let mut ops = entry.ops.clone();
    if dir == Direction::Undo {
        ops.reverse();
    }
    let moves = ops.iter().any(|op| moves_an_item(model, op));
    for op in &ops {
        apply_to_state(model, op, dir);
    }
    let focus = restore_cursor(model, &entry, dir, moves);

    match dir {
        Direction::Undo => model.undo.push_redo(entry),
        Direction::Redo => model.undo.push_undo(entry),
    }
    let sync = sync_to_server(model, &ops, dir);
    let ensure = ensure_at_least_one_item(model);
    Command::all([discard, focus, sync, ensure])
}

/// Applies one recorded operation to the state, in the undo or the redo direction.
fn apply_to_state(model: &mut Model, op: &UndoOp, dir: Direction) {
    match op {
        UndoOp::Update { before, after } => {
            let target = if dir == Direction::Undo {
                before
            } else {
                after
            };
            if !is_move(model, before, after) {
                model.state.replace_item(target);
            } else {
                // The item goes where its index sorts it
                model.state.delete_item(&target.id);
                let position = position_by_index(model.state.items(), target);
                model.state.add_item(target.clone(), &position);
            }
            if before.collapsed != after.collapsed {
                model.state.toggle_collapsed(&target.id);
            }
        }
        UndoOp::Create { item } => {
            if dir == Direction::Undo {
                model.state.delete_item(&item.id);
            } else {
                let position = position_by_index(model.state.items(), item);
                model.state.add_item(item.clone(), &position);
            }
        }
        UndoOp::Delete { item } => {
            if dir == Direction::Undo {
                let position = position_by_index(model.state.items(), item);
                model.state.add_item(item.clone(), &position);
            } else {
                model.state.delete_item(&item.id);
            }
        }
    }
}

/// True when an update moved an item of this page. A move is recorded as an update that
/// changes the index. An item inside a transclusion is left where it is: this page does not
/// hold the order of the items of the source page.
fn is_move(model: &Model, before: &Item, after: &Item) -> bool {
    before.idx != after.idx && model.state.is_own_item(&before.id)
}

fn moves_an_item(model: &Model, op: &UndoOp) -> bool {
    matches!(op, UndoOp::Update { before, after } if is_move(model, before, after))
}

/// Opens the editor where the cursor was before (undo) or after (redo) the entry.
/// Entries without a recorded cursor, such as body edits, keep editing the first updated item.
/// When the entry moves an item, the blur of an editor does not end its edit meanwhile.
fn restore_cursor(model: &mut Model, entry: &UndoEntry, dir: Direction, moves: bool) -> Cmd {
    // The item edited before the replay may be gone now; a mark left on it would hold back
    // every later reload from another client
    model.state.remove_all_editing_items();
    let Some(cursor) = cursor_of(entry, dir) else {
        return done();
    };
    model.state.add_editing_item(&cursor.item_id);
    model
        .state
        .set_focused_item(&cursor.item_id, Some(cursor.position));
    let id = cursor.item_id.clone();
    run(move |api| async move {
        // The editor of a row that is put in another place loses its focus
        let _handover = moves.then(|| Handover::begin(&api));
        focus_editor(&api, Some(id)).await
    })
}

fn cursor_of(entry: &UndoEntry, dir: Direction) -> Option<CursorState> {
    let recorded = if dir == Direction::Undo {
        &entry.cursor_before
    } else {
        &entry.cursor_after
    };
    if let Some(c) = recorded {
        return Some(c.clone());
    }
    entry.ops.iter().find_map(|op| match op {
        UndoOp::Update { before, after } => {
            let target = if dir == Direction::Undo {
                before
            } else {
                after
            };
            Some(CursorState::new(
                &target.id,
                crate::util::char_len(&target.body),
            ))
        }
        _ => None,
    })
}

/// What an undo or a redo sends to the server in one request.
#[derive(Default)]
struct Batch {
    ops: Vec<BatchOp>,
    /// The items whose body or task state the replay changed: they take the server's parse.
    reparsed: Vec<String>,
    /// The items the batch makes again: they are shown as the server made them.
    recreated: Vec<String>,
}

impl Batch {
    /// Adds the update of a replayed item. A transclusion root shows the task state of its
    /// source item, so the task state goes there, as an update of its own.
    fn push_update(&mut self, before: &Item, after: &Item, target: &Item) {
        let task_changed = before.task_state() != after.task_state();
        let task_target = save_target_id(target);
        let own_task = task_changed && task_target == target.id;
        self.ops.push(BatchOp::Update {
            id: target.id.clone(),
            body: Some(target.body.clone()),
            level: Some(target.level),
            collapsed: Some(target.collapsed),
            task: own_task.then(|| target.task_state()),
        });
        // An entry that kept the body and the task state also keeps its parse
        if before.body != after.body || own_task {
            self.reparsed.push(target.id.clone());
        }
        if task_changed && !own_task {
            self.ops.push(BatchOp::Update {
                id: task_target.clone(),
                body: None,
                level: None,
                collapsed: None,
                task: Some(target.task_state()),
            });
            self.reparsed.push(task_target);
        }
    }

    /// Adds the making of an item the replay put back, with the id it had, where the
    /// replayed state has it.
    fn push_create(&mut self, model: &Model, item: &Item) {
        self.ops.push(BatchOp::Create {
            id: item.id.clone(),
            page_id: item.page_id.clone(),
            position: position_by_index(model.state.items(), item),
            level: item.level,
            body: item.body.clone(),
        });
        self.recreated.push(item.id.clone());
    }

    /// Adds the move of a run of items the replay put back in their place: the run goes
    /// after the item above its first item, in one move, so that a parent and its children
    /// stay together.
    fn push_move(&mut self, model: &Model, run: Option<(Vec<String>, &Item)>) {
        let Some((ids, first)) = run else { return };
        self.ops.push(BatchOp::Move {
            ids,
            page_id: first.page_id.clone(),
            position: position_by_index(model.state.items(), first),
        });
    }
}

/// Shows what the server answered to the batch of an undo or a redo: where it put each item,
/// how it parsed the items whose body or task state changed, and the items it made again.
pub fn show_synced(
    model: &mut Model,
    sync: u64,
    items: Vec<Item>,
    reparsed: Vec<String>,
    recreated: Vec<String>,
) -> Cmd {
    for item in &items {
        // A later sync wrote the item after this one: its answer is the one to show
        if !model.undo_syncs.wrote_last(sync, &item.id) {
            continue;
        }
        if recreated.contains(&item.id) {
            model.state.replace_item(item);
            continue;
        }
        if reparsed.contains(&item.id) {
            show_parsed(model, item);
        }
        model.state.update_item(&item.id, |i| {
            i.idx = item.idx.clone();
            i.level = item.level;
        });
    }
    done()
}

/// Sends what was replayed to the server as one request: it writes all of it or none. When
/// it writes none, the page is shown as the server has it, as the screen is then ahead of it.
fn sync_to_server(model: &mut Model, ops: &[UndoOp], dir: Direction) -> Cmd {
    let mut batch = Batch::default();
    let mut put_back = HashSet::new();
    let mut moved = HashSet::new();
    // The server deletes last, as one group: a parent deleted on its own is refused
    let mut deletes = Vec::new();
    for op in ops {
        match op {
            UndoOp::Update { before, after } => {
                let target = if dir == Direction::Undo {
                    before
                } else {
                    after
                };
                batch.push_update(before, after, target);
                if is_move(model, before, after) {
                    moved.insert(target.id.as_str());
                }
            }
            UndoOp::Create { item } | UndoOp::Delete { item } => {
                // The redo of a create and the undo of a delete put the item back
                let exists_again = matches!(op, UndoOp::Create { .. }) == (dir == Direction::Redo);
                if exists_again {
                    put_back.insert(item.id.as_str());
                } else {
                    deletes.push(BatchOp::Delete {
                        id: item.id.clone(),
                    });
                }
            }
        }
    }
    // What the replay put in place goes from the top down, after the updates: an item is
    // made or moved after the one above it, which must be there, at a level it allows
    let mut moving: Option<(Vec<String>, &Item)> = None;
    for item in model.state.items() {
        let joins = moved.contains(item.id.as_str())
            && moving
                .as_ref()
                .is_some_and(|(_, first)| first.page_id == item.page_id);
        if let (true, Some((ids, _))) = (joins, moving.as_mut()) {
            ids.push(item.id.clone());
            continue;
        }
        batch.push_move(model, moving.take());
        if put_back.contains(item.id.as_str()) {
            batch.push_create(model, item);
        } else if moved.contains(item.id.as_str()) {
            moving = Some((vec![item.id.clone()], item));
        }
    }
    batch.push_move(model, moving.take());
    batch.ops.extend(deletes);
    let sync = model.undo_syncs.begin(&batch.ops);
    run(move |api| async move {
        match api.apply_batch(batch.ops).await {
            Ok(items) => api.send_event(Event::UndoSynced {
                sync,
                items,
                reparsed: batch.reparsed,
                recreated: batch.recreated,
            }),
            Err(e) => {
                log_error(format!("Failed to sync undo/redo to server: {e}"));
                reload_shown_page(&api).await;
            }
        }
    })
}
