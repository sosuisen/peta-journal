//! The keys of the browser as the core knows them (ADR 052). What a key means is in the core's
//! [`KeyMap`]; the browser only names its keys.

use client_core::app::SelectionKey;
use client_core::keys::{Chord, EditorKey, Key, KeyMap, KeyPlatform};

thread_local! {
    static KEYS: KeyMap = KeyMap::of(KeyPlatform::Web);
}

/// The chord of a key event. None for a key that means nothing to the outline in any map.
pub fn chord_of(ev: &web_sys::KeyboardEvent) -> Option<Chord> {
    let name = ev.key();
    let key = match name.as_str() {
        "Enter" => Key::Enter,
        "Tab" => Key::Tab,
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "ArrowLeft" => Key::Left,
        "ArrowRight" => Key::Right,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "Escape" => Key::Escape,
        // Shift makes a letter upper case
        letter if letter.chars().count() == 1 => Key::Letter(letter.to_lowercase()),
        _ => return None,
    };
    Some(Chord {
        key,
        shift: ev.shift_key(),
        ctrl: ev.ctrl_key(),
        alt: ev.alt_key(),
        cmd: ev.meta_key(),
    })
}

/// What a key event means in the editor of an item.
pub fn in_editor(ev: &web_sys::KeyboardEvent) -> Option<EditorKey> {
    let chord = chord_of(ev)?;
    KEYS.with(|keys| keys.in_editor(&chord))
}

/// What a key event means while items are selected.
pub fn in_selection(ev: &web_sys::KeyboardEvent) -> Option<SelectionKey> {
    let chord = chord_of(ev)?;
    KEYS.with(|keys| keys.in_selection(&chord))
}
