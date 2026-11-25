//! The life of an edit, ported from `item-edit.service.ts` and `auto-save.service.ts`:
//! what was typed and not yet saved, how it reaches the server, and when the blur of an
//! editor may end the edit.
//!
//! Typed text goes to the server in two ways. The auto-save sends it after a short pause;
//! a handler that needs the text saved now sends it itself and cancels the auto-save, so
//! the same text is never sent twice.

use crate::app::{done, log_error, run, Cmd, Event, Model, PendingSave};
use crate::effects::{Api, ApiResult, TimerOp, TimerOutput};
use crate::model::Item;
use crux_core::Command;

/// How long typing may pause before the body is saved.
pub const AUTO_SAVE_MS: u32 = 300;

/// The item an edit of the shown item is saved to: a transclusion root edits its source item.
pub fn save_target_id(item: &Item) -> String {
    item.transclude_item_id
        .clone()
        .unwrap_or_else(|| item.id.clone())
}

/// The body of an item with what was typed into it and is not saved yet.
pub fn typed_body(model: &Model, item_id: &str) -> Option<String> {
    let item = model.state.get_item_by_id(item_id)?;
    Some(item.body.clone())
}

/// True while an editor is open. Dirty ids alone do not count: an editor that was closed
/// without a blur leaves its id behind while the auto-save finishes the save.
pub fn is_edit_in_progress(model: &Model) -> bool {
    !model.state.editing_item_ids().is_empty()
}

/// Records typed text: the state shows it at once and the auto-save sends it later. Text
/// typed into a transclusion root goes into its source item too, as the save does: the row
/// of the root shows the body of the source.
pub fn record_typing(model: &mut Model, item: &Item, new_body: &str) -> Cmd {
    let target = save_target_id(item);
    model.state.set_body(&item.id, new_body);
    if target != item.id {
        model.state.set_body(&target, new_body);
    }
    model.edits.dirty_item_ids.insert(item.id.clone());
    schedule_save(model, &target, new_body)
}

/// Marks an item as needing a save on the next edit end, e.g. after a paste changed its body.
pub fn mark_dirty(model: &mut Model, item_id: &str) {
    model.edits.dirty_item_ids.insert(item_id.to_string());
}

/// Schedules a debounced save of a body; replaces any pending save of the same item.
fn schedule_save(model: &mut Model, target_id: &str, body: &str) -> Cmd {
    let timer_id = model.next_timer_id();
    let previous = model.edits.pending_saves.insert(
        target_id.to_string(),
        PendingSave {
            body: body.to_string(),
            timer_id,
        },
    );
    let mut cmds = Vec::new();
    if let Some(prev) = previous {
        cmds.push(Command::notify_shell(TimerOp::Cancel { id: prev.timer_id }).into());
    }
    let target = target_id.to_string();
    cmds.push(run(move |api| async move {
        if api.sleep(timer_id, AUTO_SAVE_MS).await != TimerOutput::Fired {
            return;
        }
        api.send_event(Event::AutoSaveFired { target, timer_id });
    }));
    Command::all(cmds)
}

/// The pause after typing ended: the body waiting for the item `target` is saved. Nothing
/// is saved when a later edit took the place of the wait `timer_id`, or the body was saved
/// another way.
///
/// Nor is a body saved that ends inside a block that is not closed (ADR 050). The item stays
/// dirty, and the body is closed and saved when the edit ends.
pub fn auto_save(model: &mut Model, target: String, timer_id: u64) -> Cmd {
    let pending = match model.edits.pending_saves.get(&target) {
        Some(p) if p.timer_id == timer_id => model.edits.pending_saves.remove(&target).unwrap(),
        _ => return done(),
    };
    if flatmarkdown::has_unclosed_block(&pending.body) {
        return done();
    }
    run(move |api| async move {
        if let Err(e) = api.update_body(&target, &pending.body).await {
            log_error(format!("Auto-save failed: {e}"));
        }
    })
}

/// Cancels a pending save without saving it.
fn cancel_save(model: &mut Model, target_id: &str) -> Cmd {
    match model.edits.pending_saves.remove(target_id) {
        Some(p) => Command::notify_shell(TimerOp::Cancel { id: p.timer_id }).into(),
        None => done(),
    }
}

/// Drops typed text that is still waiting for the auto-save, because it is saved another way.
pub fn discard_pending_edit(model: &mut Model, item: &Item) -> Cmd {
    model.edits.dirty_item_ids.remove(&item.id);
    cancel_save(model, &save_target_id(item))
}

/// The text typed into an item and not saved yet, for a request that carries it to the
/// server with what it does (ADR 011). Its auto-save is dropped, or it would overwrite what
/// the request wrote a moment later. A block left open is closed: the server refuses a body
/// that is not (ADR 050).
pub fn take_unsaved(model: &mut Model, item: &Item) -> (Option<String>, Cmd) {
    if !model.edits.dirty_item_ids.contains(&item.id) {
        return (None, done());
    }
    (
        typed_body(model, &item.id).map(|body| flatmarkdown::close_blocks(&body)),
        discard_pending_edit(model, item),
    )
}

/// Drops the pending edit of the focused item, e.g. before an undo replaces that item.
pub fn discard_pending_edit_of_focused_item(model: &mut Model) -> Cmd {
    let Some(focused_id) = model.state.focused_item_id().map(str::to_string) else {
        return done();
    };
    if !model.edits.dirty_item_ids.contains(&focused_id) {
        return done();
    }
    let target = model
        .state
        .get_item_by_id(&focused_id)
        .map(|i| save_target_id(&i))
        .unwrap_or(focused_id.clone());
    model.edits.dirty_item_ids.remove(&focused_id);
    cancel_save(model, &target)
}

/// Schedules the auto-save of the typed text of `unsaved` again: a request that was to
/// carry it did not reach the server.
pub fn auto_save_again(model: &mut Model, unsaved: Option<&str>) -> Cmd {
    let typed = unsaved.and_then(|id| Some((model.cloned_item(id)?, typed_body(model, id)?)));
    match typed {
        Some((item, typed)) => record_typing(model, &item, &typed),
        None => done(),
    }
}

/// Takes what the server found in a body it was sent: the types and the properties. The body
/// stays: text typed while the answer was on its way must not go.
pub fn show_parsed(model: &mut Model, answer: &Item) {
    model.state.update_item(&answer.id, |i| {
        i.types = answer.types.clone();
        i.properties = answer.properties.clone();
    });
}

/// Saves a body to the server and shows the server's rendering of it. A block that is not
/// closed is closed first, as the edit of the body ends (ADR 050).
pub async fn save_body(api: &Api<Event>, item_id: &str, body: &str) -> ApiResult<Item> {
    let normalized = flatmarkdown::close_blocks(body);
    api.send_event(Event::BodyShown {
        item_id: item_id.to_string(),
        body: normalized.clone(),
    });
    let updated = api.update_body(item_id, &normalized).await?;
    api.send_event(Event::BodySaved {
        sent: normalized,
        item: Box::new(updated.clone()),
    });
    Ok(updated)
}

/// A body is on its way to the server: the item shows it as it is sent.
pub fn show_body(model: &mut Model, item_id: &str, body: String) -> Cmd {
    model.state.update_item(item_id, |i| i.body = body);
    done()
}

/// The server saved a body: the item shows the body and what the server parsed from it.
/// An item that no longer shows the body that was sent keeps what it shows: an undo, for
/// one, changed it while the answer was on its way, and wrote its own body after this one.
pub fn show_saved_body(model: &mut Model, sent: &str, saved: &Item) -> Cmd {
    if model
        .state
        .get_item_by_id(&saved.id)
        .is_some_and(|held| held.body != sent)
    {
        return done();
    }
    model
        .state
        .update_item(&saved.id, |i| i.body = saved.body.clone());
    show_parsed(model, saved);
    done()
}

/// What was typed into the shown item, as the edit leaves it. The state holds the typed body:
/// the copy of `item` may be the visible one, and a transclusion root shows the body of its
/// source there.
fn left_text(model: &Model, item: &Item) -> String {
    match model.state.get_item_by_id(&item.id) {
        Some(held) => without_blank_end(&held.body),
        None => without_blank_end(&item.body),
    }
}

/// The body without the lines of only blanks at its end. The file keeps the blank lines at
/// the start and in the middle as lines of one backslash (ADR 070), but one at the end would
/// show as a `\` in other Markdown viewers. Only when the edit ends: while typing, a line just
/// begun at the end would go.
fn without_blank_end(body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .map_or(0, |i| i + 1);
    lines[..end].join("\n")
}

/// Saves a body of the shown item. When the item is a transclusion root, the AST and types
/// returned for the source are copied back to the shown item.
async fn save_shown_body(api: &Api<Event>, item: &Item, body: &str) -> ApiResult<Item> {
    let target_id = save_target_id(item);
    let updated = save_body(api, &target_id, body).await?;
    if target_id != item.id {
        api.send_event(Event::RootParsed {
            root_id: item.id.clone(),
            source: Box::new(updated.clone()),
        });
    }
    Ok(updated)
}

/// The server saved the source of a transclusion root: the root takes the types that the
/// server found in it.
pub fn show_root_parsed(model: &mut Model, root_id: &str, source: Item) -> Cmd {
    model.state.update_item(root_id, |i| {
        i.types = source.types;
    });
    done()
}

/// The typed text of an item that is to be saved right away, taken out of the auto-save.
/// None when nothing was typed into the item: a clean item is left alone.
pub struct DirtyEdit {
    item: Item,
    body: String,
}

/// Takes the typed text of an item for a save right away, with the command that drops its
/// auto-save.
pub fn take_dirty(model: &mut Model, item: &Item) -> Option<(DirtyEdit, Cmd)> {
    if !model.edits.dirty_item_ids.contains(&item.id) {
        return None;
    }
    let discard = discard_pending_edit(model, item);
    let edit = DirtyEdit {
        item: item.clone(),
        body: left_text(model, item),
    };
    Some((edit, discard))
}

impl DirtyEdit {
    /// Saves the typed text and closes the editor of the item. The editor stays open when
    /// the save fails.
    pub async fn save(self, api: &Api<Event>) {
        if save_shown_body(api, &self.item, &self.body).await.is_ok() {
            close_editor(api, self.item.id);
        }
    }
}

/// Ends the edit of an item when its editor loses focus: saves typed text, or a body
/// with a block that is still open, and switches the item to view mode. Does nothing while
/// a handover runs or when another handler already ended the edit.
pub fn finish_edit(model: &mut Model, item_id: &str) -> Cmd {
    if model.edits.edit_handovers > 0 || !model.state.is_editing(item_id) {
        return done();
    }
    let Some(item) = model.cloned_item(item_id) else {
        // An item no longer shown has nothing to save, but its mark would hold back every
        // later reload
        model.state.remove_editing_item(item_id);
        return done();
    };
    let close_wiki = close_wiki_of(model, item_id);
    if model.edits.dirty_item_ids.contains(item_id) {
        let discard = discard_pending_edit(model, &item);
        let save = save_and_close(model, item);
        return Command::all([close_wiki, discard, save]);
    }
    Command::all([close_wiki, end_clean_edit(model, item)])
}

/// Ends the edit of an item that the edit left for another one, as `finish_edit` does, also
/// while a handover runs (ADR 057). Typed text is saved in a handover of its own: the blur of
/// the editor, which a shell may send after, does not end the edit again meanwhile.
pub fn leave(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.cloned_item(item_id) else {
        model.state.remove_editing_item(item_id);
        return done();
    };
    let close_wiki = close_wiki_of(model, item_id);
    let Some((edit, discard)) = take_dirty(model, &item) else {
        if !model.state.is_editing(item_id) {
            return close_wiki;
        }
        return Command::all([close_wiki, end_clean_edit(model, item)]);
    };
    begin_handover(model);
    let save = run(move |api| async move {
        edit.save(&api).await;
        api.send_event(Event::HandoverEnded);
    });
    Command::all([close_wiki, discard, save])
}

/// Closes the wiki link popup when it belongs to the item.
fn close_wiki_of(model: &mut Model, item_id: &str) -> Cmd {
    match model.wiki.item_id.as_deref() == Some(item_id) {
        true => crate::ops::wiki::close(model),
        false => done(),
    }
}

/// Ends the edit of an item without typed text: a body with a block that is still open is
/// saved closed, and the item is no longer marked as edited.
fn end_clean_edit(model: &mut Model, item: Item) -> Cmd {
    if flatmarkdown::has_unclosed_block(&item.body) {
        return save_and_close(model, item);
    }
    model.state.remove_editing_item(&item.id);
    done()
}

/// Saves the shown item and closes its editor, whether or not the save went through.
fn save_and_close(model: &Model, item: Item) -> Cmd {
    let body = left_text(model, &item);
    run(move |api| async move {
        if let Err(e) = save_shown_body(&api, &item, &body).await {
            log_error(format!("Failed to save item: {e}"));
        }
        close_editor(&api, item.id);
    })
}

/// Switches an item to view mode from inside an async command.
pub fn close_editor(api: &Api<Event>, item_id: String) {
    api.send_event(Event::EditorClosed { item_id });
}

/// Keeps the blur of an editor from ending an edit while a handler that saves the item and
/// decides which item is edited next is running. The handler sends
/// [`Event::HandoverEnded`] when it is done.
pub fn begin_handover(model: &mut Model) {
    model.edits.edit_handovers += 1;
}

/// A handover ended: the blur of an editor ends an edit again, once no other one runs.
pub fn end_handover(model: &mut Model) -> Cmd {
    model.edits.edit_handovers = model.edits.edit_handovers.saturating_sub(1);
    done()
}

/// The field of an ordered editor took the keyboard: the core stops holding keys, and the keys
/// held since the order go to the same editor with an order of their own (ADR 087).
pub fn editor_focused(model: &mut Model, item_id: Option<String>) -> Cmd {
    let keys = model.key_relay.release();
    if keys.is_empty() {
        return done();
    }
    model.state.order_focus(item_id.as_deref());
    run(move |api| async move { api.focus_editor(item_id, keys).await })
}

/// Focuses the editor of an item once the next render is done, playing the keys that were
/// held for it; the first editor on the page when None.
pub fn focus_editor(model: &mut Model, item_id: Option<String>) -> Cmd {
    model.state.order_focus(item_id.as_deref());
    let keys = model.key_relay.hand_over();
    run(move |api| async move { api.focus_editor(item_id, keys).await })
}
