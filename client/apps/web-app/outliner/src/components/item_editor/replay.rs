//! Keys pressed before this editor was on screen (held by the core's key relay) are played
//! into the textarea once it is.

use super::text::replace_range;
use crate::core::CoreHandle;
use client_core::model::{DeferredKey, TextKey};
use client_core::util::utf16;
use client_core::Event;
use leptos::prelude::*;

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
    let value = el.value();
    // The textarea counts UTF-16 units; the core counts characters (ADR 043)
    let chars = |at: Result<Option<u32>, _>| {
        utf16::chars_of_utf16(&value, at.ok().flatten().unwrap_or(0) as usize)
    };
    let units = |at: usize| utf16::utf16_of_chars(&value, at) as u32;
    let (start, end) = (chars(el.selection_start()), chars(el.selection_end()));
    match key.text_key(&value, start, end) {
        Some(TextKey::Delete { from, to }) => {
            replace_range(core, item_id, el, "", units(from), units(to));
            true
        }
        Some(TextKey::MoveTo(at)) => {
            let _ = el.set_selection_range(units(at), units(at));
            true
        }
        None => false,
    }
}
