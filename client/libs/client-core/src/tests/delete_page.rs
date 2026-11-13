//! The header's "Delete page": the shown page goes after a confirmation.

use crate::app::{Event, JournalState, PageState, Screen};
use crate::effects::{ApiOp, UiOp};
use crate::harness::Harness;
use crate::model::Page;

fn showing(screen: Screen) -> Harness {
    let mut h = Harness::new();
    h.model.screen = screen;
    h
}

/// A page the screen has loaded, with the id the server gave it
fn loaded(id: &str) -> Option<Page> {
    Some(Page {
        id: id.into(),
        ..Page::default()
    })
}

fn doc(page_name: &str) -> Screen {
    Screen::Page(PageState {
        page_name: page_name.into(),
        page: loaded("page-Idea"),
        ..Default::default()
    })
}

fn journal(page_name: &str) -> Screen {
    Screen::Journal(JournalState {
        page_name: page_name.into(),
        page: loaded("page-journal"),
        ..Default::default()
    })
}

fn deleted_pages(h: &Harness) -> Vec<String> {
    h.api
        .calls
        .iter()
        .filter_map(|c| match c {
            ApiOp::DeletePage { page_id } => Some(page_id.clone()),
            _ => None,
        })
        .collect()
}

fn delete(h: &mut Harness) {
    h.send(Event::DeleteCurrentPage);
}

fn urls(h: &Harness) -> Vec<String> {
    h.navigations.iter().map(|n| n.url.clone()).collect()
}

// 開いている doc ページは、確認の後に、画面が持つページのIDで削除され、今日のジャーナルが開く。最近のページも読み直される。
// サーバーへの要求は、削除の1つである。ページを引く要求は送らない。
#[test]
fn a_doc_page_is_deleted_after_a_confirmation_and_today_opens() {
    let mut h = showing(doc("notes/Idea"));
    delete(&mut h);

    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::Confirm(m) if m == "Delete \"notes/Idea\"?")));
    assert_eq!(deleted_pages(&h), ["page-Idea"]);
    assert!(!h.api.called("GetOrCreateDoc"));
    assert!(h.api.called("GetRecentPages"));
    assert_eq!(urls(&h), ["/today"]);
}

// 確認に「いいえ」と答えると、何も削除されない。
#[test]
fn a_no_to_the_confirmation_deletes_nothing() {
    let mut h = showing(doc("notes/Idea"));
    h.confirm = false;
    delete(&mut h);

    assert!(!h.api.called("DeletePage"));
    assert!(urls(&h).is_empty());
}

// 日付のジャーナルも、画面が持つページのIDで削除される。ジャーナルを引く要求は送らない。
#[test]
fn a_journal_page_is_deleted_by_the_id_the_screen_has() {
    let mut h = showing(journal("2026-02-10"));
    delete(&mut h);

    assert_eq!(deleted_pages(&h), ["page-journal"]);
    assert!(!h.api.called("GetOrCreateJournal"));
}

// ページがまだ読み込まれていなければ、確認も削除もしない。
#[test]
fn a_page_that_is_not_loaded_yet_is_not_deleted() {
    let mut h = showing(Screen::Page(PageState {
        page_name: "notes/Idea".into(),
        ..Default::default()
    }));
    delete(&mut h);

    assert!(!h.ui.iter().any(|op| matches!(op, UiOp::Confirm(_))));
    assert!(deleted_pages(&h).is_empty());
}

// `today` のジャーナルと設定画面では、削除するページがないので何もしない。
#[test]
fn today_and_the_settings_have_no_page_to_delete() {
    for screen in [
        journal("today"),
        journal(""),
        Screen::Settings,
        Screen::None,
    ] {
        let mut h = showing(screen);
        delete(&mut h);
        assert!(!h.ui.iter().any(|op| matches!(op, UiOp::Confirm(_))));
        assert!(!h.api.called("DeletePage"));
    }
}

// 削除をサーバーが断ると、理由か既定の文が出て、今日へは移らない。
#[test]
fn a_failure_is_told_and_today_does_not_open() {
    let mut h = showing(doc("Idea"));
    h.api.fail_next("DeletePage", "has child pages");
    delete(&mut h);
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::Alert(m) if m == "has child pages")));
    assert!(urls(&h).is_empty());

    let mut h = showing(doc("Idea"));
    h.api.fail_next("DeletePage", "");
    delete(&mut h);
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::Alert(m) if m == "Failed to delete page")));
    assert!(urls(&h).is_empty());
}
