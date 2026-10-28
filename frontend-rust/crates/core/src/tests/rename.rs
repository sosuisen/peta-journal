//! Renaming a page to the name another page has.

use crate::app::{Event, PageState, Screen};
use crate::effects::{ApiOp, ApiOutput, UiOp};
use crate::harness::Harness;
use crate::model::Page;

fn showing_a_page() -> Harness {
    let page = Page {
        id: "a/mine".into(),
        dir_path: "a/".into(),
        name: "mine".into(),
        page_type: "doc".into(),
        ..Page::default()
    };
    let mut h = Harness::new();
    h.model.screen = Screen::Page(PageState {
        page_name: page.canonical_path(),
        page: Some(page),
        ..PageState::default()
    });
    h.api
        .answer_next("UpdatePage", Ok(ApiOutput::PageNameTaken("taken".into())));
    h
}

fn rename_to_theirs(h: &mut Harness) {
    h.send(Event::PageTitleSave {
        dir_path: "a/".into(),
        file_name: "theirs".into(),
    });
}

fn overwrites(h: &Harness) -> Vec<bool> {
    h.api
        .calls_named("UpdatePage")
        .into_iter()
        .map(|op| match op {
            ApiOp::UpdatePage { overwrite, .. } => *overwrite,
            _ => unreachable!(),
        })
        .collect()
}

#[test]
fn a_yes_overwrites_the_page_that_has_the_name() {
    let mut h = showing_a_page();
    rename_to_theirs(&mut h);

    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::Confirm(q) if q.contains("a/theirs"))));
    assert_eq!(overwrites(&h), vec![false, true]);
    // The fake server echoes only the fields it got, so the folder is not in the answer
    assert!(h.navigations.last().unwrap().url.ends_with("/theirs"));
}

#[test]
fn a_no_keeps_both_pages() {
    let mut h = showing_a_page();
    h.confirm = false;
    rename_to_theirs(&mut h);

    assert_eq!(overwrites(&h), vec![false]);
    assert!(h.navigations.is_empty());
}
