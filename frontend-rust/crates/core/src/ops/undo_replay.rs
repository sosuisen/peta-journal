//! Replays the entries recorded in the undo stacks, ported from `undo-replay.service.ts`:
//! applies them to the outline state, restores the cursor, and then brings the server up to
//! date in the background. Server errors are logged; the screen already shows the replayed
//! state.

use crate::app::{apply, done, log_error, run, Cmd, Model};
use crate::model::{Item, ItemPosition};
use crate::ops::edit::discard_pending_edit_of_focused_item;
use crate::ops::edit::{focus_editor, show_parsed};
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
    let sync = sync_to_server(ops, dir);
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

/// Sends the replayed ops to the server, in the order they were applied.
fn sync_to_server(ops: Vec<UndoOp>, dir: Direction) -> Cmd {
    run(move |api| async move {
        // The deletes go last, as one group: a parent deleted on its own is refused
        let mut to_delete = Vec::new();
        for op in ops {
            let result = match &op {
                UndoOp::Update { before, after } => {
                    let target = if dir == Direction::Undo {
                        before
                    } else {
                        after
                    };
                    match api
                        .update_item(
                            &target.id,
                            Some(target.body.clone()),
                            Some(target.level),
                            Some(target.collapsed),
                        )
                        .await
                    {
                        Ok(updated) => {
                            // An entry that kept the body also keeps its parse: the task state
                            // is not replayed to the server, so its answer has the old types
                            let body_changed = before.body != after.body;
                            apply(&api, "undo sync: answer", move |m| {
                                if body_changed {
                                    show_parsed(m, &updated);
                                }
                                m.state.update_item(&updated.id, |i| {
                                    i.idx = updated.idx;
                                    i.level = updated.level;
                                });
                                done()
                            });
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
                UndoOp::Create { item } | UndoOp::Delete { item } => {
                    let exists_again =
                        matches!(op, UndoOp::Create { .. }) == (dir == Direction::Redo);
                    if exists_again {
                        recreate_on_server(&api, item).await
                    } else {
                        to_delete.push(item.id.clone());
                        Ok(())
                    }
                }
            };
            if let Err(e) = result {
                log_error(format!("Failed to sync undo/redo to server: {e}"));
                return;
            }
        }
        if !to_delete.is_empty() {
            if let Err(e) = api.delete_items(to_delete).await {
                log_error(format!("Failed to sync undo/redo to server: {e}"));
                reload_shown_page(&api).await;
            }
        }
    })
}

/// Creates the item of a create/delete op again on the server and shows the server's item.
async fn recreate_on_server(
    api: &crate::effects::Api<crate::app::Event>,
    item: &Item,
) -> Result<(), crate::effects::ApiError> {
    let it = item.clone();
    let position: ItemPosition =
        crate::ops::edit::read(api, move |m| position_by_index(m.state.items(), &it)).await;
    let new_item = api
        .create_item(
            &item.page_id,
            position.clone(),
            item.level,
            &item.body,
            None,
        )
        .await?;
    let old_id = item.id.clone();
    apply(api, "undo sync: recreated", move |m| {
        m.state.delete_item(&old_id);
        m.state.add_item(new_item, &position);
        done()
    });
    Ok(())
}
