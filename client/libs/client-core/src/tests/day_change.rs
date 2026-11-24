//! A new day while the journal of today is open: the core opens the journal of the new day
//! (#796). The core asks the clock of the shell, so no shell has to watch the date.

use crate::app::{Event, Route};
use crate::effects::ApiOp;
use crate::harness::Harness;
use crate::rest::ClientId;
use crate::view::ScreenView;
use crate::EditPlace;

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

// 日付が変わらなければ、時計を見たあとも、サーバーには何も頼まない。
#[test]
fn nothing_is_asked_while_the_date_stays_the_same() {
    let mut h = today_open();
    let calls = h.api.calls.len();

    h.fire_timers();
    h.fire_timers();

    assert_eq!(h.api.calls.len(), calls);
    assert_eq!(shown_date(&h), "2026-02-10");
}

// 時計は、決まった間隔でくり返し見る。2回目の確かめで日付が変わっても、新しい日を開く。
#[test]
fn the_clock_is_looked_at_again_and_again() {
    let mut h = today_open();

    h.fire_timers();
    h.local_now.day = 11;
    h.fire_timers();

    assert_eq!(shown_date(&h), "2026-02-11");
}

// 項目を編集している間は、日付が変わっても開き直さない。編集が終わったあとの確かめで、新しい日を開く。
#[test]
fn an_item_being_edited_keeps_the_old_day_until_the_edit_ends() {
    let mut h = today_open();
    h.send(Event::EditRequested {
        item_id: "item-2026-02-10".into(),
        place: EditPlace::Markdown(0),
    });

    h.local_now.day = 11;
    h.fire_timers();
    assert_eq!(shown_date(&h), "2026-02-10");

    h.send(Event::EditFinished {
        item_id: "item-2026-02-10".into(),
    });
    h.fire_timers();
    assert_eq!(shown_date(&h), "2026-02-11");
}

// 日付が変わっても、`/today`ではない日付のページは開き直さない。今日の日付は新しい日になる。
#[test]
fn a_journal_opened_by_its_date_stays_when_the_date_changes() {
    let mut h = today_open();
    h.send(Event::Route {
        route: Route::Page {
            name: "2026-02-10".into(),
            fragment: None,
        },
        today: "2026-02-10".into(),
    });

    h.local_now.day = 11;
    h.fire_timers();

    assert_eq!(journals_asked(&h), ["2026-02-10", "2026-02-10"]);
    assert_eq!(shown_date(&h), "2026-02-10");
    assert_eq!(h.model.today, "2026-02-11");
}

// シェルが「日付が変わったかもしれない」と知らせると（スリープから戻ったときなど）、次の確かめを待たずに、新しい日を開く。
#[test]
fn a_shell_that_tells_of_a_possible_new_day_opens_it_without_waiting() {
    let mut h = today_open();

    h.local_now.day = 11;
    h.send(Event::DateMayHaveChanged);

    assert_eq!(journals_asked(&h), ["2026-02-10", "2026-02-11"]);
    assert_eq!(shown_date(&h), "2026-02-11");
}

// シェルの知らせは、時計を見る確かめを増やさない。知らせのあとも、待っている確かめは1つである。
#[test]
fn a_tell_of_a_possible_new_day_keeps_one_look_at_the_clock_waiting() {
    let mut h = today_open();
    let waiting = h.pending_timer_ids().len();

    h.send(Event::DateMayHaveChanged);
    h.send(Event::DateMayHaveChanged);

    assert_eq!(h.pending_timer_ids().len(), waiting);
}
