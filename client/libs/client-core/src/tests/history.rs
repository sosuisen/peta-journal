//! The history of the pages that were opened (ADR 035).

use crate::app::{Event, Screen};
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
