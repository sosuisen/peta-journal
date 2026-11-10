//! One-shot commands on items, ported from `item-action.service.ts`: task state, zoom,
//! collapsing, transclusion, archiving, and deleting or copying several items at once.

use crate::app::{apply, done, log_error, run, Cmd, Event, Model};
use crate::effects::{Api, UiOp};
use crate::model::{Item, ItemPosition, Stashed};
use crate::ops::edit::{auto_save_again, save_target_id, take_unsaved};
use crate::ops::file_reload::reload_shown_page;
use crux_core::Command;

/// Cycles the task state: normal → task → running → done → normal.
/// An item that just became a task on a daily journal gets the journal date as its plan.
pub fn cycle_task(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.cloned_item(item_id) else {
        return done();
    };
    run(move |api| async move {
        let target_id = save_target_id(&item);
        let was_task = item.is_task();
        let mut updated = match api.cycle_task(&target_id).await {
            Ok(u) => u,
            Err(e) => return log_error(format!("Failed to cycle task: {e}")),
        };
        if !was_task && updated.is_task() && item.is_daily_journal_item() {
            match api
                .upsert_item_property(&target_id, "plan", "date", &item.name)
                .await
            {
                Ok(u) => updated = u,
                Err(e) => return log_error(format!("Failed to set plan: {e}")),
            }
        }
        apply(&api, "cycle task: apply", move |m| {
            let (types, properties) = (updated.types.clone(), updated.properties.clone());
            m.state.update_item(&target_id, |i| {
                i.types = types;
                i.properties = properties;
            });
            let mut after = item.clone();
            after.types = updated.types;
            after.properties = updated.properties;
            m.undo
                .record_update("cycle-task", &item, &after, m.clock_ms);
            done()
        });
    })
}

/// Shift+Wheel: a tenth more or less zoom, between 0.1 and 3.0.
pub fn zoom_step(model: &mut Model, item_id: &str, up: bool) -> Cmd {
    let Some(item) = model.current_item(item_id) else {
        return done();
    };
    let current = item.zoom();
    let delta = if up { 0.1 } else { -0.1 };
    let new_zoom = ((current + delta).clamp(0.1, 3.0) * 10.0).round() / 10.0;
    if (new_zoom - current).abs() < f64::EPSILON {
        return done();
    }
    set_zoom(model, item_id, new_zoom)
}

/// Shows and saves a zoom factor for the item.
pub fn set_zoom(model: &mut Model, item_id: &str, zoom: f64) -> Cmd {
    let zoom_text = format_zoom(zoom);
    let z = zoom_text.clone();
    model.state.update_item(item_id, |i| {
        i.properties
            .insert("zoom".into(), serde_json::Value::String(z));
    });
    let id = item_id.to_string();
    run(move |api| async move {
        if let Err(e) = api
            .upsert_item_property(&id, "zoom", "number", &zoom_text)
            .await
        {
            log_error(format!("Failed to update zoom: {e}"));
        }
    })
}

/// `String(zoom)` as JavaScript prints it: no trailing `.0`.
fn format_zoom(zoom: f64) -> String {
    if (zoom - zoom.round()).abs() < f64::EPSILON {
        format!("{}", zoom.round() as i64)
    } else {
        format!("{zoom}")
    }
}

/// Collapses or expands an item with children. The screen changes at once and is put
/// back when the save fails.
pub fn toggle_collapsed(model: &mut Model, item_id: &str) -> Cmd {
    if !model.state.has_children(item_id) {
        return done();
    }
    let Some(item) = model.cloned_item(item_id) else {
        return done();
    };
    let new_collapsed = !item.collapsed;
    model.state.toggle_collapsed(&item.id);
    model
        .state
        .update_item(&item.id, |i| i.collapsed = new_collapsed);
    run(move |api| async move {
        match api
            .update_item(&item.id, None, None, Some(new_collapsed))
            .await
        {
            Ok(_) => apply(&api, "collapse: record", move |m| {
                let mut after = item.clone();
                after.collapsed = new_collapsed;
                m.undo.record_update("collapse", &item, &after, m.clock_ms);
                done()
            }),
            Err(e) => {
                log_error(format!("Failed to toggle collapse: {e}"));
                apply(&api, "collapse: revert", move |m| {
                    m.state.toggle_collapsed(&item.id);
                    let was = item.collapsed;
                    m.state.update_item(&item.id, |i| i.collapsed = was);
                    done()
                });
            }
        }
    })
}

/// Makes the item a transclusion of the item whose id is on the clipboard.
pub fn paste_as_transclusion(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.cloned_item(item_id) else {
        return done();
    };
    run(move |api| async move {
        let pasted = api.read_clipboard_text().await;
        transclude(&api, &item, &pasted).await;
    })
}

fn looks_like_item_id(s: &str) -> bool {
    s.len() >= 10
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Makes the item a transclusion of another item, when the pasted text is an item id.
async fn transclude(api: &Api<Event>, item: &Item, pasted_text: &str) {
    let source_id = pasted_text.trim();
    if !looks_like_item_id(source_id) {
        return;
    }
    match api.set_transclusion(&item.id, source_id).await {
        Ok(updated) => show_transclusion_change(api, updated).await,
        Err(e) => log_error(format!("Failed to set transclusion: {e}")),
    }
}

/// Shows an item whose transclusion changed: the item at once, then the page again, since
/// the source tree of a transclusion comes with the page only.
pub async fn show_transclusion_change(api: &Api<Event>, updated: Item) {
    apply(api, "transclusion changed", move |m| {
        m.state.replace_item(&updated);
        done()
    });
    reload_shown_page(api).await;
}

/// Turns a transclusion back into a plain item.
pub fn remove_transclusion(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.state.get_item_by_id(item_id) else {
        return done();
    };
    let id = item.id.clone();
    run(move |api| async move {
        match api.remove_transclusion(&id).await {
            Ok(updated) => show_transclusion_change(&api, updated).await,
            Err(e) => log_error(format!("Failed to remove transclusion: {e}")),
        }
    })
}

/// Sends an item with its children to the stash of its page: under the `# Stash` heading,
/// which the server makes at the end of the page when there is none. One request, which
/// carries the text typed into the item and not saved yet (ADR 011); the screen changes when
/// the answer is there.
pub fn send_to_stash(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.cloned_item(item_id) else {
        return done();
    };
    let (body, discard) = take_unsaved(model, &item);
    let request = run(move |api| async move {
        match api.stash_item(&item.id, body.as_deref()).await {
            Ok(stashed) => api.send_event(Event::Stashed { stashed }),
            Err(e) => {
                log_error(format!("Failed to send item to the stash: {e}"));
                api.send_event(Event::StashFailed {
                    unsaved: body.map(|_| item.id),
                });
            }
        }
    });
    Command::all([discard, request])
}

/// Shows what sending an item to the stash did: the heading made at the end of the page,
/// and the items at their place under it.
pub fn show_stashed(model: &mut Model, stashed: Stashed) -> Cmd {
    let ids: Vec<String> = stashed.items.iter().map(|item| item.id.clone()).collect();
    if ids
        .iter()
        .any(|id| model.state.get_item_by_id(id).is_none())
    {
        // The screen went on to another page while the server was asked
        return done();
    }
    if stashed.created {
        let last = model.state.items().last().map(|item| item.id.clone());
        let at = last.map_or(ItemPosition::First, ItemPosition::After);
        model.state.add_item(stashed.heading.clone(), &at);
    }
    let under = ItemPosition::After(stashed.heading.id);
    model.state.replace_items(&ids, stashed.items, &under);
    done()
}

/// Schedules the auto-save of the typed text a stash was to carry, when the stash did not
/// reach the server.
pub fn stash_failed(model: &mut Model, unsaved: Option<&str>) -> Cmd {
    auto_save_again(model, unsaved)
}

/// Whether the items may be deleted: yes when no other item transcludes them, or when the
/// user says OK to a dialog that answers No by default. The deletion leaves only plain copies
/// in the transcluding items, which is hard to undo on their pages. When the server cannot
/// be asked, the deletion goes ahead, so a deletion never stops working on that account.
pub async fn confirm_deleting_transcluded(api: &Api<Event>, item_ids: Vec<String>) -> bool {
    let transcluded = match api.transcluded_items(item_ids).await {
        Ok(transcluded) => transcluded,
        Err(e) => {
            log_error(format!("Failed to ask which items are transcluded: {e}"));
            return true;
        }
    };
    transcluded.is_empty()
        || api
            .confirm_destructive(&transcluded_delete_message(transcluded.len()))
            .await
}

fn transcluded_delete_message(count: usize) -> String {
    let subject = if count == 1 {
        "1 item to delete is transcluded by another item".to_string()
    } else {
        format!("{count} items to delete are transcluded by other items")
    };
    format!("{subject}. Delete anyway? Each transclusion keeps a plain copy.")
}

/// Deletes items from the screen first and then from the server, as one undo entry.
pub fn delete_items(model: &mut Model, item_ids: &[String]) -> Cmd {
    if item_ids.is_empty() {
        return done();
    }
    let deleted: Vec<Item> = item_ids
        .iter()
        .filter_map(|id| model.state.get_item_by_id(id))
        .map(|b| (*b).clone())
        .collect();
    model.undo.begin("delete-selection", None);
    for b in &deleted {
        model.undo.push_delete(b);
    }
    model.undo.commit(None, model.clock_ms);
    model.state.delete_items(item_ids);
    let ids = item_ids.to_vec();
    run(move |api| async move {
        // One group: the server deletes all of them or none, and copies what a transclusion
        // showed before any of it goes. If it refuses, the page is shown as the server has it
        if let Err(e) = api.delete_items(ids).await {
            log_error(format!("Failed to delete selected items: {e}"));
            reload_shown_page(&api).await;
        }
    })
}

/// Copies items to the system clipboard as tree markdown (and HTML made from it), and keeps
/// the flat markdown so that a paste back into this app keeps the structure.
pub fn copy_items(item_ids: Vec<String>) -> Cmd {
    if item_ids.is_empty() {
        return done();
    }
    run(move |api| async move {
        match api.export_tree_markdown(item_ids).await {
            Ok(result) => {
                api.notify(UiOp::CopyMarkdown {
                    markdown: result.tree_markdown.clone(),
                });
                apply(&api, "copy: remember", move |m| {
                    m.clipboard = Some((result.flat_markdown, result.tree_markdown));
                    done()
                });
            }
            Err(e) => {
                api.notify(UiOp::CopyFailed);
                log_error(format!("Failed to copy selected items: {e}"));
            }
        }
    })
}
