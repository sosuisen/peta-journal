//! Replaces the items of a page with the server's version after its markdown file changed
//! on disk, ported from `file-reload.service.ts`. A page is read again at once, also while
//! the user is editing: only the bodies typed and not saved yet stay as they are here
//! (ADR 069).

use crate::app::{done, log_error, run, Cmd, Event, Model};
use crate::effects::Api;
use crate::model::{Item, ItemPosition, PageWithItems};
use crate::ops::edit::typed_body;
use crate::outline::tree::is_out_of_place;
use crux_core::Command;

/// Reloads one page of the outline. Pages that the outline does not show are ignored.
pub fn reload_page(model: &mut Model, page_id: &str) -> Cmd {
    if !model.state.items().iter().any(|i| i.page_id == page_id) {
        return done();
    }
    read_again(model, page_id)
}

/// Reads one page again right now, whatever the user is doing.
pub fn read_again(model: &Model, page_id: &str) -> Cmd {
    let (page_id, asked_at) = (page_id.to_string(), AskedAt::now(model));
    run(move |api| async move { replace_page(&api, &page_id, asked_at).await })
}

/// Reads the indices of a page again when one of the items does not sort between its
/// neighbours: the
/// server gave them new indices that this side does not have, and an undo places items by
/// their indices.
pub fn read_again_if_out_of_place(model: &Model, item_ids: &[String]) -> Cmd {
    let out_of_place = item_ids
        .iter()
        .filter(|id| is_out_of_place(model.state.items(), id))
        .find_map(|id| model.state.get_item_by_id(id));
    match out_of_place {
        Some(item) => read_indices_again(model, &item.page_id),
        None => done(),
    }
}

/// Reads the indices of the items of a page again. Only the indices are taken from the
/// answer: the item that was just made or moved is often being edited, and its typing is not
/// saved yet.
fn read_indices_again(model: &Model, page_id: &str) -> Cmd {
    let (page_id, asked_at) = (page_id.to_string(), AskedAt::now(model));
    run(move |api| async move {
        match api.get_page_by_id(&page_id).await {
            Ok(loaded) => api.send_event(Event::IndicesRead {
                page_id,
                loaded: Box::new(loaded),
                asked_at,
            }),
            Err(e) => log_error(format!("Failed to read the indices of {page_id}: {e}")),
        }
    })
}

/// The server answered a page read for its indices: each item shown takes its index from the
/// answer, and keeps its body and its place. An answer that a change made here has overtaken
/// is dropped: that change got its own answer, which is checked in its turn.
pub fn take_indices(
    model: &mut Model,
    page_id: &str,
    loaded: PageWithItems,
    asked_at: AskedAt,
) -> Cmd {
    if asked_at.is_overtaken(model) {
        return done();
    }
    for item in loaded.items.iter().filter(|i| i.page_id == page_id) {
        let idx = item.idx.clone();
        model
            .state
            .update_item(&item.id, move |held| held.idx = idx);
    }
    done()
}

/// When a page was asked for: the count of the changes made here until then.
///
/// The requests go to the server one at a time, so the answer can take a while, and the
/// user may undo in that time. The answer then holds the page as it was before the undo. On
/// 2026-11-18 such an answer put back, on the screen, a split that Ctrl+Z had just taken back.
// Crosses as the value it holds, as serde writes it and the generated types read it (#1209)
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), facet(transparent))]
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
    let keys = model.key_relay.hand_over();
    if keys.is_empty() {
        // No editor is focused: no answer would end the hold
        model.key_relay.release();
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
/// request of the change, so its answer has the change.
pub fn show_replaced_page(
    model: &mut Model,
    page_id: &str,
    loaded: PageWithItems,
    asked_at: AskedAt,
) -> Cmd {
    if asked_at.is_overtaken(model) {
        return read_again(model, page_id);
    }
    let edited_before = edited_body(model);
    let ids_before: Vec<String> = model.state.items().iter().map(|i| i.id.clone()).collect();
    let mut items = loaded.items;
    keep_typed_bodies(model, &mut items);
    let made_again = make_lost_typing_again(model, page_id, &mut items);
    replace_page_items(model, page_id, items);
    model
        .state
        .merge_transcluded_trees(loaded.transcluded_trees);
    if let Some(focus) = end_lost_edits(model, &ids_before) {
        return Command::all([made_again, focus]);
    }
    Command::all([made_again, reopen_changed_edit(model, edited_before)])
}

/// Puts back, among the items read again, the items of the page that have unsaved typing
/// and that the server does not have any more, and asks the server to make them again with
/// the same ids. Each goes after the item before it here, or first when that one went too.
/// The server does not take a save of an item it does not have (ADR 069).
fn make_lost_typing_again(model: &Model, page_id: &str, items: &mut Vec<Item>) -> Cmd {
    let shown: Vec<Item> = model
        .state
        .items()
        .iter()
        .filter(|i| i.page_id == page_id)
        .map(|i| (**i).clone())
        .collect();
    let mut cmds = Vec::new();
    for (at, lost) in shown.iter().enumerate() {
        if !model.edits.dirty_item_ids.contains(&lost.id) || items.iter().any(|i| i.id == lost.id) {
            continue;
        }
        let before = shown[..at]
            .iter()
            .rev()
            .find_map(|b| items.iter().position(|i| i.id == b.id));
        let (index, position) = match before {
            Some(b) => (b + 1, ItemPosition::after(items[b].id.clone())),
            None => (0, ItemPosition::First),
        };
        items.insert(index, lost.clone());
        let (page, item) = (page_id.to_string(), lost.clone());
        cmds.push(run(move |api| async move {
            let made = api
                .create_item(
                    &page,
                    position,
                    item.level,
                    &item.body,
                    Some(item.id.clone()),
                )
                .await;
            if let Err(e) = made {
                log_error(format!("Failed to make {} again: {e}", item.id));
            }
        }));
    }
    Command::all(cmds)
}

/// Ends the edits of the items that the page read again does not have any more. When the
/// focused item went, the item before it takes the focus.
fn end_lost_edits(model: &mut Model, ids_before: &[String]) -> Option<Cmd> {
    let lost: Vec<String> = model
        .state
        .editing_item_ids()
        .iter()
        .filter(|id| model.state.get_item_by_id(id).is_none())
        .cloned()
        .collect();
    for id in &lost {
        model.state.remove_editing_item(id);
    }
    let focused = model.state.focused_item_id()?.to_string();
    if !lost.contains(&focused) {
        return None;
    }
    let at = ids_before.iter().position(|id| *id == focused)?;
    let before = ids_before[..at]
        .iter()
        .rev()
        .find_map(|id| model.state.get_item_by_id(id))?;
    Some(open_editor_at(
        model,
        &before.id,
        before.body.chars().count(),
    ))
}

/// Puts the cursor into the editor of an item, `cursor` characters into its body.
fn open_editor_at(model: &mut Model, id: &str, cursor: usize) -> Cmd {
    model.state.add_editing_item(id);
    model.state.set_focused_item(id, Some(cursor));
    model.state.order_focus(Some(id));
    let (id, keys) = (id.to_string(), model.key_relay.hand_over());
    run(move |api| async move { api.focus_editor(Some(id), keys).await })
}

/// Gives the items read again the bodies typed here and not saved yet: the save that follows
/// writes them over the server's version (ADR 069).
fn keep_typed_bodies(model: &Model, items: &mut [Item]) {
    for item in items.iter_mut() {
        if model.edits.dirty_item_ids.contains(&item.id) {
            if let Some(typed) = typed_body(model, &item.id) {
                item.body = typed;
            }
        }
    }
}

/// The focused item that is being edited, and its body.
fn edited_body(model: &Model) -> Option<(String, String)> {
    let id = model.state.focused_item_id()?;
    if !model.state.is_editing(id) {
        return None;
    }
    let item = model.state.get_item_by_id(id)?;
    Some((id.to_string(), item.body.clone()))
}

/// Opens the editor of the edited item again when the page read again changed its body, so
/// the editor shows the new body. The cursor stays inside it.
fn reopen_changed_edit(model: &mut Model, before: Option<(String, String)>) -> Cmd {
    let Some((id, body)) = before else {
        return done();
    };
    let Some(now) = model.state.get_item_by_id(&id).map(|i| i.body.clone()) else {
        return done();
    };
    if now == body {
        return done();
    }
    let length = now.chars().count();
    let cursor = model
        .state
        .focus_request()
        .filter(|r| r.item_id == id)
        .and_then(|r| r.cursor)
        .map_or(length, |c| c.min(length));
    open_editor_at(model, &id, cursor)
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
