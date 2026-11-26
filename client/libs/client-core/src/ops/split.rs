//! What Enter does in an editor: splits the item at the cursor, or puts an empty item before
//! or after it. An empty last child is outdented instead. Inside an open block it adds a
//! line to the item.
//!
//! The server splits an item in one request, and decides where the new item goes (ADR 011).
//! Inside a transclusion the new item can be on another page, so the page is read again
//! after the answer.

use crate::app::{done, log_error, run, Cmd, Event, Model};
use crate::model::{Item, ItemPosition, ItemRef, Split};
use crate::ops::edit::{
    discard_pending_edit, focus_editor, record_typing, show_parsed, typed_body,
};
use crate::ops::file_reload::{leave_the_edit, read_again_if_out_of_place, replace_page, AskedAt};
use crate::ops::level;
use crate::ops::outline_events::body_changed;
use crate::outline::tree::{
    get_subtree, has_sibling_below, position_before, transclusion_context_of,
};
use crate::outline::CursorState;
use crate::util::{char_len, split_at_char};
use crux_core::Command;

/// Handles Enter at a cursor position in an item.
pub fn handle_enter(model: &mut Model, item_id: &str, cursor: usize) -> Cmd {
    let Some(mut item) = model.cloned_item(item_id) else {
        return done();
    };
    let unsaved = model.edits.dirty_item_ids.contains(item_id);
    if let Some(typed) = typed_body(model, item_id).filter(|_| unsaved) {
        item.body = typed;
    }
    let (before, after) = split_at_char(&item.body, cursor);
    if flatmarkdown::has_unclosed_block(before) {
        // Not an edit that moves to another editor, so no keys are held
        let body = format!("{before}\n{after}");
        model
            .state
            .set_focused_item(item_id, Some(char_len(before) + 1));
        return body_changed(model, item_id, body);
    }
    // The next editor is focused after the next render; keys pressed before that are kept for it
    model.key_relay.hold_until_focus();
    let discard = discard_pending_edit(model, &item);

    if item.body.is_empty()
        && item.level > 1
        && !has_sibling_below(model.state.visible_items(), &item.id)
    {
        // Empty last child: outdent instead of creating a new item
        let outdent = level::outdent(model, &item.id);
        let id = item.id.clone();
        let focus = focus_editor(model, Some(id));
        return Command::all([discard, outdent, focus]);
    }

    let pressed_at = cursor.min(char_len(&item.body));
    let (body, cursor) = without_the_newline_before(&item.body, pressed_at);
    item.body = body;
    let split = split_on_server(model, item, cursor, pressed_at, unsaved);
    Command::all([discard, split])
}

/// The body to split and the cursor to split it at. At the start of a later line, the newline
/// before the cursor goes: kept, it would end the item above with an empty line (#927).
fn without_the_newline_before(body: &str, cursor: usize) -> (String, usize) {
    let (before, after) = split_at_char(body, cursor);
    match before.strip_suffix('\n') {
        Some(line) => (format!("{line}{after}"), cursor - 1),
        None => (body.to_string(), cursor),
    }
}

/// Asks the server to split an item at the cursor. The request carries the body the editor
/// has now, so what was typed and not saved yet is saved with it. A block still open after
/// the cursor is closed first: the server refuses a body that is not (ADR 050). The undo puts
/// the cursor back at `pressed_at`, where Enter was pressed.
fn split_on_server(
    model: &Model,
    item: Item,
    cursor: usize,
    pressed_at: usize,
    unsaved: bool,
) -> Cmd {
    // Of an item of another page, the server has to know which transclusion shows it here:
    // at the end of the last item it shows, the new item goes on this page
    let root_id = if model.state.is_own_item(&item.id) {
        None
    } else {
        showing_transclusion(model, &item).map(|root| root.id.clone())
    };
    let body = flatmarkdown::close_blocks(&item.body);
    run(move |api| async move {
        match api
            .split_item(&item.id, root_id.as_deref(), &body, cursor)
            .await
        {
            Ok(split) => api.send_event(Event::SplitDone {
                item_id: item.id,
                cursor_before: pressed_at,
                split,
            }),
            Err(e) => {
                log_error(format!("Failed to split item: {e}"));
                api.send_event(Event::SplitFailed {
                    item_id: item.id,
                    unsaved,
                });
            }
        }
    })
}

/// Shows what the server wrote for a split, records it as one undo entry, and moves the edit
/// to the new item.
pub fn show_split(model: &mut Model, item_id: &str, cursor_before: usize, split: Split) -> Cmd {
    if let Some(host_page_id) = host_page_of_transclusion(model, item_id) {
        return show_split_in_transclusion(model, item_id, cursor_before, split, host_page_id);
    }
    let keys = model.key_relay.release();
    let Some(before) = model.state.get_item_by_id(item_id) else {
        return done();
    };
    let Split { item, created } = split;
    model
        .state
        .update_item(item_id, |i| i.body = item.body.clone());
    show_parsed(model, &item);
    // The levels say where the new item is: an item above sorts before the split one, a
    // child comes right below it, and a sibling comes after all that is below it
    let position = if created.idx < item.idx {
        position_before(model.state.items(), item_id)
    } else if created.level > item.level {
        ItemPosition::after(item_id)
    } else {
        let subtree = get_subtree(model.state.items(), item_id);
        ItemPosition::after(subtree[subtree.len() - 1].id.clone())
    };
    model.state.add_item(created.clone(), &position);

    model
        .undo
        .begin("enter", Some(CursorState::new(item_id, cursor_before)));
    if before.body != item.body {
        if let Some(after) = model.state.get_item_by_id(item_id) {
            model.undo.push_update(&before, &after);
        }
    }
    model.undo.push_create(&created);
    model
        .undo
        .commit(Some(CursorState::new(&created.id, 0)), model.clock_ms);

    model.state.add_editing_item(&created.id);
    model.state.remove_editing_item(item_id);
    // The caret stays where the text was cut
    model.state.set_focused_item(&created.id, Some(0));
    model.state.order_focus(Some(&created.id));
    let reread = read_again_if_out_of_place(model, std::slice::from_ref(&created.id));
    let focus = run(move |api| async move { api.focus_editor(Some(created.id), keys).await });
    Command::all([focus, reread])
}

/// Ends the wait for the next editor after a split that did not reach the server, and
/// schedules the auto-save of the typed text the split was to carry.
pub fn split_failed(model: &mut Model, item_id: &str, unsaved: bool) -> Cmd {
    model.key_relay.release();
    match (model.cloned_item(item_id), typed_body(model, item_id)) {
        (Some(item), Some(typed)) if unsaved => record_typing(model, &item, &typed),
        _ => done(),
    }
}

/// The transclusion of this page that shows the item below itself, for an item of another
/// page.
fn showing_transclusion(model: &Model, item: &Item) -> Option<ItemRef> {
    let ctx = transclusion_context_of(item, model.state.items(), model.state.transcluded_trees())?;
    model.state.get_item_by_id(&ctx.root_item_id)
}

/// The page that shows the item through a transclusion: the page of the transclusion.
/// None for an item outside a transclusion, and for an item that is not shown any more.
pub fn host_page_of_transclusion(model: &Model, item_id: &str) -> Option<String> {
    let item = model.current_item(item_id)?;
    if model.state.is_own_item(item_id) {
        return item
            .transclude_item_id
            .as_ref()
            .map(|_| item.page_id.clone());
    }
    showing_transclusion(model, &item).map(|root| root.page_id.clone())
}

/// Records a split inside a transclusion as one undo entry, and reads the page of the
/// transclusion again. The new item is on the source page, or below the transclusion on its
/// own page: the server's version of the page says whether and where it is shown.
fn show_split_in_transclusion(
    model: &mut Model,
    item_id: &str,
    cursor_before: usize,
    split: Split,
    host_page_id: String,
) -> Cmd {
    let Split {
        item: written,
        created,
    } = split;
    // The item written to is the source item when a transclusion was split. The undo entry
    // holds that item, so that an undo sends the old body to it. The transclusion shows the
    // body of its source, and keeps a copy of it
    let before = model.state.get_item_by_id(&written.id);
    for id in [written.id.as_str(), item_id] {
        model
            .state
            .update_item(id, |i| i.body = written.body.clone());
    }
    show_parsed(model, &written);
    model
        .undo
        .begin("enter", Some(CursorState::new(item_id, cursor_before)));
    if let (Some(before), Some(after)) = (before, model.state.get_item_by_id(&written.id)) {
        if before.body != after.body {
            model.undo.push_update(&before, &after);
        }
    }
    model.undo.push_create(&created);
    model
        .undo
        .commit(Some(CursorState::new(&created.id, 0)), model.clock_ms);

    let item_id = item_id.to_string();
    let asked_at = AskedAt::now(model);
    run(move |api| async move {
        replace_page(&api, &host_page_id, asked_at).await;
        api.send_event(Event::SplitPageReloaded {
            item_id,
            created_id: created.id,
            asked_at,
        });
    })
}

/// Moves the edit to the item a split inside a transclusion made, once the page is read
/// again. An item made beside the source item of a transclusion is not on this page: the
/// edit then stays in the item that was split, at the end of the text it keeps.
///
/// An undo may have taken the split back while the page was read. The edit then stays where
/// the undo put it.
pub fn focus_after_split(
    model: &mut Model,
    item_id: &str,
    created_id: &str,
    asked_at: AskedAt,
) -> Cmd {
    if asked_at.is_overtaken(model) {
        return leave_the_edit(model);
    }
    let keys = model.key_relay.release();
    let (target, cursor) = if model.state.get_visible_item(created_id).is_some() {
        (created_id.to_string(), 0)
    } else if let Some(item) = model.current_item(item_id) {
        (item_id.to_string(), char_len(&item.body))
    } else {
        return done();
    };
    model.state.add_editing_item(&target);
    if target != item_id {
        model.state.remove_editing_item(item_id);
    }
    model.state.set_focused_item(&target, Some(cursor));
    model.state.order_focus(Some(&target));
    run(move |api| async move { api.focus_editor(Some(target), keys).await })
}
