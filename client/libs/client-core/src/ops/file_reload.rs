//! Replaces the items of a page with the server's version after its markdown file changed
//! on disk, ported from `file-reload.service.ts`. A page is not replaced while the user is
//! editing, so their typing is not overwritten; it is reloaded once editing stops.

use crate::app::{done, log_error, run, Cmd, Event, Model};
use crate::effects::{Api, TimerOutput};
use crate::model::{Item, PageWithItems};
use crate::ops::edit::is_edit_in_progress;

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
    read_again(model, page_id)
}

/// Reads one page again right now, whatever the user is doing.
pub fn read_again(model: &Model, page_id: &str) -> Cmd {
    let (page_id, asked_at) = (page_id.to_string(), AskedAt::now(model));
    run(move |api| async move { replace_page(&api, &page_id, asked_at).await })
}

/// When a page was asked for: the count of the changes made here until then.
///
/// The requests go to the server one at a time, so the answer can take a while, and the
/// user may undo in that time. The answer then holds the page as it was before the undo. On
/// 2026-11-18 such an answer put back, on the screen, a split that Ctrl+Z had just taken back.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AskedAt(u64);

impl AskedAt {
    /// Now: after every change made so far. Take it when the request is made, after the
    /// change that the request follows is recorded.
    pub fn now(model: &Model) -> AskedAt {
        AskedAt(model.undo.revision())
    }

    /// True when a change was made here after the page was asked for.
    pub fn is_overtaken(self, model: &Model) -> bool {
        self != AskedAt::now(model)
    }
}

/// What follows a page read again, when a change made here overtook the reading: the user has
/// gone on, so the edit and the cursor stay where that change left them. The keys that waited
/// for the next editor go to the editor that has the focus now.
pub fn leave_the_edit(model: &mut Model) -> Cmd {
    let keys = model.key_relay.release();
    if keys.is_empty() {
        return done();
    }
    let focused = model.state.focused_item_id().map(str::to_string);
    run(move |api| async move { api.focus_editor(focused, keys).await })
}

/// Replaces the items of one page with the server's version right now, in place, so the
/// other pages of a journal stay where they are. The transcluded trees are merged for the
/// same reason.
pub async fn replace_page(api: &Api<Event>, page_id: &str, asked_at: AskedAt) {
    match api.get_page_by_id(page_id).await {
        Ok(result) => {
            api.send_event(Event::PageReplaced {
                page_id: page_id.to_string(),
                loaded: Box::new(result),
                asked_at,
            });
        }
        Err(e) => log_error(format!("Failed to reload page {page_id}: {e}")),
    }
}

/// The server answered a page that is read again: its items take the place of the shown
/// ones, and its transcluded trees are merged.
///
/// An answer that a change made here has overtaken is not shown: the screen has the change
/// and the answer does not. The page is read once more, and that request waits behind the
/// request of the change, so its answer has the change. While the user is editing, the page
/// waits for the end of the edit, as a page changed by another client does.
pub fn show_replaced_page(
    model: &mut Model,
    page_id: &str,
    loaded: PageWithItems,
    asked_at: AskedAt,
) -> Cmd {
    if asked_at.is_overtaken(model) {
        if is_edit_in_progress(model) {
            model.file_reload_pending.insert(page_id.to_string());
            return done();
        }
        return read_again(model, page_id);
    }
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
pub fn reload_shown_page(model: &Model) -> Cmd {
    match model.shown_page_id() {
        Some(page_id) => read_again(model, &page_id),
        None => done(),
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
