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
    h.api.answer_next("FindPage", Ok(ApiOutput::FoundPage(None)));

    back(&mut h);

    assert_eq!(shown(&h), "a");
    assert_eq!(made(&h, "gone"), made_before, "the gone page is not made again");
    assert!(!h.view().history.can_go_back);
}
