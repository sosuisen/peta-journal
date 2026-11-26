//! Replays the entries recorded in the undo stacks: applies them to the outline state,
//! restores the cursor, and then brings the server up to date with one request (ADR 011). The
//! screen shows the replayed state at once; when the server refuses the request, the page is
//! read again.

use crate::app::{done, log_error, run, Cmd, Event, Model};
use crate::model::ItemExt;
use crate::model::{BatchOpInput, Item, ItemPosition, ItemRef, TaskState};
use crate::ops::action::show_empty_item;
use crate::ops::edit::discard_pending_edit_of_focused_item;
use crate::ops::edit::{begin_handover, save_target_id, show_parsed};
use crate::ops::file_reload::read_again_if_out_of_place;
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
    fn begin(&mut self, ops: &[BatchOpInput]) -> u64 {
        self.sent += 1;
        for op in ops {
            let ids = match op {
                BatchOpInput::Create { id, .. }
                | BatchOpInput::Update { id, .. }
                | BatchOpInput::Delete { id } => std::slice::from_ref(id),
                BatchOpInput::Move { ids, .. } => ids.as_slice(),
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

/// Undoes or redoes the most recent entry, then makes sure the page keeps one item. An undo
/// pressed while the answer of an operation is on its way waits for that answer: the
/// operation is recorded for undo only when its answer is there, and the undo is meant for it.
pub fn perform(model: &mut Model, dir: Direction) -> Cmd {
    if model.undo.is_pending() {
        return done();
    }
    if dir == Direction::Undo && answer_awaited(model) {
        model.held_undos += 1;
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
    Command::all([discard, focus, sync])
}

/// True while an operation that is recorded for undo when its answer is there waits for it.
fn answer_awaited(model: &Model) -> bool {
    model.move_in_progress || model.task_cycles_awaited > 0
}

/// Runs the undos that waited for an answer, once no answer is awaited any longer.
pub fn replay_held_undos(model: &mut Model) -> Cmd {
    if answer_awaited(model) {
        return done();
    }
    let held = std::mem::take(&mut model.held_undos);
    Command::all((0..held).map(|_| perform(model, Direction::Undo)))
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
                model.state.set_collapsed(&target.id, target.collapsed);
            }
            // The row of a transclusion root shows the body of its source, which the typing
            // changed too (`edit::record_typing`). The root holds a copy of that body
            let source = save_target_id(target);
            if source != target.id {
                model.state.set_body(&source, &target.body);
            }
        }
        UndoOp::Create { item } => {
            if dir == Direction::Undo {
                model.state.delete_item(&item.id);
            } else {
                show_again(model, item);
            }
        }
        UndoOp::Delete { item } => {
            if dir == Direction::Undo {
                show_again(model, item);
            } else {
                model.state.delete_item(&item.id);
            }
        }
    }
}

/// Shows an item that the replay makes again, where its index sorts it among the items of
/// this page. An item of a page that only a transclusion shows is left out: this page does not
/// hold the order of the source page, which is read again after the answer.
fn show_again(model: &mut Model, item: &Item) {
    if is_of_a_transclusion(model, item) {
        return;
    }
    let position = position_by_index(model.state.items(), item);
    model.state.add_item(item.clone(), &position);
}

/// True when the item is of a page that this page shows only through a transclusion.
fn is_of_a_transclusion(model: &Model, item: &Item) -> bool {
    let on_page = |i: &ItemRef| i.page_id == item.page_id;
    !model.state.items().iter().any(on_page)
        && model
            .state
            .transcluded_trees()
            .values()
            .flatten()
            .any(on_page)
}

/// Where an item of a source page goes when the replay makes it again: after the item of
/// that page that a transclusion shows and that its index sorts after.
fn position_on_the_source_page(model: &Model, item: &Item, made: &[&Item]) -> ItemPosition {
    let shown = model.state.transcluded_trees().values().flatten();
    let above = shown
        .map(|i| &**i)
        .chain(made.iter().copied())
        .filter(|i| i.page_id == item.page_id && i.idx < item.idx)
        .max_by(|a, b| a.idx.cmp(&b.idx));
    above.map_or(ItemPosition::First, |i| ItemPosition::after(i.id.clone()))
}

/// True when an update moved an item of this page. A move is recorded as an update that
/// changes the index. An item inside a transclusion is left where it is: this page does not
/// hold the order of the items of the source page.
fn is_move(model: &Model, before: &Item, after: &Item) -> bool {
    before.idx != after.idx && model.state.is_own_item(&before.id)
}

/// True when an update moved an item that a transclusion shows. This page does not hold the
/// order of the source page; the item goes back after the item it followed in the tree of its
/// transclusion.
fn is_transcluded_move(model: &Model, before: &Item, after: &Item) -> bool {
    before.idx != after.idx && !model.state.is_own_item(&before.id)
}

/// Where an item that a transclusion shows goes, among the items of its tree: after the last
/// one its index sorts after, or right after the source root. The items of `run` move with it,
/// so they are not a place to go to.
fn position_in_transcluded_tree(
    model: &Model,
    item: &Item,
    run: &[String],
) -> Option<ItemPosition> {
    let tree = model
        .state
        .transcluded_trees()
        .values()
        .find(|tree| tree.iter().any(|i| i.id == item.id))?;
    let (root, rest) = tree.split_first()?;
    let above = rest
        .iter()
        .filter(|i| !run.contains(&i.id) && i.idx < item.idx)
        .max_by(|a, b| a.idx.cmp(&b.idx));
    Some(ItemPosition::after(
        above.map_or(&root.id, |i| &i.id).clone(),
    ))
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
    model.state.order_focus(Some(&id));
    let keys = model.key_relay.hand_over();
    // The editor of a row that is put in another place loses its focus
    if moves {
        begin_handover(model);
    }
    run(move |api| async move {
        api.focus_editor(Some(id), keys).await;
        if moves {
            api.send_event(Event::HandoverEnded);
        }
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
    ops: Vec<BatchOpInput>,
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
        self.ops.push(BatchOpInput::Update {
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
            self.ops.push(BatchOpInput::Update {
                id: task_target.clone(),
                body: None,
                level: None,
                collapsed: None,
                task: Some(target.task_state()),
            });
            self.reparsed.push(task_target);
        }
    }

    /// Adds the making of an item the replay put back, with the id it had, at `position`.
    fn push_create(&mut self, item: &Item, position: ItemPosition) {
        self.ops.push(BatchOpInput::Create {
            id: item.id.clone(),
            page_id: item.page_id.clone(),
            position,
            level: item.level,
            body: item.body.clone(),
            transclude_item_id: item.transclude_item_id.clone(),
            properties: item.properties.clone(),
        });
        // The making writes the body, the level, the transclusion and the properties
        let task = item.task_state();
        let has_task = task != TaskState::default();
        if item.collapsed || has_task {
            self.ops.push(BatchOpInput::Update {
                id: item.id.clone(),
                body: None,
                level: None,
                collapsed: item.collapsed.then_some(true),
                task: has_task.then_some(task),
            });
        }
        self.recreated.push(item.id.clone());
    }

    /// Adds the move of a run of items the replay put back in their place: the run goes
    /// after the item above its first item, in one move, so that a parent and its children
    /// stay together.
    fn push_move(&mut self, model: &Model, run: Option<(Vec<String>, &Item)>) {
        let Some((ids, first)) = run else { return };
        self.ops.push(BatchOpInput::Move {
            ids,
            page_id: first.page_id.clone(),
            position: position_by_index(model.state.items(), first),
        });
    }
}

/// Shows what the server answered to the batch of an undo or a redo: where it put each item,
/// how it parsed the items whose body or task state changed, the items it made again, and
/// the empty item it made when the replay left the page without an item.
pub fn show_synced(
    model: &mut Model,
    sync: u64,
    items: Vec<Item>,
    reparsed: Vec<String>,
    recreated: Vec<String>,
    created: Vec<Item>,
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
    let ids: Vec<String> = items.into_iter().map(|item| item.id).collect();
    let reread = read_again_if_out_of_place(model, &ids);
    Command::all([show_empty_item(model, created), reread])
}

/// Sends what was replayed to the server as one request: it writes all of it or none. When
/// it writes none, the page is shown as the server has it, as the screen is then ahead of it.
fn sync_to_server(model: &mut Model, ops: &[UndoOp], dir: Direction) -> Cmd {
    let mut batch = Batch::default();
    let mut put_back = HashSet::new();
    let mut moved = HashSet::new();
    // The server deletes last, as one group: a parent deleted on its own is refused
    let mut deletes = Vec::new();
    // The items moved inside a transclusion, with the ones that move along with them
    let mut transcluded_runs: Vec<(&Item, Vec<String>)> = Vec::new();
    // The items of a source page made again, which this page does not show until it is read
    let mut made_in_transclusions: Vec<&Item> = Vec::new();
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
                if is_transcluded_move(model, before, after) {
                    // A child moved with its parent: the move of the parent takes it along
                    match transcluded_runs.last_mut() {
                        Some((first, ids)) if target.level > first.level => {
                            ids.push(target.id.clone())
                        }
                        _ => transcluded_runs.push((target, vec![target.id.clone()])),
                    }
                }
            }
            UndoOp::Create { item } | UndoOp::Delete { item } => {
                // The redo of a create and the undo of a delete put the item back
                let exists_again = matches!(op, UndoOp::Create { .. }) == (dir == Direction::Redo);
                if exists_again && is_of_a_transclusion(model, item) {
                    made_in_transclusions.push(item);
                } else if exists_again {
                    put_back.insert(item.id.as_str());
                } else {
                    deletes.push(BatchOpInput::Delete {
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
            batch.push_create(item, position_by_index(model.state.items(), item));
        } else if moved.contains(item.id.as_str()) {
            moving = Some((vec![item.id.clone()], item));
        }
    }
    batch.push_move(model, moving.take());
    for (first, ids) in &transcluded_runs {
        if let Some(position) = position_in_transcluded_tree(model, first, ids) {
            batch.ops.push(BatchOpInput::Move {
                ids: ids.clone(),
                page_id: first.page_id.clone(),
                position,
            });
        }
    }
    // From the top down, so that a child is made after its parent
    made_in_transclusions.sort_by(|a, b| a.idx.cmp(&b.idx));
    for (at, item) in made_in_transclusions.iter().enumerate() {
        let position = position_on_the_source_page(model, item, &made_in_transclusions[..at]);
        batch.push_create(item, position);
    }
    // The transclusion shows the source tree as the server has it now
    let read_again = !transcluded_runs.is_empty() || !made_in_transclusions.is_empty();
    batch.ops.extend(deletes);
    let sync = model.undo_syncs.begin(&batch.ops);
    run(move |api| async move {
        match api.apply_batch(batch.ops).await {
            Ok(batched) => {
                api.send_event(Event::UndoSynced {
                    sync,
                    items: batched.items,
                    reparsed: batch.reparsed,
                    recreated: batch.recreated,
                    created: batched.created,
                });
                if read_again {
                    api.send_event(Event::ReloadShownPage);
                }
            }
            Err(e) => {
                log_error(format!("Failed to sync undo/redo to server: {e}"));
                api.send_event(Event::ReloadShownPage);
            }
        }
    })
}
