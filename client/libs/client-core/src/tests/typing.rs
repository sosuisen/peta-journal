use crate::typing::{event_of, Typing};

const NOW: u64 = 1_000;

// 打った文字は、その時刻とともにコアへ届く。カーソルは、UTF-16の位置から、コアの数える文字の位置になる。絵文字は、UTF-16では2つ、コアでは1文字である。
#[test]
fn typed_text_reaches_the_core_with_its_time_and_the_cursor_in_characters() {
    let typing = Typing::Typed {
        item_id: "a".into(),
        body: "😀ab".into(),
        cursor: 3,
    };

    let event = event_of(typing, NOW);

    assert_eq!(
        format!("{event:?}"),
        r#"Typed { item_id: "a", body: "😀ab", cursor: 2, now_ms: 1000 }"#
    );
}

// カーソルのない本文の変更は、その時刻とともにコアへ届く。
#[test]
fn a_change_without_a_cursor_reaches_the_core_with_its_time() {
    let typing = Typing::Changed {
        item_id: "a".into(),
        body: "cut".into(),
    };

    let event = event_of(typing, NOW);

    assert_eq!(
        format!("{event:?}"),
        r#"BodyChanged { item_id: "a", body: "cut", now_ms: 1000 }"#
    );
}

// 本文の外のカーソルは、本文の末尾とみなす。
#[test]
fn a_cursor_beyond_the_body_is_at_its_end() {
    let typing = Typing::Typed {
        item_id: "a".into(),
        body: "ab".into(),
        cursor: u64::MAX,
    };

    let event = event_of(typing, NOW);

    assert!(format!("{event:?}").contains("cursor: 2,"));
}
