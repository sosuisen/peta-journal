//! Pastes into an item editor. Text goes to the server in one request: the server cuts it into
//! items, joins the first piece to the text before the cursor and the last piece to the text
//! after it, and writes them all or none (ADR 011). An image is uploaded and linked at the
//! cursor.

use crate::app::{apply, done, log_error, run, Cmd, Event, Model};
use crate::model::{ItemPosition, PasteMode, Pasted};
use crate::ops::edit::{
    discard_pending_edit, focus_editor, mark_dirty, record_typing, save_body, show_parsed,
};
use crate::ops::file_reload::reload_shown_page;
use crate::outline::tree::edit_context_of;
use crate::outline::CursorState;
use crate::util::split_at_char;
use crux_core::Command;

/// Asks the server to paste text into an item. Supports plain text, tree markdown, and flat
/// markdown that was copied from this app. A raw paste keeps the text in one item as it is.
pub fn paste_text(
    model: &mut Model,
    item_id: &str,
    text: String,
    _html: String,
    cursor: usize,
    raw: bool,
) -> Cmd {
    let Some(item) = model.cloned_item(item_id) else {
        return done();
    };
    let (text, mode) = match &model.clipboard {
        _ if raw => (text, PasteMode::Raw),
        // Text copied from this app is pasted from the Flat Markdown stored with it
        Some((flat, tree)) if *tree == text => (flat.clone(), PasteMode::Flat),
        _ => (text, PasteMode::Auto),
    };
    // What was typed before the paste travels in the request. Its auto-save is dropped, or
    // it would overwrite the pasted text a moment later
    let unsaved = model.edits.dirty_item_ids.contains(item_id);
    // A transclusion root shows the body of its source item. What was typed into it is in
    // the root itself until it is saved
    let body = match model.state.get_item_by_id(item_id) {
        Some(typed) if unsaved => typed.body.clone(),
        _ => item.body.clone(),
    };
    let discard = discard_pending_edit(model, &item);
    let request = run(move |api| async move {
        match api.paste_text(&item.id, &body, cursor, &text, mode).await {
            Ok(pasted) => api.send_event(Event::Pasted {
                item_id: item.id,
                cursor_before: cursor,
                pasted,
            }),
            Err(e) => {
                log_error(format!("Failed to process paste: {e}"));
                api.send_event(Event::PasteFailed {
                    item_id: item.id,
                    unsaved,
                });
            }
        }
    });
    Command::all([discard, request])
}

/// Schedules the auto-save of the typed text a failed paste was to carry, as it did not reach
/// the server.
pub fn paste_failed(model: &mut Model, item_id: &str, unsaved: bool) -> Cmd {
    match model.cloned_item(item_id) {
        Some(item) if unsaved => record_typing(model, &item, &item.body),
        _ => done(),
    }
}

/// Shows what the server wrote for a paste, records it as one undo entry, and puts the cursor
/// right after the pasted text.
pub fn show_pasted(model: &mut Model, item_id: &str, cursor_before: usize, pasted: Pasted) -> Cmd {
    let (Some(pasted_into), Some((first, created))) =
        (model.cloned_item(item_id), pasted.items.split_first())
    else {
        return done();
    };
    // Inside a transclusion the items made belong to the source page. This page shows them
    // only when they are below the source item, so the server's version of it is read again
    let in_transclusion = edit_context_of(
        &pasted_into,
        model.state.own_item_ids(),
        model.state.transcluded_trees(),
    )
    .in_transclusion;
    model
        .undo
        .begin("paste", Some(CursorState::new(item_id, cursor_before)));
    // The item written to is the source item when a transclusion root was pasted into. The
    // undo entry holds that item, so that an undo sends the old body to it. The root shows
    // the body of its source, and keeps a copy of it
    let before = model.state.get_item_by_id(&first.id);
    for id in [first.id.as_str(), item_id] {
        model.state.update_item(id, |i| i.body = first.body.clone());
    }
    show_parsed(model, first);
    mark_dirty(model, item_id);
    if let (Some(before), Some(after)) = (before, model.state.get_item_by_id(&first.id)) {
        model.undo.push_update(&before, &after);
    }
    let mut previous_id = item_id.to_string();
    for new_item in created {
        if !in_transclusion {
            model
                .state
                .add_items(vec![new_item.clone()], &ItemPosition::after(previous_id));
        }
        let shown = model
            .state
            .get_item_by_id(&new_item.id)
            .map_or_else(|| new_item.clone(), |i| (*i).clone());
        model.undo.push_create(&shown);
        previous_id = new_item.id.clone();
    }
    model.undo.commit(None, model.clock_ms);

    let cursor = pasted.cursor;
    model
        .state
        .set_focused_item(&cursor.item_id, Some(cursor.position));
    let keys = model.key_relay.release();
    run(move |api| async move {
        if in_transclusion {
            reload_shown_page(&api).await;
        }
        api.focus_editor(Some(cursor.item_id), keys).await
    })
}

/// Uploads a pasted image and puts a link to it at the cursor.
pub fn paste_image(model: &mut Model, item_id: &str, file: u64, cursor: usize) -> Cmd {
    let Some(item) = model.cloned_item(item_id) else {
        return done();
    };
    run(move |api| async move {
        let file_name = match api.upload_asset(file).await {
            Ok(name) => name,
            Err(e) => return log_error(format!("Failed to paste image: {e}")),
        };
        let markdown = format!("![{file_name}](../assets/{file_name})");
        let (before, after) = split_at_char(&item.body, cursor);
        let new_body = format!("{before}{markdown}{after}");
        let it = item.clone();
        apply(&api, "paste image: discard pending", move |m| {
            discard_pending_edit(m, &it)
        });
        if let Err(e) = save_body(&api, &item.id, &new_body).await {
            return log_error(format!("Failed to paste image: {e}"));
        }
        let new_pos = cursor + crate::util::char_len(&markdown);
        let id = item.id.clone();
        apply(&api, "paste image: cursor", move |m| {
            mark_dirty(m, &id);
            m.state.set_focused_item(&id, Some(new_pos));
            done()
        });
        focus_editor(&api, Some(item.id.clone())).await;
    })
}
