//! The life of an edit, ported from `item-edit.service.ts` and `auto-save.service.ts`:
//! what was typed and not yet saved, how it reaches the server, and when the blur of an
//! editor may end the edit.
//!
//! Typed text goes to the server in two ways. The auto-save sends it after a short pause;
//! a handler that needs the text saved now sends it itself and cancels the auto-save, so
//! the same text is never sent twice.

use crate::app::{apply, cont, done, log_error, run, Cmd, Event, Model, PendingSave};
use crate::effects::{Api, ApiResult, TimerOp, TimerOutput};
use crate::model::Item;
use crate::util::{ensure_code_fence_closed, has_unclosed_code_fence};
use crux_core::Command;

/// How long typing may pause before the body is saved.
pub const AUTO_SAVE_MS: u32 = 300;

/// The item an edit of the shown item is saved to: a transclusion root edits its source item.
pub fn save_target_id(item: &Item) -> String {
    item.transclude_item_id
        .clone()
        .unwrap_or_else(|| item.id.clone())
}

/// The body of an item with what was typed into it and is not saved yet. A transclusion
/// shows the body of its source item, and holds the typed text itself until it is saved.
pub fn typed_body(model: &Model, item_id: &str) -> Option<String> {
    let item = model.state.get_item_by_id(item_id)?;
    Some(item.body.clone())
}

/// True while an editor is open. Dirty ids alone do not count: an editor that was closed
/// without a blur leaves its id behind while the auto-save finishes the save.
pub fn is_edit_in_progress(model: &Model) -> bool {
    !model.state.editing_item_ids().is_empty()
}

/// Records typed text: the state shows it at once and the auto-save sends it later.
pub fn record_typing(model: &mut Model, item: &Item, new_body: &str) -> Cmd {
    let body = new_body.to_string();
    model.state.update_item(&item.id, |i| i.body = body);
    model.edits.dirty_item_ids.insert(item.id.clone());
    schedule_save(model, &save_target_id(item), new_body)
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
pub fn auto_save(model: &mut Model, target: String, timer_id: u64) -> Cmd {
    let pending = match model.edits.pending_saves.get(&target) {
        Some(p) if p.timer_id == timer_id => model.edits.pending_saves.remove(&target).unwrap(),
        _ => return done(),
    };
    run(move |api| async move {
        if let Err(e) = api
            .update_body(&target, &ensure_code_fence_closed(&pending.body))
            .await
        {
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
/// the request wrote a moment later.
pub fn take_unsaved(model: &mut Model, item: &Item) -> (Option<String>, Cmd) {
    if !model.edits.dirty_item_ids.contains(&item.id) {
        return (None, done());
    }
    (
        typed_body(model, &item.id),
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

/// Takes what the server parsed from a body it was sent, because the view renders that and
/// not the body. The body stays: text typed while the answer was on its way must not go.
pub fn show_parsed(model: &mut Model, answer: &Item) {
    model.state.update_item(&answer.id, |i| {
        i.ast = answer.ast.clone();
        i.types = answer.types.clone();
        i.properties = answer.properties.clone();
    });
}

/// Saves a body to the server and shows the server's rendering of it. An open code fence
/// is closed first.
pub async fn save_body(api: &Api<Event>, item_id: &str, body: &str) -> ApiResult<Item> {
    let normalized = ensure_code_fence_closed(body);
    api.send_event(Event::BodyShown {
        item_id: item_id.to_string(),
        body: normalized.clone(),
    });
    let updated = api.update_body(item_id, &normalized).await?;
    api.send_event(Event::BodySaved {
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
pub fn show_saved_body(model: &mut Model, saved: &Item) -> Cmd {
    model
        .state
        .update_item(&saved.id, |i| i.body = saved.body.clone());
    show_parsed(model, saved);
    done()
}

/// Saves the body the shown item has in the state. When the item is a transclusion root,
/// the AST and types returned for the source are copied back to the shown item.
pub async fn save_shown_item(api: &Api<Event>, item: &Item) -> ApiResult<Option<Item>> {
    let id = item.id.clone();
    let body = read(api, move |m| {
        m.state.get_item_by_id(&id).map(|i| i.body.clone())
    })
    .await;
    let Some(body) = body else {
        return Ok(None);
    };
    let target_id = save_target_id(item);
    let updated = save_body(api, &target_id, &body).await?;
    if target_id != item.id {
        api.send_event(Event::RootParsed {
            root_id: item.id.clone(),
            source: Box::new(updated.clone()),
        });
    }
    Ok(Some(updated))
}

/// The server saved the source of a transclusion root: the root shows what the server
/// parsed from it.
pub fn show_root_parsed(model: &mut Model, root_id: &str, source: Item) -> Cmd {
    model.state.update_item(root_id, |i| {
        i.ast = source.ast;
        i.types = source.types;
    });
    done()
}

/// Saves typed text right away and closes the editor; a clean item is left alone.
pub async fn save_item_if_dirty(api: &Api<Event>, item: &Item) -> ApiResult<()> {
    let it = item.clone();
    let dirty = read_with(api, "save_item_if_dirty: check", move |m| {
        if !m.edits.dirty_item_ids.contains(&it.id) {
            return (false, done());
        }
        (true, discard_pending_edit(m, &it))
    })
    .await;
    if !dirty {
        return Ok(());
    }
    if save_shown_item(api, item).await?.is_some() {
        close_editor(api, item.id.clone());
    }
    Ok(())
}

/// Ends the edit of an item when its editor loses focus: saves typed text, or a body
/// whose code fence is still open, and switches the item to view mode. Does nothing while
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
    let close_wiki = if model.wiki.item_id.as_deref() == Some(item_id) {
        crate::ops::wiki::close(model)
    } else {
        done()
    };

    if model.edits.dirty_item_ids.contains(item_id) {
        let discard = discard_pending_edit(model, &item);
        let save = save_and_close(item);
        return Command::all([close_wiki, discard, save]);
    }

    let unclosed = model
        .state
        .get_item_by_id(item_id)
        .map(|c| has_unclosed_code_fence(&c.body))
        .unwrap_or(false);
    if unclosed {
        let save = save_and_close(item);
        return Command::all([close_wiki, save]);
    }
    model.state.remove_editing_item(item_id);
    close_wiki
}

/// Saves the shown item and closes its editor, whether or not the save went through.
fn save_and_close(item: Item) -> Cmd {
    run(move |api| async move {
        if let Err(e) = save_shown_item(&api, &item).await {
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
/// decides which item is edited next is running. Dropping the guard ends the handover.
pub struct Handover {
    api: Api<Event>,
}

impl Handover {
    /// Starts a handover from inside an async command.
    pub fn begin(api: &Api<Event>) -> Handover {
        apply(api, "handover: begin", |m| {
            m.edits.edit_handovers += 1;
            done()
        });
        Handover {
            api: Api::new(api.ctx().clone()),
        }
    }
}

impl Drop for Handover {
    fn drop(&mut self) {
        self.api.send_event(cont("handover: end", |m| {
            m.edits.edit_handovers = m.edits.edit_handovers.saturating_sub(1);
            done()
        }));
    }
}

/// Reads something from the model from inside an async command.
pub async fn read<T: Send + 'static>(
    api: &Api<Event>,
    f: impl FnOnce(&mut Model) -> T + Send + 'static,
) -> T {
    read_with(api, "read", move |m| (f(m), done())).await
}

/// Reads something from the model and runs a command on it in the same step, from inside
/// an async command.
pub async fn read_with<T: Send + 'static>(
    api: &Api<Event>,
    label: &'static str,
    f: impl FnOnce(&mut Model) -> (T, Cmd) + Send + 'static,
) -> T {
    let (tx, rx) = futures::channel::oneshot::channel::<T>();
    apply(api, label, move |m| {
        let (value, cmd) = f(m);
        let _ = tx.send(value);
        cmd
    });
    rx.await.expect("the core always answers a read")
}

/// Focuses the editor of an item once the next render is done, playing the keys that were
/// held for it; the first editor on the page when None.
pub fn focus_editor(model: &mut Model, item_id: Option<String>) -> Cmd {
    let keys = model.key_relay.release();
    run(move |api| async move { api.focus_editor(item_id, keys).await })
}
