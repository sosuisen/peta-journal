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

// 確認に「はい」と答えると、その名前を持つページが上書きされる。
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

// 確認に「いいえ」と答えると、両方のページがそのまま残る。
#[test]
fn a_no_keeps_both_pages() {
    let mut h = showing_a_page();
    h.confirm = false;
    rename_to_theirs(&mut h);

    assert_eq!(overwrites(&h), vec![false]);
    assert!(h.navigations.is_empty());
}

fn save_title(h: &mut Harness, dir_path: &str, file_name: &str) {
    h.send(Event::PageTitleSave {
        dir_path: dir_path.into(),
        file_name: file_name.into(),
    });
}

fn alerts(h: &Harness) -> Vec<String> {
    h.ui.iter()
        .filter_map(|op| match op {
            UiOp::Alert(m) => Some(m.clone()),
            _ => None,
        })
        .collect()
}

// 空の名前と、ファイル名に使えない文字を含む名前は、理由とともに断られる。
#[test]
fn an_empty_or_invalid_name_is_refused_with_the_reason() {
    let mut h = showing_a_page();
    save_title(&mut h, "a/", "  ");
    save_title(&mut h, "a/", "a:b");

    assert_eq!(
        alerts(&h),
        [
            "Page name cannot be empty",
            "Title contains ':' (not allowed in filenames)"
        ]
    );
    assert!(!h.api.called("UpdatePage"));
}

// 名前の中の `/` はフォルダの区切りである。`b/c` と付けると、フォルダ `a/b/` の `c` になる。
#[test]
fn a_slash_in_the_name_moves_the_page_into_a_folder() {
    let mut h = showing_a_page();
    h.api.answer_next(
        "UpdatePage",
        Ok(ApiOutput::PageInfo(Page {
            id: "a/mine".into(),
            dir_path: "a/b/".into(),
            name: "c".into(),
            page_type: "doc".into(),
            ..Page::default()
        })),
    );
    save_title(&mut h, "a", "b/c");

    assert!(h.api.calls.iter().any(|c| matches!(
        c,
        ApiOp::UpdatePage { title, dir_path, .. }
            if title.as_deref() == Some("c") && dir_path.as_deref() == Some("a/b/")
    )));
    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b/c");
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::SetTitle(t) if t == "a/b/c")));
    assert!(h.api.called("GetRecentPages"));
}

// 名前もフォルダも変わらなければ、サーバーには何も送られない。
#[test]
fn an_unchanged_name_is_not_sent() {
    let mut h = showing_a_page();
    save_title(&mut h, "a/", "mine");
    assert!(!h.api.called("UpdatePage"));
}

// サーバーが名前の変更を断ると、その理由が出る。理由がなければ既定の文が出る。
#[test]
fn a_refused_rename_shows_the_reason_or_a_default_message() {
    let mut h = showing_a_page();
    h.api.fail_next("UpdatePage", "disk full");
    save_title(&mut h, "a/", "theirs");
    h.api.fail_next("UpdatePage", "");
    save_title(&mut h, "a/", "theirs");

    assert_eq!(alerts(&h), ["disk full", "Failed to rename page"]);
    assert!(h.navigations.is_empty());
}

// doc ページを開いていなければ、名前の保存は何もしない。
#[test]
fn without_a_doc_page_there_is_nothing_to_rename() {
    let mut h = Harness::new();
    save_title(&mut h, "a/", "theirs");
    assert!(!h.api.called("UpdatePage"));
    assert!(alerts(&h).is_empty());
}
