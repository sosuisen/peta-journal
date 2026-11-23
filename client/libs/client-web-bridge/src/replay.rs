//! The play of a held key into the editor that was not on screen when it was pressed. The core
//! decides what the key does (`DeferredKey`); the shell does it to its textarea.

use client_core::model::{DeferredKey, TextKey};
use client_core::util::utf16::{chars_of_utf16, utf16_of_chars};

/// What the shell does with a held key, in the textarea that has `text`. Places count UTF-16
/// units, as the textarea does.
#[derive(Debug, Clone, PartialEq, Eq, facet::Facet)]
#[repr(C)]
pub enum Replay {
    /// The key types this text over the selection.
    Type(String),
    /// The text from `from` to `to` goes.
    Delete { from: u64, to: u64 },
    /// The cursor goes to the place.
    MoveTo(u64),
    /// The key goes to the editor as a key press, as Enter or Tab do.
    Press,
}

/// What `key` does in `text` with the selection from `start` to `end`.
pub fn replay_of(key: &DeferredKey, text: &str, start: u64, end: u64) -> Replay {
    if key.is_typed_character() {
        return Replay::Type(key.key.clone());
    }
    let chars = |units: u64| chars_of_utf16(text, usize::try_from(units).unwrap_or(usize::MAX));
    let units = |chars: usize| utf16_of_chars(text, chars) as u64;
    match key.text_key(text, chars(start), chars(end)) {
        Some(TextKey::Delete { from, to }) => Replay::Delete {
            from: units(from),
            to: units(to),
        },
        Some(TextKey::MoveTo(at)) => Replay::MoveTo(units(at)),
        None => Replay::Press,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> DeferredKey {
        DeferredKey {
            key: name.into(),
            ..DeferredKey::default()
        }
    }

    // 文字のキーは、選んだ範囲にその文字を打つ。
    #[test]
    fn a_key_of_a_letter_types_it() {
        assert_eq!(replay_of(&key("a"), "xy", 1, 1), Replay::Type("a".into()));
    }

    // 文字を消すキーと動かすキーは、UTF-16の位置で答える。前の文字が絵文字なら、2単位を消す。
    #[test]
    fn keys_that_delete_or_move_answer_in_utf16_units() {
        let text = "a👍b";

        assert_eq!(
            replay_of(&key("Backspace"), text, 3, 3),
            Replay::Delete { from: 1, to: 3 }
        );
        assert_eq!(replay_of(&key("Home"), text, 3, 3), Replay::MoveTo(0));
        assert_eq!(replay_of(&key("ArrowRight"), text, 1, 1), Replay::MoveTo(3));
    }

    // 名前のあるキーと、Ctrlとともに押したキーは、キーとして編集欄に渡す。
    #[test]
    fn a_named_key_and_a_key_with_ctrl_go_to_the_editor_as_keys() {
        let ctrl_z = DeferredKey {
            ctrl_key: true,
            ..key("z")
        };

        assert_eq!(replay_of(&key("Enter"), "x", 1, 1), Replay::Press);
        assert_eq!(replay_of(&ctrl_z, "x", 1, 1), Replay::Press);
        assert_eq!(replay_of(&key("Backspace"), "x", 0, 0), Replay::Press);
    }
}
