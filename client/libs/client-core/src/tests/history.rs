//! The history of the pages that were opened (ADR 035).

use crate::app::{Event, Screen};
use crate::effects::{ApiOp, ApiOutput};
use crate::harness::Harness;

const TODAY: &str = "2026-02-10";

fn open(h: &mut Harness, url: &str) {
    h.send(Event::Open {
        url: url.into(),
        today: TODAY.into(),
    });
}

fn back(h: &mut Harness) {
    h.send(Event::Back {
        today: TODAY.into(),
    });
}

/// The name of the page on screen, or "settings".
fn shown(h: &Harness) -> String {
    match &h.model.screen {
        Screen::Page(page) => page.page_name.clone(),
        Screen::Journal(journal) => journal.page_name.clone(),
        Screen::Settings => "settings".into(),
        Screen::None => "nothing".into(),
    }
}

// 1つ目のページを開いただけでは、戻る先も進む先もない。
#[test]
fn the_first_page_has_nothing_to_go_back_or_forward_to() {
    let mut h = Harness::new();

    open(&mut h, "/pages/a");

    assert_eq!(shown(&h), "a");
    assert!(!h.view().history.can_go_back);
    assert!(!h.view().history.can_go_forward);
}

// 2つ目のページを開くと、戻れるようになる。戻ると、前のページが開き、進めるようになる。
#[test]
fn going_back_opens_the_page_before_and_lets_the_user_go_forward() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");
    open(&mut h, "/pages/b");
    assert!(h.view().history.can_go_back);

    h.send(Event::Back {
        today: TODAY.into(),
    });

    assert_eq!(shown(&h), "a");
    assert!(!h.view().history.can_go_back);
    assert!(h.view().history.can_go_forward);
}

// 戻ったあとに進むと、戻る前のページが開く。
#[test]
fn going_forward_after_going_back_opens_the_page_left() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");
    open(&mut h, "/pages/b");
    back(&mut h);

    h.send(Event::Forward {
        today: TODAY.into(),
    });

    assert_eq!(shown(&h), "b");
    assert!(h.view().history.can_go_back);
    assert!(!h.view().history.can_go_forward);
}

// 戻ったあとに別のページを開くと、進む先はなくなる。戻ると、戻る前に開いていたページが開く。
#[test]
fn opening_a_page_after_going_back_drops_the_pages_ahead() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");
    open(&mut h, "/pages/b");
    back(&mut h);

    open(&mut h, "/pages/c");

    assert!(!h.view().history.can_go_forward);
    back(&mut h);
    assert_eq!(shown(&h), "a");
}

// コアが置き換えで開くように頼んだページは、今のページの代わりに履歴に入る。戻ると、その前のページが開く。
#[test]
fn a_page_the_core_asks_to_open_in_place_takes_the_place_of_the_page_on_screen() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");
    open(&mut h, "/pages/old");
    h.send(Event::PageRedirected { path: "new".into() });
    let asked = h
        .navigations
        .last()
        .expect("the core asks to open the page");
    assert!(asked.replace);

    let url = asked.url.clone();
    open(&mut h, &url);

    assert_eq!(shown(&h), "new");
    back(&mut h);
    assert_eq!(shown(&h), "a");
}

// 今のページをもう一度開いても、履歴には増えない。戻るで、同じページに留まることはない。
#[test]
fn opening_the_page_on_screen_again_adds_nothing_to_the_history() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");

    open(&mut h, "/pages/a");

    assert!(!h.view().history.can_go_back);
}

// 履歴は100件までである。それを超えると、いちばん古いページから消える。
#[test]
fn the_history_keeps_the_last_hundred_pages() {
    let mut h = Harness::new();
    for number in 0..=100 {
        open(&mut h, &format!("/pages/p{number}"));
    }

    for _ in 0..99 {
        back(&mut h);
    }

    assert_eq!(shown(&h), "p1");
    assert!(!h.view().history.can_go_back);
}

// URLの変わったことだけを伝えるシェル（Web版）では、履歴は空のままである。
#[test]
fn a_route_from_a_shell_with_its_own_history_keeps_nothing() {
    let mut h = Harness::new();
    for name in ["a", "b"] {
        h.send(Event::Route {
            route: crate::app::Route::Page {
                name: name.into(),
                fragment: None,
            },
            today: TODAY.into(),
        });
    }

    assert!(!h.view().history.can_go_back);
}

// 戻る先も進む先もないときに戻るや進むを押しても、今のページのままである。
#[test]
fn going_back_or_forward_with_nowhere_to_go_keeps_the_page_on_screen() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");

    back(&mut h);
    h.send(Event::Forward {
        today: TODAY.into(),
    });

    assert_eq!(shown(&h), "a");
}

// 何も指さないURLを開いても、画面も履歴も変わらない。
#[test]
fn a_url_that_names_nothing_changes_neither_the_screen_nor_the_history() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");

    open(&mut h, "/");

    assert_eq!(shown(&h), "a");
    assert!(!h.view().history.can_go_back);
}

// 履歴で今開いているURLが、ビューモデルに出る。戻ると、戻った先のURLになる。シェルはこれを次の起動のために覚える。
#[test]
fn the_view_tells_the_url_on_screen_in_the_history() {
    let mut h = Harness::new();
    assert_eq!(h.view().history.url, None);
    open(&mut h, "/pages/a");
    open(&mut h, "/pages/b");

    back(&mut h);

    assert_eq!(h.view().history.url.as_deref(), Some("/pages/a"));
}

/// The calls that made or opened the doc page of the title.
fn made(h: &Harness, title: &str) -> usize {
    h.api
        .calls_named("GetOrCreateDoc")
        .into_iter()
        .filter(|op| matches!(op, ApiOp::GetOrCreateDoc { title: t, .. } if t == title))
        .count()
}

// 戻る先のページが同期で消えていれば、そのページを作らずに、さらに前のページを開く（#763）。
#[test]
fn going_back_skips_a_page_that_is_gone_and_makes_none() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");
    open(&mut h, "/pages/gone");
    open(&mut h, "/pages/c");
    let made_before = made(&h, "gone");
    h.api
        .answer_next("FindPage", Ok(ApiOutput::FoundPage(None)));

    back(&mut h);

    assert_eq!(shown(&h), "a");
    assert_eq!(
        made(&h, "gone"),
        made_before,
        "the gone page is not made again"
    );
    assert!(!h.view().history.can_go_back);
}

fn forward(h: &mut Harness) {
    h.send(Event::Forward {
        today: TODAY.into(),
    });
}

fn gone_next(h: &mut Harness) {
    h.api
        .answer_next("FindPage", Ok(ApiOutput::FoundPage(None)));
}

// 進む先のページが同期で消えていれば、そのページを作らずに、さらに先のページを開く。
#[test]
fn going_forward_skips_a_page_that_is_gone_and_makes_none() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");
    open(&mut h, "/pages/gone");
    open(&mut h, "/pages/c");
    back(&mut h);
    back(&mut h);
    let made_before = made(&h, "gone");
    gone_next(&mut h);

    forward(&mut h);

    assert_eq!(shown(&h), "c");
    assert_eq!(made(&h, "gone"), made_before);
    assert!(!h.view().history.can_go_forward);
    assert!(h.view().history.can_go_back);
}

// 前のページがすべて消えていれば、画面は今のページのままで、もう戻れない。
#[test]
fn going_back_stays_when_every_page_before_is_gone() {
    let mut h = Harness::new();
    open(&mut h, "/pages/gone-1");
    open(&mut h, "/pages/gone-2");
    open(&mut h, "/pages/c");
    gone_next(&mut h);
    gone_next(&mut h);

    back(&mut h);

    assert_eq!(shown(&h), "c");
    assert_eq!(h.view().history.url.as_deref(), Some("/pages/c"));
    assert!(!h.view().history.can_go_back);
}

// 先のページがすべて消えていれば、画面は今のページのままで、もう進めない。
#[test]
fn going_forward_stays_when_every_page_ahead_is_gone() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");
    open(&mut h, "/pages/gone");
    back(&mut h);
    gone_next(&mut h);

    forward(&mut h);

    assert_eq!(shown(&h), "a");
    assert_eq!(h.view().history.url.as_deref(), Some("/pages/a"));
    assert!(!h.view().history.can_go_forward);
}

// ページを探している間に別のページを開けば、探した答えは画面を変えない。
#[test]
fn an_answer_for_a_page_the_history_has_left_is_not_shown() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");
    open(&mut h, "/pages/b");
    h.hold("FindPage");
    back(&mut h);

    open(&mut h, "/pages/c");
    h.release_held();

    assert_eq!(shown(&h), "c");
}

// サーバーがページの有無を答えられなければ、戻る先のページを今までどおり開く。
#[test]
fn going_back_opens_the_page_when_the_server_cannot_tell() {
    let mut h = Harness::new();
    open(&mut h, "/pages/a");
    open(&mut h, "/pages/b");
    h.api.fail_next("FindPage", "offline");

    back(&mut h);

    assert_eq!(shown(&h), "a");
}

// ジャーナルはジャーナルの名前で、docページはフォルダとタイトルで探す。
#[test]
fn a_journal_is_looked_up_by_its_name_and_a_doc_page_by_its_folder_and_title() {
    let mut h = Harness::new();
    open(&mut h, "/pages/2026-02-01");
    open(&mut h, "/pages/d/a");
    open(&mut h, "/pages/c");

    back(&mut h);
    back(&mut h);

    let asked: Vec<_> = h.api.calls_named("FindPage").into_iter().cloned().collect();
    assert_eq!(
        asked,
        vec![
            ApiOp::FindPage {
                journal: None,
                title: "a".into(),
                dir_path: "d/".into(),
            },
            ApiOp::FindPage {
                journal: Some("2026-02-01".into()),
                title: String::new(),
                dir_path: String::new(),
            },
        ]
    );
    assert_eq!(shown(&h), "2026-02-01");
}

// 今日のジャーナルには、探さずに戻る。今日のジャーナルは、開くときに作るページだからである。
#[test]
fn going_back_to_today_opens_it_without_looking_it_up() {
    let mut h = Harness::new();
    open(&mut h, "/today");
    open(&mut h, "/pages/b");

    back(&mut h);

    assert!(!h.api.called("FindPage"));
    assert_eq!(h.view().history.url.as_deref(), Some("/today"));
    assert!(matches!(h.model.screen, Screen::Journal(_)));
}
