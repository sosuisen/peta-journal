//! A journal URL with an item as its fragment, as a backlink or a search result opens it.

use crate::app::{Event, Route};
use crate::effects::UiOp;
use crate::harness::Harness;

#[test]
fn a_journal_url_with_an_item_scrolls_to_the_item_and_selects_it() {
    let mut h = Harness::new();
    h.send(Event::Route {
        route: Route::Page {
            name: "2031-01-02".into(),
            fragment: Some("item-2031-01-02".into()),
        },
        today: "2031-01-05".into(),
    });

    assert!(h.ui.iter().any(|op| matches!(
        op,
        UiOp::ScrollToItem { item_id, .. } if item_id == "item-2031-01-02"
    )));
    assert!(h.view().outline.rows.iter().any(|r| r.item.id == "item-2031-01-02" && r.is_selected));
}

#[test]
fn a_journal_url_without_an_item_does_not_scroll() {
    let mut h = Harness::new();
    h.send(Event::Route {
        route: Route::Page {
            name: "2031-01-02".into(),
            fragment: None,
        },
        today: "2031-01-05".into(),
    });

    assert!(!h.ui.iter().any(|op| matches!(op, UiOp::ScrollToItem { .. })));
}
