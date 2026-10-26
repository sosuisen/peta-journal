//! The backlinks at the end of a doc page.

use crate::app::{Event, Route};
use crate::effects::{ApiOutput, SseEvent};
use crate::harness::{item, Harness};
use crate::model::{BacklinkEntry, Page};
use crate::view::{BacklinkGroup, BacklinkItem, ScreenView};

fn page(id: &str, dir_path: &str, name: &str, page_type: &str) -> Page {
    Page {
        id: id.into(),
        dir_path: dir_path.into(),
        name: name.into(),
        page_type: page_type.into(),
        ..Default::default()
    }
}

fn backlinks(h: &Harness) -> Vec<BacklinkGroup> {
    match &h.view().screen {
        ScreenView::Page(Some(p)) => p.backlinks.clone(),
        _ => panic!("no page shown"),
    }
}

fn open(h: &mut Harness, name: &str) {
    h.send(Event::Route {
        route: Route::Page {
            name: name.into(),
            fragment: None,
        },
        today: "2031-01-01".into(),
    });
}

#[test]
fn a_page_lists_its_backlinks_by_source_page_with_the_linking_items_under_it() {
    let mut h = Harness::new();
    let tagging = page("p1", "notes/", "tagged", "doc");
    let journal = page("p2", "", "2031-01-02", "journal");
    h.api.answer_next(
        "GetBacklinks",
        Ok(ApiOutput::Backlinks(vec![
            BacklinkEntry {
                source_page: journal.clone(),
                source_item: Some(item("i1", 1, "see #topic\nmore lines")),
            },
            BacklinkEntry {
                source_page: journal,
                source_item: Some(item("i2", 2, "and [[tags/topic]]")),
            },
            BacklinkEntry {
                source_page: tagging,
                source_item: None,
            },
        ])),
    );
    open(&mut h, "tags/topic");

    assert!(h.api.called("GetBacklinks"));
    assert_eq!(
        backlinks(&h),
        vec![
            BacklinkGroup {
                title: "2031年1月2日 木曜日".into(),
                url: "/pages/2031-01-02".into(),
                items: vec![
                    BacklinkItem {
                        id: "i1".into(),
                        text: "see #topic".into(),
                        url: "/pages/2031-01-02#i1".into(),
                    },
                    BacklinkItem {
                        id: "i2".into(),
                        text: "and [[tags/topic]]".into(),
                        url: "/pages/2031-01-02#i2".into(),
                    },
                ],
            },
            BacklinkGroup {
                title: "notes/tagged".into(),
                url: "/pages/notes/tagged".into(),
                items: Vec::new(),
            },
        ]
    );
}

#[test]
fn a_change_to_any_page_loads_the_backlinks_again() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.api.answer_next(
        "GetBacklinks",
        Ok(ApiOutput::Backlinks(vec![BacklinkEntry {
            source_page: page("p1", "", "other", "doc"),
            source_item: None,
        }])),
    );
    h.send(Event::Sse(SseEvent::Message(
        r#"{"type":"pagesUpdated","pageId":"p1","source":"app"}"#.into(),
    )));

    assert_eq!(backlinks(&h).len(), 1);
}
