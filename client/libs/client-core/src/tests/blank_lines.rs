//! Blank lines in a body. A blank line between two blocks would end the item in its file, so
//! the file has a line of one backslash in its place (ADR 070). The shell sees blank lines.

use crate::app::{Event, Route};
use crate::effects::ApiOutput;
use crate::harness::{item, split_calls, typed, update_calls, Harness};
use crate::model::{Page, PageWithItems};

fn notes() -> Page {
    Page {
        id: "notes".into(),
        name: "notes".into(),
        page_type: "doc".into(),
        ..Default::default()
    }
}

/// Opens the page `notes`, whose file holds an item with this body.
fn open_with_body(h: &mut Harness, body: &str) {
    for name in ["GetOrCreateDoc", "GetPageById"] {
        h.api.answer_next(
            name,
            Ok(ApiOutput::Page(PageWithItems {
                page: notes(),
                items: vec![item("a", 1, body)],
                created: false,
                transcluded_trees: None,
            })),
        );
    }
    h.send(Event::Route {
        route: Route::Page {
            name: "notes".into(),
            fragment: None,
        },
        today: "2026-02-10".into(),
    });
}

// ファイルの本文の`\`だけの行は、画面では空行になる。
#[test]
fn a_backslash_line_of_a_loaded_body_is_shown_as_a_blank_line() {
    let mut h = Harness::new();

    open_with_body(&mut h, "a\n\\\nb");

    assert_eq!(h.bodies(), ["a=a\n\nb"]);
}

// 自動保存は、本文の途中の空行を`\`だけの行として送る。ファイルで項目が分かれないためである。
#[test]
fn the_auto_save_sends_a_blank_line_as_a_backslash_line() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "a\n\nb");

    h.fire_timers();

    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("a\n\\\nb".into()))]
    );
}

// `\`だけの行は、`\`を1つ足して送る。読み戻すときに、空行と区別するためである。
#[test]
fn a_line_of_backslashes_is_sent_with_one_more() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "a\n\\");

    h.fire_timers();

    assert_eq!(update_calls(&h.api), [("a".into(), Some("a\n\\\\".into()))]);
}

// 分割は、ファイルの形の本文と、その本文の中で同じ場所を指すカーソルを送る。空行が`\`だけの行になると、後ろの文字の位置がずれるためである。
#[test]
fn a_split_sends_the_body_of_the_file_and_the_cursor_in_it() {
    let mut h = Harness::with_items(vec![item("a", 1, "a\n\nb\n\\c")]);
    h.model.state.add_editing_item("a");

    let now_ms = h.now_ms;
    h.send(Event::EnterPressed {
        item_id: "a".into(),
        cursor: "a\n\nb\n\\".len(),
        now_ms,
    });

    assert_eq!(
        split_calls(&h.api),
        [("a".to_string(), "a\n\\\nb\n\\c".to_string(), 7)]
    );
}
