//! Replaces the items of a page with the server's version after its markdown file changed
//! on disk, ported from `file-reload.service.ts`. A page is not replaced while the user is
//! editing, so their typing is not overwritten; it is reloaded once editing stops.

use crate::app::{done, log_error, run, Cmd, Event, Model};
use crate::effects::{Api, TimerOutput};
use crate::model::{Item, PageWithItems};
use crate::ops::edit::{is_edit_in_progress, read};

/// How long the auto-save gets to flush before pending pages are replaced.
const FLUSH_DELAY_MS: u32 = 1500;

/// Reloads one page of the outline, or remembers it when the user is editing.
/// Pages that the outline does not show are ignored.
pub fn reload_page(model: &mut Model, page_id: &str) -> Cmd {
    if !model.state.items().iter().any(|i| i.page_id == page_id) {
        return done();
    }
    if is_edit_in_progress(model) {
        model.file_reload_pending.insert(page_id.to_string());
        return done();
    }
    let page_id = page_id.to_string();
    run(move |api| async move { replace_page(&api, &page_id).await })
}

/// Replaces the items of one page with the server's version right now, in place, so the
/// other pages of a journal stay where they are. The transcluded trees are merged for the
/// same reason.
pub async fn replace_page(api: &Api<Event>, page_id: &str) {
    match api.get_page_by_id(page_id).await {
        Ok(result) => {
            api.send_event(Event::PageReplaced {
                page_id: page_id.to_string(),
                loaded: Box::new(result),
            });
        }
        Err(e) => log_error(format!("Failed to reload page {page_id}: {e}")),
    }
}

/// The server answered a page that is read again: its items take the place of the shown
/// ones, and its transcluded trees are merged.
pub fn show_replaced_page(model: &mut Model, page_id: &str, loaded: PageWithItems) -> Cmd {
    replace_page_items(model, page_id, loaded.items);
    if let Some(trees) = loaded.transcluded_trees {
        model.state.merge_transcluded_trees(trees);
    }
    done()
}

/// Puts the given items in place of the items of one page. The items of the other pages keep
/// their places. A page with no items shown gets them at the end.
pub fn replace_page_items(m: &mut Model, page_id: &str, page_items: Vec<Item>) {
    let items = m.state.items();
    let first = items.iter().position(|i| i.page_id == page_id);
    let mut next = Vec::with_capacity(items.len() + page_items.len());
    match first {
        None => next.extend(items.iter().cloned()),
        Some(first) => next.extend(items[..first].iter().cloned()),
    }
    next.extend(page_items.into_iter().map(std::sync::Arc::new));
    if let Some(first) = first {
        next.extend(
            items[first..]
                .iter()
                .filter(|i| i.page_id != page_id)
                .cloned(),
        );
    }
    m.state.set_item_refs(next);
}

/// Replaces all items with the server's version of the shown page, e.g. after a
/// transclusion changed.
pub async fn reload_shown_page(api: &Api<Event>) {
    if let Some(page_id) = read(api, |m| m.shown_page_id()).await {
        replace_page(api, &page_id).await;
    }
}

/// Once editing stops with pages waiting, gives the auto-save a moment and then reloads them.
/// Called after every event.
pub fn schedule_pending_flush(model: &mut Model) -> Cmd {
    if model.file_reload_pending.is_empty()
        || model.file_reload_scheduled
        || is_edit_in_progress(model)
    {
        return done();
    }
    model.file_reload_scheduled = true;
    let timer_id = model.next_timer_id();
    run(move |api| async move {
        if api.sleep(timer_id, FLUSH_DELAY_MS).await != TimerOutput::Fired {
            return;
        }
        api.send_event(Event::PendingReloadsDue);
    })
}

/// The moment given to the auto-save ended: the pages that waited are read again, unless
/// the user is editing again.
pub fn flush_pending(model: &mut Model) -> Cmd {
    model.file_reload_scheduled = false;
    if is_edit_in_progress(model) {
        return done();
    }
    let ids: Vec<String> = model.file_reload_pending.drain().collect();
    let cmds: Vec<Cmd> = ids.iter().map(|id| reload_page(model, id)).collect();
    crux_core::Command::all(cmds)
}
