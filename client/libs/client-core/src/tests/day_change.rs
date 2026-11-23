//! A new day while the journal of today is open: the core opens the journal of the new day
//! (#796). The core asks the clock of the shell, so no shell has to watch the date.

use crate::app::{Event, Route};
use crate::effects::ApiOp;
use crate::harness::Harness;
use crate::rest::ClientId;
use crate::view::ScreenView;

/// A shell that started, with the journal of 2026-02-10 open as today.
fn today_open() -> Harness {
    let mut h = Harness::new();
    h.send(Event::Start {
        client_id: ClientId::new(),
    });
    h.send(Event::Route {
        route: Route::Page {
            name: "today".into(),
            fragment: None,
        },
        today: "2026-02-10".into(),
    });
    h
}

/// The days of the journals the server was asked for, oldest first.
fn journals_asked(h: &Harness) -> Vec<String> {
    h.api
        .calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::GetOrCreateJournal { file_name } => Some(file_name.clone()),
            _ => None,
        })
        .collect()
}

fn shown_date(h: &Harness) -> String {
    match h.view().screen {
        ScreenView::Journal(j) => j.current_date,
        other => panic!("not a journal: {other:?}"),
    }
}

// 日付が変わると、`/today`を開いている画面は、新しい日のジャーナルを開く。
#[test]
fn the_journal_of_today_opens_the_new_day_when_the_date_changes() {
    let mut h = today_open();

    h.local_now.day = 11;
    h.fire_timers();

    assert_eq!(journals_asked(&h), ["2026-02-10", "2026-02-11"]);
    assert_eq!(shown_date(&h), "2026-02-11");
}
