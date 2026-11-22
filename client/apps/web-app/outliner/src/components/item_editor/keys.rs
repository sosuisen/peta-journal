//! The keyboard shortcuts of the editor.

use super::text::insert_at_cursor;
use crate::body::Row;
use crate::core::CoreHandle;
use crate::dom;
use crate::keyboard;
use client_core::keys::{event_of, EditorKey, EditorState};
use client_core::ops::wiki;
use client_core::Event;
use leptos::prelude::*;

/// The keyboard shortcuts of the editor, in the order `onKeydown` checked them.
pub(super) fn handle_keydown(
    core: &CoreHandle,
    item_id: &str,
    el: &web_sys::HtmlTextAreaElement,
    ev: &web_sys::KeyboardEvent,
    row: RwSignal<Row>,
) {
    // Let the IME handle its own keys (e.g. the Enter that confirms a conversion candidate)
    if dom::is_ime_composing(ev) {
        return;
    }
    let vm = core.view();
    // Keys pressed between Enter and the focus of the next editor belong to that editor.
    // Shortcuts with Ctrl or Meta cannot be replayed, so they are left to the browser here.
    if vm.with_untracked(|v| v.outline.key_relay_holding) && !ev.ctrl_key() && !ev.meta_key() {
        ev.prevent_default();
        core.send(Event::KeyHeld(dom::deferred_key(ev)));
        return;
    }
    // The held keys are played into the next editor after the next render. A key pressed
    // before that waits behind them
    if !ev.ctrl_key() && !ev.meta_key() && core.keep_for_waiting_replay(|| dom::deferred_key(ev)) {
        ev.prevent_default();
        return;
    }
    let key = ev.key();
    if row.with_untracked(|r| r.suggestions_visible) && wiki::takes_key(&key) {
        ev.prevent_default();
        core.send(Event::WikiKey { key });
        return;
    }
    if key == "Enter" && ev.shift_key() && !ev.ctrl_key() {
        // Shift+Enter: a hard line break, which Flat Markdown keeps as \n
        ev.prevent_default();
        insert_at_cursor(core, item_id, el, "\n", 1);
        return;
    }
    let Some(key) = keyboard::in_editor(ev) else {
        return;
    };
    let Some(event) = event_of(key, item_id, &editor_state(el, key), core.now_ms()) else {
        return;
    };
    ev.prevent_default();
    if matches!(event, Event::ShiftSelect { .. }) {
        // The outline would take Shift with an arrow as a key of the selection that starts
        ev.stop_propagation();
    }
    core.send(event);
}

/// The editor as the core reads a key. The lines are only measured for a key that asks for
/// them: the measure lays out a copy of the text.
fn editor_state(el: &web_sys::HtmlTextAreaElement, key: EditorKey) -> EditorState {
    let up = matches!(key, EditorKey::Up | EditorKey::SelectUp);
    let down = matches!(key, EditorKey::Down | EditorKey::SelectDown);
    let place = |at: Result<Option<u32>, _>| u64::from(at.ok().flatten().unwrap_or(0));
    EditorState {
        text: el.value(),
        selection_start: place(el.selection_start()),
        selection_end: place(el.selection_end()),
        on_first_line: up && dom::is_on_first_line(el),
        on_last_line: down && dom::is_on_last_line(el),
    }
}
