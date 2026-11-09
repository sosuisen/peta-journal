//! Replays the entries recorded in the undo stacks: applies them to the outline state,
//! restores the cursor, and then brings the server up to date with one request (ADR 011). The
//! screen shows the replayed state at once; when the server refuses the request, the page is
//! read again.

use crate::app::{done, log_error, run, Cmd, Event, Model};
use crate::model::{BatchOp, Item};
use crate::ops::edit::discard_pending_edit_of_focused_item;
use crate::ops::edit::{focus_editor, save_target_id, show_parsed};
use crate::ops::file_reload::reload_shown_page;
use crate::ops::outline_events::ensure_at_least_one_item;
use crate::outline::tree::position_by_index;
use crate::outline::{CursorState, UndoEntry, UndoOp};
use crux_core::Command;

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
    for op in &ops {
        apply_to_state(model, op, dir);
    }
    let focus = restore_cursor(model, &entry, dir);

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
            model.state.replace_item(target);
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

/// Opens the editor where the cursor was before (undo) or after (redo) the entry.
/// Entries without a recorded cursor, such as body edits, keep editing the first updated item.
fn restore_cursor(model: &mut Model, entry: &UndoEntry, dir: Direction) -> Cmd {
    let Some(cursor) = cursor_of(entry, dir) else {
        model.state.remove_all_editing_items();
        return done();
    };
    model.state.add_editing_item(&cursor.item_id);
    model
        .state
        .set_focused_item(&cursor.item_id, Some(cursor.position));
    let id = cursor.item_id.clone();
    run(move |api| async move { focus_editor(&api, Some(id)).await })
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
}

/// Shows what the server answered to the batch of an undo or a redo: where it put each item,
/// how it parsed the items whose body or task state changed, and the items it made again.
pub fn show_synced(
    model: &mut Model,
    items: Vec<Item>,
    reparsed: Vec<String>,
    recreated: Vec<String>,
) -> Cmd {
    for item in &items {
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
fn sync_to_server(model: &Model, ops: &[UndoOp], dir: Direction) -> Cmd {
    let mut batch = Batch::default();
    let mut put_back = Vec::new();
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
            }
            UndoOp::Create { item } | UndoOp::Delete { item } => {
                // The redo of a create and the undo of a delete put the item back
                let exists_again = matches!(op, UndoOp::Create { .. }) == (dir == Direction::Redo);
                if exists_again {
                    put_back.push(item);
                } else {
                    deletes.push(BatchOp::Delete {
                        id: item.id.clone(),
                    });
                }
            }
        }
    }
    // From the top down, after the updates: an item is made after the one above it, which
    // must be there, and at a level the one above allows
    let place_of = |item: &Item| model.state.items().iter().position(|i| i.id == item.id);
    put_back.sort_by_key(|item| place_of(item));
    for item in put_back {
        batch.push_create(model, item);
    }
    batch.ops.extend(deletes);
    run(move |api| async move {
        match api.apply_batch(batch.ops).await {
            Ok(items) => api.send_event(Event::UndoSynced {
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
