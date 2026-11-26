//! Joins two neighbouring items into one: Backspace at the start of an item pulls it into the
//! item above, Delete at the end pulls the item below into it. A transclusion is never merged,
//! and the days of a journal are not merged into each other. An item that other items
//! transclude is merged away only after the user says OK, as a deletion of it would be.
//!
//! The server joins the two in one request, which carries the text that was typed and not
//! saved yet (ADR 011). The screen changes when its answer is there.

use crate::app::Event;
use crate::app::{done, run, Cmd, EditPlace, Model};
use crate::model::Item;
use crate::model::ItemExt;
use crate::ops::action::confirm_deleting_transcluded;
use crate::ops::edit::{record_typing, show_parsed, take_unsaved, typed_body};
use crate::ops::outline_events::edit_requested;
use crate::outline::tree::index_of;
use crate::outline::CursorState;
use crate::util::char_len;
use crux_core::Command;

/// The key that joins two items. It says which of the two has the editor: Backspace is
/// pressed in the item that goes away, Delete in the item that takes the text.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum JoinKey {
    Backspace,
    Delete,
}

impl JoinKey {
    /// The item that goes away and the item that takes its text, for the key pressed in the
    /// editor of `item_id`: None when the two are not joined.
    fn join(self, model: &Model, item_id: &str) -> Option<(Item, Item)> {
        let (goes, into) = match self {
            JoinKey::Backspace => previous_join(model, item_id),
            JoinKey::Delete => next_join(model, item_id).map(|(item, next)| (next, item)),
        }?;
        // The text a transclusion shows is the text of another item: it is not joined, and
        // nothing is joined into it. The server refuses both
        let transclusion = goes.transclude_item_id.is_some() || into.transclude_item_id.is_some();
        (!transclusion).then_some((goes, into))
    }
}

/// The item Backspace at its start joins into the one above, and that item: None when the
/// two are not joined.
fn previous_join(model: &Model, item_id: &str) -> Option<(Item, Item)> {
    let item = model.cloned_item(item_id)?;
    let items = model.state.visible_items();
    let idx = index_of(items, &item.id)?;
    if idx == 0 {
        return None;
    }
    let prev: Item = (*items[idx - 1]).clone();
    // In journal daily pages, prevent merging across page boundaries
    if item.is_daily_journal_item() && prev.page_id != item.page_id {
        return None;
    }
    Some((item, prev))
}

/// The item Delete at its end takes in, and the item below it: None when the two are not
/// joined. Same limit as Backspace: journal days are not merged.
fn next_join(model: &Model, item_id: &str) -> Option<(Item, Item)> {
    let item = model.cloned_item(item_id)?;
    let items = model.state.visible_items();
    let idx = index_of(items, &item.id)?;
    let next: Item = (**items.get(idx + 1)?).clone();
    if item.is_daily_journal_item() && next.page_id != item.page_id {
        return None;
    }
    Some((item, next))
}

/// Backspace at the start: the item's text goes to the end of the item above, which gets
/// the editor, and the item is deleted.
pub fn merge_into_previous(model: &mut Model, item_id: &str) -> Cmd {
    ask(model, JoinKey::Backspace, item_id)
}

/// Delete at the end: the text of the item below is appended to this item, and the item
/// below is deleted.
pub fn merge_next_into(model: &mut Model, item_id: &str) -> Cmd {
    ask(model, JoinKey::Delete, item_id)
}

/// Asks whether the item that goes away may go, which takes a dialog when other items
/// transclude it.
fn ask(model: &mut Model, key: JoinKey, item_id: &str) -> Cmd {
    let Some((goes, _)) = key.join(model, item_id) else {
        return done();
    };
    // The editor of the joined item is focused when the server has answered; keys pressed
    // before that are kept for it
    model.key_relay.hold_until_focus();
    let item_id = item_id.to_string();
    run(move |api| async move {
        let agreed = confirm_deleting_transcluded(&api, vec![goes.id]).await;
        api.send_event(Event::MergeAnswered {
            key,
            item_id,
            agreed,
        });
    })
}

/// Sends the join once the user agrees. The join is looked at again: the page may have
/// changed while the user was asked. A No puts the cursor back where the key was pressed,
/// as the dialog took the focus.
pub fn merge_answered(model: &mut Model, key: JoinKey, item_id: &str, agreed: bool) -> Cmd {
    let Some((goes, into)) = key.join(model, item_id) else {
        model.key_relay.release();
        return done();
    };
    if !agreed {
        model.key_relay.release();
        let cursor = match key {
            JoinKey::Backspace => 0,
            JoinKey::Delete => char_len(&into.body),
        };
        return edit_requested(model, item_id, EditPlace::Markdown(cursor));
    }
    let (body, discard_goes) = take_unsaved(model, &goes);
    let (into_body, discard_into) = take_unsaved(model, &into);
    let request = run(move |api| async move {
        match api
            .merge_items(&goes.id, &into.id, body.as_deref(), into_body.as_deref())
            .await
        {
            Ok(merged) => api.send_event(Event::Merged {
                key,
                item_id: goes.id,
                merged: Box::new(merged),
            }),
            Err(e) => {
                api.alert(e.message_or("Failed to merge items"));
                let carried = [(goes.id, body), (into.id, into_body)];
                api.send_event(Event::MergeFailed {
                    unsaved: carried
                        .into_iter()
                        .filter_map(|(id, body)| body.map(|_| id))
                        .collect(),
                });
            }
        }
    });
    Command::all([discard_goes, discard_into, request])
}

/// Shows what the server wrote for a join, records it as one undo entry, and edits the
/// joined item with the cursor where the two texts meet.
pub fn show_merged(model: &mut Model, key: JoinKey, item_id: &str, merged: Item) -> Cmd {
    let keys = model.key_relay.hand_over();
    let (Some(gone), Some(before)) = (
        model.state.get_item_by_id(item_id),
        model.state.get_item_by_id(&merged.id),
    ) else {
        // No editor is focused: no answer would end the hold
        model.key_relay.release();
        return done();
    };
    let meet = char_len(&before.body);
    model
        .state
        .update_item(&merged.id, |i| i.body = merged.body.clone());
    show_parsed(model, &merged);
    model.state.delete_item(item_id);

    let (label, cursor_before) = match key {
        JoinKey::Backspace => ("backspace", CursorState::new(item_id, 0)),
        JoinKey::Delete => ("delete", CursorState::new(&merged.id, meet)),
    };
    model.undo.begin(label, Some(cursor_before));
    if let Some(after) = model.state.get_item_by_id(&merged.id) {
        model.undo.push_update(&before, &after);
    }
    model.undo.push_delete(&gone);
    model
        .undo
        .commit(Some(CursorState::new(&merged.id, meet)), model.clock_ms);

    model.state.add_editing_item(&merged.id);
    model.state.remove_editing_item(item_id);
    model.state.set_focused_item(&merged.id, Some(meet));
    model.state.order_focus(Some(&merged.id));
    run(move |api| async move { api.focus_editor(Some(merged.id), keys).await })
}

/// Ends the wait for the editor after a join that did not reach the server, and schedules
/// the auto-save of the typed text the join was to carry.
pub fn merge_failed(model: &mut Model, unsaved: &[String]) -> Cmd {
    model.key_relay.release();
    let saves: Vec<Cmd> = unsaved
        .iter()
        .filter_map(|id| Some((model.cloned_item(id)?, typed_body(model, id)?)))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(item, typed)| record_typing(model, &item, &typed))
        .collect();
    Command::all(saves)
}
