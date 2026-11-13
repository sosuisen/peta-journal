//! The "+" button of the header: a new doc page.

use crate::app::Event;
use crate::effects::UiOp;
use crate::harness::Harness;

fn new_page(h: &mut Harness, search_query: &str) {
    h.send(Event::CreateNewPage {
        search_query: search_query.into(),
    });
}

fn urls(h: &Harness) -> Vec<String> {
    h.navigations.iter().map(|n| n.url.clone()).collect()
}

// 検索バーに文があれば、それが新しいページの名前になる。名前は尋ねられず、検索は空になる。
#[test]
fn the_search_text_names_the_new_page_without_asking() {
    let mut h = Harness::new();
    h.send(Event::SearchQueryChanged(" Plan ".into()));
    new_page(&mut h, " Plan ");

    assert_eq!(urls(&h), ["/pages/Plan"]);
    assert!(!h.ui.iter().any(|op| matches!(op, UiOp::Prompt(_))));
    assert_eq!(h.view().search.query, "");
}

// 検索バーが空なら名前を尋ねる。答えがその名前になり、取り消すと何も作られない。
#[test]
fn without_search_text_the_title_is_asked_for() {
    let mut h = Harness::new();
    h.prompt = Some("Idea".into());
    new_page(&mut h, "");
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Prompt(_))));
    assert_eq!(urls(&h), ["/pages/Idea"]);

    h.prompt = None;
    new_page(&mut h, "");
    assert_eq!(urls(&h).len(), 1);
}

// 空の名前を答えると、今の日時からの Untitled の名前になる。
#[test]
fn an_empty_answer_makes_an_untitled_page_named_by_the_time() {
    let mut h = Harness::new();
    h.prompt = Some("  ".into());
    new_page(&mut h, "");
    assert_eq!(urls(&h), ["/pages/Untitled-202602100905"]);
}

// ファイル名に使えない名前は断られ、理由が出る。ジャーナルの日付の名前は検査されずに開く。
#[test]
fn a_bad_file_name_is_refused_and_a_journal_name_is_not_checked() {
    let mut h = Harness::new();
    new_page(&mut h, "a:b");
    assert!(urls(&h).is_empty());
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));

    new_page(&mut h, "2026-W07");
    assert_eq!(urls(&h), ["/pages/2026-W07"]);
}
