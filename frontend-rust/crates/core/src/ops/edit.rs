//! The life of an edit, ported from `item-edit.service.ts` and `auto-save.service.ts`:
//! what was typed and not yet saved, how it reaches the server, and when the blur of an
//! editor may end the edit.
//!
//! Typed text goes to the server in two ways. The auto-save sends it after a short pause;
//! a handler that needs the text saved now sends it itself and cancels the auto-save, so
//! the same text is never sent twice.

use crate::app::{apply, cont, done, log_error, run, Cmd, Event, Model, PendingSave};
use crate::effects::{Api, ApiResult, TimerOp, TimerOutput};
use crate::model::{Item, ItemRef};
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
        apply(&api, "auto-save fired", move |m| {
            let pending = match m.edits.pending_saves.get(&target) {
                Some(p) if p.timer_id == timer_id => m.edits.pending_saves.remove(&target).unwrap(),
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
        });
    }));
    Command::all(cmds)
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

/// Sends typed text that is still waiting for the auto-save to the server right now, before
/// an operation whose server response replaces the item in the state.
pub async fn flush_pending_edit(api: &Api<Event>, item: &Item) -> ApiResult<()> {
    let item_id = item.id.clone();
    let target = save_target_id(item);
    let (tx, rx) = futures::channel::oneshot::channel::<Option<String>>();
    apply(api, "flush: take pending", move |m| {
        if !m.edits.dirty_item_ids.remove(&item_id) {
            let _ = tx.send(None);
            return done();
        }
        let pending = m.edits.pending_saves.remove(&target);
        let _ = tx.send(pending.as_ref().map(|p| p.body.clone()));
        match pending {
            Some(p) => Command::notify_shell(TimerOp::Cancel { id: p.timer_id }).into(),
            None => done(),
        }
    });
    if let Ok(Some(body)) = rx.await {
        api.update_body(&save_target_id(item), &ensure_code_fence_closed(&body))
            .await?;
    }
    Ok(())
}

/// Saves a body to the server and shows the server's rendering of it. An open code fence
/// is closed first.
pub async fn save_body(api: &Api<Event>, item_id: &str, body: &str) -> ApiResult<Item> {
    let normalized = ensure_code_fence_closed(body);
    let (id, shown) = (item_id.to_string(), normalized.clone());
    apply(api, "save_body: show", move |m| {
        m.state.update_item(&id, |i| i.body = shown);
        done()
    });
    let updated = api.update_body(item_id, &normalized).await?;
    let u = updated.clone();
    apply(api, "save_body: server answer", move |m| {
        m.state.update_item(&u.id, |i| {
            i.body = u.body;
            i.ast = u.ast;
            i.types = u.types;
            i.properties = u.properties;
        });
        done()
    });
    Ok(updated)
}

/// Saves the body the shown item has in the state. When the item is a transclusion root,
/// the AST and types returned for the source are copied back to the shown item.
pub async fn save_shown_item(api: &Api<Event>, item: &Item) -> ApiResult<Option<Item>> {
    let (tx, rx) = futures::channel::oneshot::channel::<Option<String>>();
    let id = item.id.clone();
    apply(api, "save_shown_item: read body", move |m| {
        let _ = tx.send(m.state.get_item_by_id(&id).map(|i| i.body.clone()));
        done()
    });
    let Ok(Some(body)) = rx.await else {
        return Ok(None);
    };
    let target_id = save_target_id(item);
    let updated = save_body(api, &target_id, &body).await?;
    if target_id != item.id {
        let (shown_id, u) = (item.id.clone(), updated.clone());
        apply(api, "save_shown_item: copy to root", move |m| {
            m.state.update_item(&shown_id, |i| {
                i.ast = u.ast;
                i.types = u.types;
            });
            done()
        });
    }
    Ok(Some(updated))
}

/// Saves typed text right away and closes the editor; a clean item is left alone.
pub async fn save_item_if_dirty(api: &Api<Event>, item: &Item) -> ApiResult<()> {
    let (tx, rx) = futures::channel::oneshot::channel::<bool>();
    let it = item.clone();
    apply(api, "save_item_if_dirty: check", move |m| {
        if !m.edits.dirty_item_ids.contains(&it.id) {
            let _ = tx.send(false);
            return done();
        }
        let _ = tx.send(true);
        discard_pending_edit(m, &it)
    });
    if rx.await != Ok(true) {
        return Ok(());
    }
    if save_shown_item(api, item).await?.is_some() {
        let id = item.id.clone();
        apply(api, "save_item_if_dirty: close editor", move |m| {
            m.state.remove_editing_item(&id);
            done()
        });
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
        return done();
    };
    let close_wiki = if model.wiki.item_id.as_deref() == Some(item_id) {
        crate::ops::wiki::close(model)
    } else {
        done()
    };

    if model.edits.dirty_item_ids.contains(item_id) {
        let discard = discard_pending_edit(model, &item);
        let save = run(move |api| async move {
            if let Err(e) = save_shown_item(&api, &item).await {
                log_error(format!("Failed to save item: {e}"));
            }
            let id = item.id.clone();
            apply(&api, "finish_edit: close", move |m| {
                m.state.remove_editing_item(&id);
                done()
            });
        });
        return Command::all([close_wiki, discard, save]);
    }

    let unclosed = model
        .state
        .get_item_by_id(item_id)
        .map(|c| has_unclosed_code_fence(&c.body))
        .unwrap_or(false);
    if unclosed {
        let save = run(move |api| async move {
            if let Err(e) = save_shown_item(&api, &item).await {
                log_error(format!("Failed to save item: {e}"));
            }
            let id = item.id.clone();
            apply(&api, "finish_edit: close", move |m| {
                m.state.remove_editing_item(&id);
                done()
            });
        });
        return Command::all([close_wiki, save]);
    }
    model.state.remove_editing_item(item_id);
    close_wiki
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
    let (tx, rx) = futures::channel::oneshot::channel::<T>();
    apply(api, "read", move |m| {
        let _ = tx.send(f(m));
        done()
    });
    rx.await.expect("the core always answers a read")
}

/// Reads an item as the state has it now, from inside an async command.
pub async fn read_item(api: &Api<Event>, item_id: &str) -> Option<ItemRef> {
    let id = item_id.to_string();
    read(api, move |m| m.current_item(&id)).await
}

/// Focuses the editor of an item once the next render is done, playing the keys that were
/// held for it; the first editor on the page when None.
pub async fn focus_editor(api: &Api<Event>, item_id: Option<String>) {
    let keys = read(api, |m| m.key_relay.release()).await;
    api.focus_editor(item_id, keys).await;
}
