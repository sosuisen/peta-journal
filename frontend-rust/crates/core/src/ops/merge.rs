//! Joins two neighbouring items into one, ported from `item-merge.service.ts`: Backspace at
//! the start of an item pulls it into the item above, Delete at the end pulls the item
//! below into it. A transclusion is never merged, and the days of a journal are not merged
//! into each other.

use crate::app::{apply, done, log_error, run, Cmd, Model};
use crate::model::Item;
use crate::ops::edit::{discard_pending_edit, focus_editor};
use crate::ops::file_reload::reload_shown_page;
use crate::outline::tree::index_of;
use crate::outline::CursorState;
use crate::util::char_len;
use crux_core::Command;

/// Backspace at the start: the item's text goes to the end of the item above, which keeps
/// the editor, and the item is deleted.
pub fn merge_into_previous(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.cloned_item(item_id) else {
        return done();
    };
    if item.transclude_item_id.is_some() {
        return done();
    }
    let items = model.state.visible_items();
    let Some(idx) = index_of(items, &item.id) else {
        return done();
    };
    if idx == 0 {
        return done();
    }
    let prev: Item = (*items[idx - 1]).clone();
    // In journal daily pages, prevent merging across page boundaries
    if item.is_daily_journal_item() && prev.page_id != item.page_id {
        return done();
    }

    let discard = discard_pending_edit(model, &item);
    let cursor_pos = char_len(&prev.body);
    let merged_body = format!("{}{}", prev.body, item.body);
    let shown = merged_body.clone();
    model.state.update_item(&prev.id, |i| i.body = shown);
    model.state.delete_item(&item.id);
    model.state.add_editing_item(&prev.id);

    let merge = run(move |api| async move {
        focus_editor(&api, Some(prev.id.clone())).await;
        // The server joins the two in one step; if it refuses, the page is shown as it has it
        let updated_prev = match api.merge_items(&item.id, &prev.id).await {
            Ok(u) => u,
            Err(e) => {
                log_error(format!("Failed to merge items: {e}"));
                return reload_shown_page(&api).await;
            }
        };
        apply(&api, "backspace merge: record", move |m| {
            m.undo
                .begin("backspace", Some(CursorState::new(&item.id, 0)));
            m.undo.push_update(&prev, &updated_prev);
            m.undo.push_delete(&item);
            m.undo
                .commit(Some(CursorState::new(&prev.id, cursor_pos)), m.clock_ms);
            done()
        });
    });
    Command::all([discard, merge])
}

/// Delete at the end: the text of the item below is appended to this item, and the item
/// below is deleted.
pub fn merge_next_into(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.cloned_item(item_id) else {
        return done();
    };
    let items = model.state.visible_items();
    let Some(idx) = index_of(items, &item.id) else {
        return done();
    };
    if idx + 1 >= items.len() {
        return done();
    }
    let next: Item = (*items[idx + 1]).clone();
    // Same limits as Backspace: a transclusion is not merged away, and journal days are not merged
    if next.transclude_item_id.is_some() {
        return done();
    }
    if item.is_daily_journal_item() && next.page_id != item.page_id {
        return done();
    }

    let discard = discard_pending_edit(model, &next);
    let cursor_pos = char_len(&item.body);
    let merged_body = format!("{}{}", item.body, next.body);
    let shown = merged_body.clone();
    model.state.update_item(&item.id, |i| i.body = shown);
    model.state.delete_item(&next.id);

    let merge = run(move |api| async move {
        let updated = match api.merge_items(&next.id, &item.id).await {
            Ok(u) => u,
            Err(e) => {
                log_error(format!("Failed to merge items: {e}"));
                return reload_shown_page(&api).await;
            }
        };
        apply(&api, "delete merge: record", move |m| {
            m.undo
                .begin("delete", Some(CursorState::new(&item.id, cursor_pos)));
            m.undo.push_update(&item, &updated);
            m.undo.push_delete(&next);
            m.undo
                .commit(Some(CursorState::new(&item.id, cursor_pos)), m.clock_ms);
            done()
        });
    });
    Command::all([discard, merge])
}
