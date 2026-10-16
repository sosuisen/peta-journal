//! Keys pressed before this editor was on screen (held by the core's key relay) are played
//! into the textarea once it is.

use super::text::replace_range;
use crate::core::CoreHandle;
use leptos::prelude::*;
use peta_core::model::DeferredKey;
use peta_core::Event;

/// Plays keys that were pressed before this editor was on screen. Typed characters go into
/// the text; other keys go through the keydown handler.
pub(super) fn replay_keys(
    core: &CoreHandle,
    el: &web_sys::HtmlTextAreaElement,
    keys: Vec<DeferredKey>,
) {
    let item_id = el.id().trim_start_matches("item-editor-").to_string();
    for key in keys {
        // A replayed Enter starts a new hold; the keys after it belong to the item it creates
        if core.view().with_untracked(|v| v.outline.key_relay_holding) {
            core.send(Event::KeyHeld(key));
            continue;
        }
        if key.is_typed_character() {
            let start = el.selection_start().ok().flatten().unwrap_or(0);
            let end = el.selection_end().ok().flatten().unwrap_or(0);
            replace_range(core, &item_id, el, &key.key, start, end);
        } else if !replay_editing_key(core, &item_id, el, &key) {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key(&key.key);
            init.set_code(&key.code);
            init.set_shift_key(key.shift_key);
            init.set_ctrl_key(key.ctrl_key);
            init.set_alt_key(key.alt_key);
            init.set_meta_key(key.meta_key);
            init.set_bubbles(true);
            init.set_cancelable(true);
            if let Ok(ev) =
                web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            {
                let _ = el.dispatch_event(&ev);
            }
        }
    }
}

/// Does what the browser would have done for a key that edits or moves inside the text.
/// A synthetic keydown has no default action, so these are done by hand.
fn replay_editing_key(
    core: &CoreHandle,
    item_id: &str,
    el: &web_sys::HtmlTextAreaElement,
    key: &DeferredKey,
) -> bool {
    if key.shift_key || key.alt_key {
        return false;
    }
    let start = el.selection_start().ok().flatten().unwrap_or(0);
    let end = el.selection_end().ok().flatten().unwrap_or(0);
    let len = el.value().encode_utf16().count() as u32;
    let has_selection = start != end;
    match key.key.as_str() {
        "Backspace" => {
            if !has_selection && start == 0 {
                return false; // the handler merges with the item above
            }
            replace_range(
                core,
                item_id,
                el,
                "",
                if has_selection { start } else { start - 1 },
                end,
            );
            true
        }
        "Delete" => {
            if !has_selection && end == len {
                return false; // the handler merges the item below
            }
            replace_range(
                core,
                item_id,
                el,
                "",
                start,
                if has_selection { end } else { end + 1 },
            );
            true
        }
        "ArrowLeft" => {
            let p = start.saturating_sub(1);
            let _ = el.set_selection_range(p, p);
            true
        }
        "ArrowRight" => {
            let p = (end + 1).min(len);
            let _ = el.set_selection_range(p, p);
            true
        }
        "Home" => {
            let _ = el.set_selection_range(0, 0);
            true
        }
        "End" => {
            let _ = el.set_selection_range(len, len);
            true
        }
        _ => false,
    }
}
