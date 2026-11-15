//! What a native shell asks after a character is typed: the body with its closing added.
//! The cursor is in UTF-16 units, as a text field of the shell counts it (ADR 043).

use crate::closed_as_typed;

// 開始記号を打ち終えると、閉じる記号を補った本文が返る。カーソルはUTF-16の単位で数える。
#[test]
fn an_opening_typed_comes_back_closed_with_the_cursor_in_utf16_units() {
    let body = "😀\n>>>";

    assert_eq!(closed_as_typed(body, 6), Some("😀\n>>>\n>>>".to_string()));
}
