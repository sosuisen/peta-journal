//! The handlers of the outline component, ported from `outline.ts`: starting and ending
//! edits, moving between items, and what the keys of selection mode do.

use crate::app::{apply, done, run, Cmd, Model, MoveDirection, SelectionKey};
use crate::effects::UiOp;
use crate::model::{Item, ItemPosition};
use crate::ops::edit::{focus_editor, record_typing, save_item_if_dirty, save_target_id, Handover};
use crate::ops::{action, level, moves, undo_replay};
use crate::outline::tree::index_of;
use crux_core::Command;

/// The user clicked an item to start editing it. Ignored in selection mode.
pub fn edit_requested(model: &mut Model, item_id: &str, cursor: usize) -> Cmd {
    if model.state.item_selection_mode() || model.state.is_broken_transclusion(item_id) {
        return done();
    }
    let current = model
        .state
        .focused_item_id()
        .filter(|id| *id != item_id)
        .and_then(|id| model.state.get_item_by_id(id))
        .map(|i| (*i).clone());
    let save = match current {
        Some(current) => run(move |api| async move {
            let _handover = Handover::begin(&api);
            let _ = save_item_if_dirty(&api, &current).await;
        }),
        None => done(),
    };
    model.state.add_editing_item(item_id);
    model.state.set_focused_item(item_id, Some(cursor));
    save
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
    let target = idx as i64 + delta as i64;
    if target < 0 || target >= items.len() as i64 {
        return done();
    }
    let from: Item = (*items[idx]).clone();
    let to_id = items[target as usize].id.clone();
    run(move |api| async move {
        let _ = save_item_if_dirty(&api, &from).await;
        apply(&api, "navigate: switch editor", move |m| {
            m.state.add_editing_item(&to_id);
            m.state.set_focused_item(&to_id, Some(cursor));
            m.state.remove_editing_item(&from.id);
            done()
        });
    })
}

/// A page must keep at least one item: creates an empty one when none is left.
pub fn ensure_at_least_one_item(model: &mut Model) -> Cmd {
    if !model.state.items().is_empty() {
        return done();
    }
    let Some(page_id) = model.shown_page_id() else {
        return done();
    };
    run(move |api| async move {
        if let Ok(new_item) = api
            .create_item(&page_id, ItemPosition::First, 1, "", None)
            .await
        {
            let id = new_item.id.clone();
            let focus_id = id.clone();
            apply(&api, "ensure one item", move |m| {
                m.state.add_item(new_item, &ItemPosition::First);
                m.state.set_focused_item(&id, None);
                done()
            });
            focus_editor(&api, Some(focus_id)).await;
        }
    })
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

pub fn selection_key(model: &mut Model, key: SelectionKey) -> Cmd {
    if !model.state.item_selection_mode() {
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
            if selection_has_transclusion(model) {
                return refuse_transclusion_delete();
            }
            let copy = action::copy_items(ordered_selected_ids(model));
            let delete = delete_selected(model);
            Command::all([copy, delete])
        }
    }
}

fn delete_selected(model: &mut Model) -> Cmd {
    if selection_has_transclusion(model) {
        return refuse_transclusion_delete();
    }
    let ids = ordered_selected_ids(model);
    if ids.is_empty() {
        return done();
    }
    model.selection.exit(&mut model.state);
    let delete = action::delete_items(model, &ids);
    let ensure = ensure_at_least_one_item(model);
    Command::all([delete, ensure])
}

/// Whether a selected item is a transclusion. The selected ids stand for the source items,
/// so deleting the selection would delete the sources on their own pages.
fn selection_has_transclusion(model: &Model) -> bool {
    let selected = model.state.selected_item_ids();
    model
        .state
        .visible_items()
        .iter()
        .any(|b| selected.contains(&b.id) && b.transclude_item_id.is_some())
}

/// Keeps the selection, so the user can change it and try again.
fn refuse_transclusion_delete() -> Cmd {
    Command::notify_shell(UiOp::Alert(
        "The selection includes a transclusion, so nothing was deleted. Deleting it here would delete its source item."
            .into(),
    ))
    .into()
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
