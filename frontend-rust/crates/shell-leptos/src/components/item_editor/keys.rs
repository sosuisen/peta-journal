//! The keyboard shortcuts of the editor.

use super::text::insert_at_cursor;
use crate::core::CoreHandle;
use crate::dom;
use leptos::prelude::*;
use peta_core::view::Row;
use peta_core::Event;

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
    let value = el.value();
    let start = el.selection_start().ok().flatten().unwrap_or(0);
    let end = el.selection_end().ok().flatten().unwrap_or(0);
    let len = value.encode_utf16().count() as u32;
    let id = item_id.to_string();
    let send = |event: Event| core.send(event);

    if row.with_untracked(|r| r.suggestions_visible)
        && matches!(key.as_str(), "ArrowUp" | "ArrowDown" | "Enter" | "Escape")
    {
        ev.prevent_default();
        if key == "Escape" {
            send(Event::WikiQuery {
                item_id: id,
                context: None,
            });
        } else {
            send(Event::WikiKey { key });
        }
        return;
    }

    if (key == "z" || key == "Z") && ev.ctrl_key() && !ev.alt_key() {
        ev.prevent_default();
        send(if ev.shift_key() {
            Event::Redo
        } else {
            Event::Undo
        });
        return;
    }

    if key == "o" && ev.ctrl_key() {
        ev.prevent_default();
        send(Event::OpenLinkAt {
            item_id: id,
            cursor: dom::utf16_to_chars(&value, start),
        });
        return;
    }

    if key == "Enter" && ev.ctrl_key() {
        ev.prevent_default();
        send(Event::TaskCycle { item_id: id });
        return;
    }

    if key == "Enter" && ev.shift_key() && !ev.ctrl_key() {
        // Shift+Enter: a hard line break, which Flat Markdown keeps as \n
        ev.prevent_default();
        insert_at_cursor(core, item_id, el, "\n", 1);
        return;
    }

    if key == "Enter" && !ev.shift_key() && !ev.ctrl_key() {
        ev.prevent_default();
        send(Event::EnterPressed {
            item_id: id,
            cursor: dom::utf16_to_chars(&value, start),
            now_ms: core.now_ms(),
        });
        return;
    }

    if key == "Backspace" && start == 0 && end == 0 {
        ev.prevent_default();
        send(Event::BackspaceAtStart { item_id: id });
        return;
    }

    if key == "Delete" && start == len && end == len {
        ev.prevent_default();
        send(Event::DeleteAtEnd { item_id: id });
        return;
    }

    if ev.ctrl_key() && (key == "ArrowUp" || key == "ArrowDown") {
        ev.prevent_default();
        send(Event::CollapseToggle { item_id: id });
        return;
    }

    if ev.shift_key() && ev.alt_key() {
        let event = match key.as_str() {
            "ArrowUp" => Some(Event::MoveUp {
                item_id: id.clone(),
            }),
            "ArrowDown" => Some(Event::MoveDown {
                item_id: id.clone(),
            }),
            "ArrowLeft" => Some(Event::Outdent {
                item_id: id.clone(),
            }),
            "ArrowRight" => Some(Event::Indent {
                item_id: id.clone(),
            }),
            _ => None,
        };
        if let Some(event) = event {
            ev.prevent_default();
            send(event);
            return;
        }
    }

    if ev.shift_key()
        && !ev.alt_key()
        && !ev.ctrl_key()
        && ((key == "ArrowUp" && dom::is_on_first_line(el))
            || (key == "ArrowDown" && dom::is_on_last_line(el)))
    {
        ev.prevent_default();
        ev.stop_propagation();
        send(Event::ShiftSelect { item_id: id });
        return;
    }

    let cursor_chars = dom::utf16_to_chars(&value, start);
    if key == "ArrowUp" && dom::is_on_first_line(el) {
        ev.prevent_default();
        send(Event::NavigateUp {
            item_id: id,
            cursor: cursor_chars,
        });
        return;
    }
    if key == "ArrowDown" && dom::is_on_last_line(el) {
        ev.prevent_default();
        send(Event::NavigateDown {
            item_id: id,
            cursor: cursor_chars,
        });
        return;
    }
    if key == "ArrowLeft" && start == 0 {
        ev.prevent_default();
        // A large number puts the cursor at the end of the previous item
        send(Event::NavigateUp {
            item_id: id,
            cursor: usize::MAX / 2,
        });
        return;
    }
    if key == "ArrowRight" && start == len {
        ev.prevent_default();
        send(Event::NavigateDown {
            item_id: id,
            cursor: 0,
        });
        return;
    }

    if key == "Tab" {
        ev.prevent_default();
        send(if ev.shift_key() {
            Event::Outdent { item_id: id }
        } else {
            Event::Indent { item_id: id }
        });
    }
}
