//! The header's "Delete page": the shown page goes after a confirmation.

use crate::app::{Event, JournalState, PageState, Screen};
use crate::effects::{ApiOp, UiOp};
use crate::harness::Harness;

fn showing(screen: Screen) -> Harness {
    let mut h = Harness::new();
    h.model.screen = screen;
    h
}

fn doc(page_name: &str) -> Screen {
    Screen::Page(PageState {
        page_name: page_name.into(),
        ..Default::default()
    })
}

fn journal(page_name: &str) -> Screen {
    Screen::Journal(JournalState {
        page_name: page_name.into(),
        ..Default::default()
    })
}

fn delete(h: &mut Harness) {
    h.send(Event::DeleteCurrentPage);
}

fn urls(h: &Harness) -> Vec<String> {
    h.navigations.iter().map(|n| n.url.clone()).collect()
}

// 開いている doc ページは、確認の後にフォルダと名前で引かれて削除され、今日のジャーナルが開く。最近のページも読み直される。
#[test]
fn a_doc_page_is_deleted_after_a_confirmation_and_today_opens() {
    let mut h = showing(doc("notes/Idea"));
    delete(&mut h);

    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::Confirm(m) if m == "Delete \"notes/Idea\"?")));
    assert!(h.api.calls.iter().any(|c| matches!(
        c,
        ApiOp::GetOrCreateDoc { title, dir_path } if title == "Idea" && dir_path == "notes/"
    )));
    assert!(h
        .api
        .calls
        .iter()
        .any(|c| matches!(c, ApiOp::DeletePage { page_id } if page_id == "page-Idea")));
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

// 日付のジャーナルはジャーナルとして引かれて削除される。
#[test]
fn a_journal_page_is_looked_up_as_a_journal() {
    let mut h = showing(journal("2026-02-10"));
    delete(&mut h);

    assert!(h.api.calls.iter().any(
        |c| matches!(c, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-10")
    ));
    assert!(h.api.called("DeletePage"));
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

// ページを引けなければその理由が出る。削除をサーバーが断ると、理由か既定の文が出て、今日へは移らない。
#[test]
fn a_failure_is_told_and_today_does_not_open() {
    let mut h = showing(doc("Idea"));
    h.api.fail_next("GetOrCreateDoc", "not found");
    delete(&mut h);
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::Alert(m) if m == "not found")));

    let mut h = showing(doc("Idea"));
    h.api.fail_next("DeletePage", "");
    delete(&mut h);
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::Alert(m) if m == "Failed to delete page")));
    assert!(urls(&h).is_empty());
}
