//! Indents and outdents items, ported from `level-shift.service.ts`. The change is shown at
//! once and saved as a relative level shift, so key presses that overtake each other still
//! add up on the server.

use crate::app::{apply, done, log_error, run, Cmd, Model};
use crate::model::{Item, ItemPosition, ItemRef};
use crate::ops::file_reload;
use crate::outline::tree::{
    can_indent, get_subtree, index_of, last_descendant_outside, parent_index_of,
    transclusion_context_of,
};
use std::collections::HashSet;

/// Indents an item and its children by one level, when the item above allows it.
pub fn indent(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.current_item(item_id) else {
        return done();
    };
    let items = if model.state.is_own_item(&item.id) {
        model.state.items()
    } else {
        model.state.visible_items()
    };
    if !can_indent(items, &item.id) {
        return done();
    }
    let t_ctx =
        transclusion_context_of(&item, model.state.items(), model.state.transcluded_trees());
    if let Some(t_ctx) = t_ctx {
        let group = get_subtree(model.state.visible_items(), &item.id);
        return shift_levels(model, &group, 1, "indent", t_ctx.level_offset);
    }
    let group = model.state.get_item_with_children(&item.id);
    shift_levels(model, &group, 1, "indent", 0)
}

/// Outdents an item and its children by one level.
///
/// - If the previous item is at level-1, outdent in place (both modes).
/// - Otherwise, find the parent (an item above at level-1):
///   - logical outdent on: move the group after the parent's last child, then outdent.
///     Siblings below stay with the original parent.
///   - logical outdent off: outdent in place. Siblings below do not move but their
///     parent effectively changes to the outdented item.
pub fn outdent(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.current_item(item_id) else {
        return done();
    };
    if item.level <= 1 {
        return done();
    }
    let t_ctx =
        transclusion_context_of(&item, model.state.items(), model.state.transcluded_trees());
    if let Some(t_ctx) = &t_ctx {
        // Can't go to the same level as the transclusion root or above it
        if item.level <= t_ctx.root_display_level + 1 {
            return done();
        }
        if !model.state.is_own_item(&item.id) {
            let group = get_subtree(model.state.visible_items(), &item.id);
            return shift_levels(model, &group, -1, "outdent", t_ctx.level_offset);
        }
    }

    let items = model.state.items();
    let Some(idx) = index_of(items, &item.id) else {
        return done();
    };
    let prev_level = if idx > 0 {
        Some(items[idx - 1].level)
    } else {
        None
    };
    let Some(parent_idx) = parent_index_of(items, idx) else {
        return done();
    };
    let parent: Item = (*items[parent_idx]).clone();
    let group = model.state.get_item_with_children(&item.id);
    let needs_move = prev_level != Some(item.level - 1);

    if needs_move && model.config.logical_outdent {
        outdent_below_parent(model, &item, group, parent)
    } else {
        shift_levels(model, &group, -1, "outdent", 0)
    }
}

/// Logical outdent: the group is placed after the parent's last child and then outdented,
/// so the siblings below it stay with the original parent.
fn outdent_below_parent(model: &mut Model, item: &Item, group: Vec<ItemRef>, parent: Item) -> Cmd {
    let group_ids: Vec<String> = group.iter().map(|b| b.id.clone()).collect();
    let excluded: HashSet<String> = group_ids.iter().cloned().collect();
    let last_child_id = last_descendant_outside(model.state.items(), &parent.id, &excluded);
    let before: Vec<Item> = group.iter().map(|b| (**b).clone()).collect();
    for b in &group {
        model.state.update_item(&b.id, |i| i.level -= 1);
    }
    let page_id = item.page_id.clone();
    run(move |api| async move {
        if let Err(e) = api.shift_item_levels(group_ids.clone(), -1).await {
            log_error(format!("Failed to change item levels: {e}"));
        }
        if last_child_id != parent.id {
            let position = ItemPosition::after(last_child_id.clone());
            match api
                .move_items(group_ids.clone(), &page_id, position.clone(), None)
                .await
            {
                Ok(moved) => apply(&api, "logical outdent: moved", move |m| {
                    m.state.replace_items(&group_ids, moved.clone(), &position);
                    m.undo
                        .record_updates("outdent", before.iter().zip(&moved), m.clock_ms);
                    done()
                }),
                Err(e) => log_error(format!("Failed to move outdented items: {e}")),
            }
        } else {
            apply(&api, "logical outdent: record", move |m| {
                m.undo.begin("outdent", None);
                for b in &before {
                    let mut after = b.clone();
                    after.level -= 1;
                    m.undo.push_update(b, &after);
                }
                m.undo.commit(None, m.clock_ms);
                done()
            });
        }
    })
}

/// Changes the level of a group of items on screen, records the change for undo and saves it.
/// `level_offset` is shown level minus stored level, for items inside a transclusion.
pub fn shift_levels(
    model: &mut Model,
    group: &[ItemRef],
    delta: i32,
    label: &str,
    level_offset: i32,
) -> Cmd {
    model.undo.begin(label, None);
    for b in group {
        let mut after = (**b).clone();
        after.level += delta;
        model.undo.push_update(b, &after);
        model
            .state
            .update_item(&b.id, |i| i.level = b.level - level_offset + delta);
    }
    model.undo.commit(None, model.clock_ms);
    save_level_shift(group, delta)
}

/// Saves an indent or outdent that is already shown on screen. The whole group goes in one
/// relative request, and the response is not written back to the state: a response to an
/// older key press would undo a newer one. If the save fails, the affected pages are
/// reloaded so the screen matches the server again.
fn save_level_shift(group: &[ItemRef], delta: i32) -> Cmd {
    if group.is_empty() {
        return done();
    }
    let ids: Vec<String> = group.iter().map(|b| b.id.clone()).collect();
    let page_ids: HashSet<String> = group.iter().map(|b| b.page_id.clone()).collect();
    run(move |api| async move {
        if let Err(e) = api.shift_item_levels(ids, delta).await {
            log_error(format!("Failed to change item levels: {e}"));
            apply(&api, "level shift failed: reload", move |m| {
                let cmds: Vec<Cmd> = page_ids
                    .iter()
                    .map(|p| file_reload::reload_page(m, p))
                    .collect();
                crux_core::Command::all(cmds)
            });
        }
    })
}
