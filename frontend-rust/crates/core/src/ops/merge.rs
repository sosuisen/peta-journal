//! Joins two neighbouring items into one, ported from `item-merge.service.ts`: Backspace at
//! the start of an item pulls it into the item above, Delete at the end pulls the item
//! below into it. A transclusion is never merged, and the days of a journal are not merged
//! into each other. An item that other items transclude is merged away only after the user
//! says OK, as a deletion of it would be.

use crate::app::Event;
use crate::app::{apply, done, log_error, run, Cmd, Model};
use crate::effects::{Api, ApiResult};
use crate::model::Item;
use crate::ops::action::confirm_deleting_transcluded;
use crate::ops::edit::{flush_pending_edit, focus_editor, show_parsed};
use crate::ops::file_reload::reload_shown_page;
use crate::ops::outline_events::edit_requested;
use crate::outline::tree::index_of;
use crate::outline::CursorState;
use crate::util::char_len;

/// The item Backspace at its start joins into the one above, and that item: None when the
/// two are not joined.
fn previous_join(model: &Model, item_id: &str) -> Option<(Item, Item)> {
    let item = model.cloned_item(item_id)?;
    if item.transclude_item_id.is_some() {
        return None;
    }
    let items = model.state.visible_items();
    let idx = index_of(items, &item.id)?;
    if idx == 0 {
        return None;
    }
    let prev: Item = (*items[idx - 1]).clone();
    // In journal daily pages, prevent merging across page boundaries
    if item.is_daily_journal_item() && prev.page_id != item.page_id {
        return None;
    }
    Some((item, prev))
}

/// The item Delete at its end takes in, and the item below it: None when the two are not
/// joined. Same limits as Backspace: a transclusion is not merged away, and journal days are
/// not merged.
fn next_join(model: &Model, item_id: &str) -> Option<(Item, Item)> {
    let item = model.cloned_item(item_id)?;
    let items = model.state.visible_items();
    let idx = index_of(items, &item.id)?;
    let next: Item = (**items.get(idx + 1)?).clone();
    if next.transclude_item_id.is_some() {
        return None;
    }
    if item.is_daily_journal_item() && next.page_id != item.page_id {
        return None;
    }
    Some((item, next))
}

/// Joins once the user agrees that the item that goes away may go. A No puts the cursor back
/// where the key was pressed, as the dialog took the focus.
fn join_when_agreed(
    goes_away: String,
    editing: String,
    cursor: usize,
    join: fn(&mut Model, &str) -> Cmd,
) -> Cmd {
    run(move |api| async move {
        if confirm_deleting_transcluded(&api, vec![goes_away]).await {
            apply(&api, "merge", move |m| join(m, &editing));
        } else {
            apply(&api, "merge: refused", move |m| {
                edit_requested(m, &editing, cursor)
            });
        }
    })
}

/// Joins `item` into `into` on the server. The server joins the bodies it has, so text still
/// waiting for the auto-save goes first. A later auto-save would overwrite the joined body.
async fn merge_on_server(api: &Api<Event>, item: &Item, into: &Item) -> ApiResult<Item> {
    flush_pending_edit(api, into).await?;
    flush_pending_edit(api, item).await?;
    api.merge_items(&item.id, &into.id).await
}

/// Backspace at the start: the item's text goes to the end of the item above, which keeps
/// the editor, and the item is deleted.
pub fn merge_into_previous(model: &mut Model, item_id: &str) -> Cmd {
    let Some((item, _)) = previous_join(model, item_id) else {
        return done();
    };
    join_when_agreed(item.id.clone(), item.id, 0, join_into_previous)
}

/// The join of [`merge_into_previous`], looked at again: the page may have changed while the
/// user was asked.
fn join_into_previous(model: &mut Model, item_id: &str) -> Cmd {
    let Some((item, prev)) = previous_join(model, item_id) else {
        return done();
    };
    let cursor_pos = char_len(&prev.body);
    let merged_body = format!("{}{}", prev.body, item.body);
    let shown = merged_body.clone();
    model.state.update_item(&prev.id, |i| i.body = shown);
    model.state.delete_item(&item.id);
    model.state.add_editing_item(&prev.id);

    run(move |api| async move {
        focus_editor(&api, Some(prev.id.clone())).await;
        // The server joins the two in one step; if it refuses, the page is shown as it has it
        let updated_prev = match merge_on_server(&api, &item, &prev).await {
            Ok(u) => u,
            Err(e) => {
                log_error(format!("Failed to merge items: {e}"));
                return reload_shown_page(&api).await;
            }
        };
        apply(&api, "backspace merge: record", move |m| {
            show_parsed(m, &updated_prev);
            m.undo
                .begin("backspace", Some(CursorState::new(&item.id, 0)));
            m.undo.push_update(&prev, &updated_prev);
            m.undo.push_delete(&item);
            m.undo
                .commit(Some(CursorState::new(&prev.id, cursor_pos)), m.clock_ms);
            done()
        });
    })
}

/// Delete at the end: the text of the item below is appended to this item, and the item
/// below is deleted.
pub fn merge_next_into(model: &mut Model, item_id: &str) -> Cmd {
    let Some((item, next)) = next_join(model, item_id) else {
        return done();
    };
    join_when_agreed(
        next.id,
        item.id.clone(),
        char_len(&item.body),
        join_next_into,
    )
}

/// The join of [`merge_next_into`], looked at again: the page may have changed while the
/// user was asked.
fn join_next_into(model: &mut Model, item_id: &str) -> Cmd {
    let Some((item, next)) = next_join(model, item_id) else {
        return done();
    };
    let cursor_pos = char_len(&item.body);
    let merged_body = format!("{}{}", item.body, next.body);
    let shown = merged_body.clone();
    model.state.update_item(&item.id, |i| i.body = shown);
    model.state.delete_item(&next.id);

    run(move |api| async move {
        let updated = match merge_on_server(&api, &next, &item).await {
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
    })
}
