//! Blank lines in a body. A blank line between two blocks would end the item in its file, so
//! the file has a line of one backslash in its place (ADR 070). The shell sees blank lines.

use crate::app::{Event, Route};
use crate::effects::ApiOutput;
use crate::harness::{item, Harness};
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
