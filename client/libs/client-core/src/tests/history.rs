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
