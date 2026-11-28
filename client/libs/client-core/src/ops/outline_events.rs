//! The handlers of the outline component, ported from `outline.ts`: starting and ending
//! edits, moving between items, and what the keys of selection mode do.

use crate::app::{done, run, Cmd, EditPlace, Event, Model, MoveDirection, SelectionKey};
use crate::model::Item;
use crate::ops::edit::{leave, record_typing, save_target_id, take_dirty};
use crate::ops::{action, level, moves, undo_replay};
use crate::outline::tree::{index_of, transclusion_context_of};
use crate::util::auto_close;
use crux_core::Command;
use std::collections::HashSet;

/// The user starts to edit an item, with the cursor at `place` (ADR 057). A selection ends,
/// and so does the edit of the item edited before (`edit::leave`).
pub fn edit_requested(model: &mut Model, item_id: &str, place: EditPlace) -> Cmd {
    if model.state.is_broken_transclusion(item_id) {
        return done();
    }
    if model.state.item_selection_mode() {
        model.selection.exit(&mut model.state);
    }
    let cursor = place_in(model, item_id, place);
    let left = model
        .state
        .focused_item_id()
        .filter(|id| *id != item_id)
        .map(str::to_string);
    model.state.add_editing_item(item_id);
    model.state.set_focused_item(item_id, Some(cursor));
    match left {
        Some(left) => leave(model, &left),
        None => done(),
    }
}

/// The place in the Markdown of the item that `place` names, in characters.
fn place_in(model: &Model, item_id: &str, place: EditPlace) -> usize {
    let tags_dir = &model.config.tags_dir;
    let in_body = |at: &dyn Fn(&str) -> usize| {
        model
            .state
            .get_item_by_id(item_id)
            .map_or(0, |item| at(&item.body))
    };
    match place {
        EditPlace::Markdown(cursor) => cursor,
        EditPlace::ShownLetter(letter) => {
            in_body(&|body| crate::rich_text::touched_place(body, tags_dir, letter))
        }
        EditPlace::AfterShownText(before) => {
            in_body(&|body| crate::rich_text::place_after(body, &before))
        }
        EditPlace::Unplaced => in_body(&|body| body.chars().count()),
    }
}

/// The user typed into the editor of an item (ADR 057). An opening just typed gets its
/// closing. Not when the body got shorter, as by a delete back to an opening, or the closing
/// would come back at every delete.
pub fn typed(model: &mut Model, item_id: &str, body: String, cursor: usize) -> Cmd {
    let before = model
        .state
        .get_item_by_id(item_id)
        .map(|item| item.body.clone())
        .unwrap_or_default();
    let body = closed_typing(&before, &body, cursor).unwrap_or(body);
    body_changed(model, item_id, body)
}

/// What a text typed from `before` to `body` becomes, with the cursor at the character index
/// `cursor`: the opening just typed with its closing added, as [`auto_close`] adds it. None
/// when it stays as it is, as when it got shorter. Also for a text the shell keeps and the core
/// does not, as the dialog of a text of the atlas.
pub fn closed_typing(before: &str, body: &str, cursor: usize) -> Option<String> {
    if body.chars().count() < before.chars().count() {
        return None;
    }
    auto_close(body, cursor)
}

/// The user typed in an editor: the state shows it, the auto-save is scheduled and the
/// edit is recorded for undo.
pub fn body_changed(model: &mut Model, item_id: &str, new_body: String) -> Cmd {
    let Some(before) = model.cloned_item(item_id) else {
        return done();
    };
    let cmd = record_typing(model, &before, &new_body);
    let mut after = before.clone();
    after.body = new_body;
    model
        .undo
        .record_update("body", &before, &after, model.clock_ms);
    cmd
}

/// Arrow up or down out of an editor: saves the item being edited and opens the editor of
/// the neighbour, with the cursor at the same column.
pub fn navigate(model: &mut Model, item_id: &str, cursor: usize, delta: i32) -> Cmd {
    let items = model.state.visible_items();
    let Some(idx) = index_of(items, item_id) else {
        return done();
    };
    // A broken transclusion has no editor to move into: step past it
    let mut target = idx as i64 + delta as i64;
    while target >= 0
        && target < items.len() as i64
        && model
            .state
            .is_broken_transclusion(&items[target as usize].id)
    {
        target += delta as i64;
    }
    if target < 0 || target >= items.len() as i64 {
        return done();
    }
    let from: Item = (*items[idx]).clone();
    let to_id = items[target as usize].id.clone();
    let from_id = from.id.clone();
    let Some((edit, discard)) = take_dirty(model, &from) else {
        return switch_editor(model, &from_id, &to_id, cursor);
    };
    let save = run(move |api| async move {
        edit.save(&api).await;
        api.send_event(Event::EditorSwitched {
            from_id,
            to_id,
            cursor,
        });
    });
    Command::all([discard, save])
}

/// Moves the edit from one item to another, with the cursor at `cursor` in the new one.
pub fn switch_editor(model: &mut Model, from_id: &str, to_id: &str, cursor: usize) -> Cmd {
    model.state.add_editing_item(to_id);
    model.state.set_focused_item(to_id, Some(cursor));
    model.state.remove_editing_item(from_id);
    done()
}

/// The ids of the selected items in outline order, each resolved to the item its edits are saved to.
fn ordered_selected_ids(model: &Model) -> Vec<String> {
    let selected = model.state.selected_item_ids();
    model
        .state
        .visible_items()
        .iter()
        .filter(|b| selected.contains(&b.id))
        .map(|b| save_target_id(b))
        .collect()
}

/// A key of the whole page while items are selected. Undo and Redo also act when nothing is
/// selected or edited, as right after the selection was deleted: no editor is there to take
/// them then.
pub fn selection_key(model: &mut Model, key: SelectionKey) -> Cmd {
    let page_undo = matches!(key, SelectionKey::Undo | SelectionKey::Redo)
        && model.state.editing_item_ids().is_empty();
    if !model.state.item_selection_mode() && !page_undo {
        return done();
    }
    match key {
        SelectionKey::Escape => {
            model.selection.exit(&mut model.state);
            done()
        }
        SelectionKey::Undo => undo_replay::perform(model, undo_replay::Direction::Undo),
        SelectionKey::Redo => undo_replay::perform(model, undo_replay::Direction::Redo),
        SelectionKey::Delete => delete_selected(model),
        SelectionKey::MoveUp => moves::enqueue_move(model, "", MoveDirection::Up),
        SelectionKey::MoveDown => moves::enqueue_move(model, "", MoveDirection::Down),
        SelectionKey::Indent => indent_selected(model),
        SelectionKey::Outdent => outdent_selected(model),
        SelectionKey::ExtendUp => {
            model.selection.extend_up(&mut model.state);
            done()
        }
        SelectionKey::ExtendDown => {
            model.selection.extend_down(&mut model.state);
            done()
        }
        SelectionKey::Copy => action::copy_items(ordered_selected_ids(model)),
        SelectionKey::Cut => {
            let copy = action::copy_items(ordered_selected_ids(model));
            let delete = delete_selected(model);
            Command::all([copy, delete])
        }
    }
}

/// Deletes one item with everything under it, as a selection of it would, but without the
/// selection: when the user or the server says no, nothing is left to undo on the screen.
pub fn delete_item(model: &mut Model, item_id: &str) -> Cmd {
    let group = model.state.selection_group(item_id);
    delete(deletion_of(model, &group))
}

fn delete_selected(model: &mut Model) -> Cmd {
    delete(deletion_of(model, model.state.selected_item_ids()))
}

fn delete(ids: Vec<String>) -> Cmd {
    if ids.is_empty() {
        return done();
    }
    run(move |api| async move {
        if action::confirm_deleting_transcluded(&api, ids.clone()).await {
            action::request_delete(&api, ids).await;
        }
    })
}

/// The items to delete for the selected ones, each by its own id: every selected own item,
/// collapsed ones too, as a selected item takes its descendants with it. A selected
/// transclusion stands for itself, not for its source item. The source items it shows are
/// selected with it but left out: they belong to the source page, and only the transclusion
/// goes. Source items selected on their own are deleted on their page.
fn deletion_of(model: &Model, selected: &HashSet<String>) -> Vec<String> {
    let state = &model.state;
    let own = state.own_item_ids();
    let shown_by_selected_transclusion = |item: &Item| {
        transclusion_context_of(item, state.items(), state.transcluded_trees())
            .is_some_and(|ctx| selected.contains(&ctx.root_item_id))
    };
    let own_items = state.items().iter().filter(|b| selected.contains(&b.id));
    let source_items = state.visible_items().iter().filter(|b| {
        selected.contains(&b.id) && !own.contains(&b.id) && !shown_by_selected_transclusion(b)
    });
    own_items
        .chain(source_items)
        .map(|b| b.id.clone())
        .collect()
}

fn indent_selected(model: &mut Model) -> Cmd {
    let selected = model.state.selected_item_ids().clone();
    let all = model.state.items();
    let to_indent: Vec<_> = all
        .iter()
        .filter(|b| selected.contains(&b.id))
        .cloned()
        .collect();
    let Some(first) = to_indent.first() else {
        return done();
    };
    if index_of(all, &first.id).unwrap_or(0) == 0 {
        return done();
    }
    level::shift_levels(model, &to_indent, 1, "indent-selection", 0)
}

fn outdent_selected(model: &mut Model) -> Cmd {
    let selected = model.state.selected_item_ids().clone();
    let to_outdent: Vec<_> = model
        .state
        .items()
        .iter()
        .filter(|b| selected.contains(&b.id))
        .cloned()
        .collect();
    if to_outdent.is_empty() || to_outdent.iter().any(|b| b.level <= 1) {
        return done();
    }
    level::shift_levels(model, &to_outdent, -1, "outdent-selection", 0)
}
