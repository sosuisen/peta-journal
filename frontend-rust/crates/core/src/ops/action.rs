//! One-shot commands on items, ported from `item-action.service.ts`: task state, zoom,
//! collapsing, transclusion, archiving, and deleting or copying several items at once.

use crate::app::{apply, done, log_error, run, Cmd, Event, Model};
use crate::effects::{Api, UiOp};
use crate::model::{Item, ItemPosition};
use crate::ops::edit::{read, save_target_id};
use crate::ops::file_reload::reload_shown_page;

/// Cycles the task state: normal → task → running → done → normal.
/// An item that just became a task on a daily journal gets the journal date as its plan.
pub fn cycle_task(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.current_item(item_id) else {
        return done();
    };
    let item: Item = (*item).clone();
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
            m.undo.begin("cycle-task", None);
            m.undo.push_update(&item, &after);
            m.undo.commit(None, m.clock_ms);
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
    let Some(item) = model.current_item(item_id) else {
        return done();
    };
    let item: Item = (*item).clone();
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
                m.undo.begin("collapse", None);
                m.undo.push_update(&item, &after);
                m.undo.commit(None, m.clock_ms);
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
    let Some(item) = model.current_item(item_id) else {
        return done();
    };
    let item: Item = (*item).clone();
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
        Ok(updated) => {
            apply(api, "transclude: apply", move |m| {
                m.state.replace_item(&updated);
                done()
            });
            reload_shown_page(api).await;
        }
        Err(e) => log_error(format!("Failed to set transclusion: {e}")),
    }
}

/// Turns a transclusion back into a plain item.
pub fn remove_transclusion(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.state.get_item_by_id(item_id) else {
        return done();
    };
    let id = item.id.clone();
    run(move |api| async move {
        match api.remove_transclusion(&id).await {
            Ok(updated) => {
                apply(&api, "remove transclusion: apply", move |m| {
                    m.state.replace_item(&updated);
                    done()
                });
                reload_shown_page(&api).await;
            }
            Err(e) => log_error(format!("Failed to remove transclusion: {e}")),
        }
    })
}

/// The heading the items sent to the stash are moved under.
const STASH_HEADING: &str = "# Stash";

/// Sends an item (and its children) to the page's stash: moves the group under the
/// `# Stash` heading, which is created at the end of the page when missing.
pub fn send_to_stash(model: &mut Model, item_id: &str) -> Cmd {
    let Some(item) = model.current_item(item_id) else {
        return done();
    };
    let item: Item = (*item).clone();
    let group_ids: Vec<String> = model
        .state
        .get_item_with_children(&item.id)
        .iter()
        .map(|b| b.id.clone())
        .collect();
    if group_ids.is_empty() {
        return done();
    }
    run(move |api| async move {
        if let Err(e) = send_to_stash_now(&api, item, group_ids).await {
            log_error(format!("Failed to send item to the stash: {e}"));
        }
    })
}

async fn send_to_stash_now(
    api: &Api<Event>,
    item: Item,
    group_ids: Vec<String>,
) -> Result<(), crate::effects::ApiError> {
    let found = read(api, |m| {
        let items = m.state.items();
        let stash = items
            .iter()
            .find(|it| it.body.trim() == STASH_HEADING)
            .map(|a| (**a).clone());
        (stash, items.last().map(|l| l.id.clone()))
    })
    .await;
    let stash = match found {
        (Some(stash), _) => stash,
        (None, Some(last_id)) => {
            let position = ItemPosition::after(last_id);
            let created = api
                .create_item(&item.page_id, position.clone(), 1, STASH_HEADING, None)
                .await?;
            let c = created.clone();
            apply(api, "stash: created", move |m| {
                m.state.add_item(c, &position);
                done()
            });
            created
        }
        (None, None) => return Ok(()),
    };
    // The newest comes first, right under the heading
    let level_delta = (stash.level + 1) - item.level;
    let position = ItemPosition::after(stash.id.clone());
    let moved = api
        .move_items(
            group_ids.clone(),
            &item.page_id,
            position.clone(),
            Some(level_delta),
        )
        .await?;
    apply(api, "stash: moved", move |m| {
        m.state.replace_items(&group_ids, moved, &position);
        done()
    });
    Ok(())
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
        let results = futures::future::join_all(ids.iter().map(|id| api.delete_item(id))).await;
        if let Some(e) = results.into_iter().find_map(Result::err) {
            log_error(format!("Failed to delete selected items: {e}"));
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
