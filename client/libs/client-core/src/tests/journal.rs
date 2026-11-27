//! Opening the journal of a day, week, month or year, and moving between them.

use crate::app::{Event, JournalNav, Route};
use crate::effects::{ApiOp, ApiOutput, UiOp};
use crate::harness::Harness;
use crate::model::{JournalBefore, JournalPage, Page, WeekNumbering};
use crate::testing::{item_with, plain_item};
use crate::view::ScreenView;

const TODAY: &str = "2026-02-10";

fn visit(h: &mut Harness, name: &str) {
    h.send(Event::Route {
        route: Route::Page {
            name: name.into(),
            fragment: None,
        },
        today: TODAY.into(),
    });
}

fn journal_page(name: &str) -> Page {
    Page {
        id: format!("page-{name}"),
        name: name.into(),
        page_type: "journal".into(),
        ..Default::default()
    }
}

/// An item of the journal `name`.
fn journal_item(id: &str, name: &str) -> crate::model::Item {
    item_with(id, 1, |i| {
        i.page_id = format!("page-{name}");
        i.item_type = "journal".into();
        i.name = name.into();
    })
}

/// The server's answer to "the journal before": the journal `name` with one item.
fn older_journal(name: &str) -> ApiOutput {
    ApiOutput::JournalBefore(JournalBefore {
        found: true,
        page: Some(journal_page(name)),
        items: vec![journal_item(&format!("item-{name}"), name)],
        transcluded_trees: Default::default(),
    })
}

fn titles(h: &Harness) -> Vec<String> {
    h.ui.iter()
        .filter_map(|op| match op {
            UiOp::SetTitle(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

fn journal_view(h: &Harness) -> crate::view::JournalView {
    match h.view().screen {
        ScreenView::Journal(j) => j,
        other => panic!("not a journal: {other:?}"),
    }
}

fn before_calls(h: &Harness) -> Vec<String> {
    h.api
        .calls
        .iter()
        .filter_map(|c| match c {
            ApiOp::GetMostRecentJournalBefore { before } => Some(before.clone()),
            _ => None,
        })
        .collect()
}

// `today` は今日の日付のジャーナルを開く。タイトルは Today、項目はサーバーのもの、サイドバーの今のページはそのジャーナルになる。
#[test]
fn today_opens_the_journal_of_todays_date() {
    let mut h = Harness::new();
    visit(&mut h, "today");

    assert_eq!(journal_view(&h).current_date, TODAY);
    assert_eq!(journal_view(&h).page_id.as_deref(), Some("page-2026-02-10"));
    assert_eq!(h.items_ids(), ["item-2026-02-10"]);
    assert_eq!(titles(&h), ["Today"]);
    assert_eq!(
        h.model.current_page.as_ref().map(|p| p.id.as_str()),
        Some("page-2026-02-10")
    );
}

// 日付のジャーナルは、その日の日本語表記をタイトルと見出しに持つ。
#[test]
fn a_date_is_titled_in_japanese() {
    let mut h = Harness::new();
    visit(&mut h, "2026-02-09");

    assert_eq!(titles(&h), ["2026年2月9日 月曜日"]);
    assert_eq!(journal_view(&h).display_date, "2026年2月9日 月曜日");
}

// 日次のジャーナルの下には、前のジャーナルが見つからなくなるまで続けて読み込まれる。読み込んだ日は記録され、見出しは日付ごとに付く。
#[test]
fn the_journals_before_are_appended_until_there_is_none() {
    let mut h = Harness::new();
    h.api.answer_next(
        "GetMostRecentJournalBefore",
        Ok(older_journal("2026-02-09")),
    );
    h.api.answer_next(
        "GetMostRecentJournalBefore",
        Ok(older_journal("2026-02-07")),
    );
    visit(&mut h, TODAY);

    assert_eq!(before_calls(&h), ["2026-02-10", "2026-02-09", "2026-02-07"]);
    assert_eq!(
        h.items_ids(),
        ["item-2026-02-10", "item-2026-02-09", "item-2026-02-07"]
    );
    let dates: Vec<Option<String>> = h
        .view()
        .outline
        .rows
        .iter()
        .map(|r| r.date_header.clone())
        .collect();
    assert_eq!(
        dates,
        [
            None,
            Some("2026年2月9日 月曜日".into()),
            Some("2026年2月7日 土曜日".into())
        ]
    );
}

// 週のジャーナルは前の週を読み込み、月と年のジャーナルは前のものを読み込まない。
#[test]
fn a_week_loads_the_weeks_before_and_a_month_or_a_year_loads_nothing_before() {
    let mut h = Harness::new();
    visit(&mut h, "2026-W07");
    assert!(h.api.called("GetMostRecentWeeklyJournalBefore"));
    assert_eq!(titles(&h), ["2026 Week 07"]);

    let mut h = Harness::new();
    visit(&mut h, "2026-02");
    visit(&mut h, "2026");
    assert!(!h.api.called("GetMostRecentJournalBefore"));
    assert!(!h.api.called("GetMostRecentWeeklyJournalBefore"));
    assert_eq!(titles(&h), ["2026年2月", "2026年"]);
}

// ジャーナルを読み込めなければ、画面は空のままである。前のジャーナルの読み込みの失敗は、そこで止まる。
#[test]
fn a_failed_load_leaves_the_journal_empty() {
    let mut h = Harness::new();
    h.api.fail_next("GetOrCreateJournal", "offline");
    visit(&mut h, TODAY);
    assert_eq!(journal_view(&h).page_id, None);
    assert!(h.items_ids().is_empty());

    let mut h = Harness::new();
    h.api.fail_next("GetMostRecentJournalBefore", "offline");
    visit(&mut h, TODAY);
    assert_eq!(h.items_ids(), ["item-2026-02-10"]);
}

// 別の日へ移ったあとに届いた前の日の答えは、表示されず、前の日より古いジャーナルの読み込みも始めない。
#[test]
fn the_answer_for_a_day_left_behind_loads_no_older_journals() {
    let mut h = Harness::new();
    visit(&mut h, "2026-02-05");
    let calls = before_calls(&h).len();

    h.send(Event::JournalLoaded {
        date: "2026-02-09".into(),
        title: "2026年2月9日 月曜日".into(),
        loaded: Box::new(JournalPage {
            page: journal_page("2026-02-09"),
            items: vec![journal_item("item-2026-02-09", "2026-02-09")],
            ..Default::default()
        }),
    });

    assert_eq!(before_calls(&h).len(), calls);
    assert_eq!(h.items_ids(), ["item-2026-02-05"]);
}

// 前の日の下に読み込んでいた古いジャーナルの答えは、別の日へ移ったあとに届いても、今の日の下に足されない。今の日の読み込みも止めない。
#[test]
fn an_older_journal_loaded_for_a_day_left_behind_is_not_appended() {
    let mut h = Harness::new();
    visit(&mut h, "2026-02-05");

    h.send(Event::OlderJournalLoaded {
        journal: "2026-02-10".into(),
        weekly: false,
        target_item_count: 50,
        name: "2026-02-08".into(),
        items: vec![journal_item("item-2026-02-08", "2026-02-08")],
        transcluded_trees: Default::default(),
    });
    h.send(Event::PreviousJournalsLoaded {
        journal: "2026-02-10".into(),
    });

    assert_eq!(h.items_ids(), ["item-2026-02-05"]);
    assert!(!before_calls(&h).contains(&"2026-02-08".to_string()));
}

// 同じジャーナルへの遷移では読み直さない。項目を指すフラグメント付きなら読み直す。
#[test]
fn the_same_journal_is_not_loaded_again_unless_an_item_is_named() {
    let mut h = Harness::new();
    visit(&mut h, TODAY);
    visit(&mut h, TODAY);
    assert_eq!(h.api.calls_named("GetOrCreateJournal").len(), 1);

    h.send(Event::Route {
        route: Route::Page {
            name: TODAY.into(),
            fragment: Some("item-2026-02-10".into()),
        },
        today: TODAY.into(),
    });
    assert_eq!(h.api.calls_named("GetOrCreateJournal").len(), 2);
}

// 遡って読み込んだジャーナルで折りたたまれた項目は、その子を隠す。
#[test]
fn a_collapsed_item_of_an_older_journal_hides_its_children() {
    let name = "2026-02-09";
    let mut parent = journal_item("parent", name);
    parent.collapsed = true;
    let mut child = journal_item("child", name);
    child.level = 2;
    let mut h = Harness::new();
    h.api.answer_next(
        "GetMostRecentJournalBefore",
        Ok(ApiOutput::JournalBefore(JournalBefore {
            found: true,
            page: Some(journal_page(name)),
            items: vec![parent, child],
            transcluded_trees: Default::default(),
        })),
    );
    visit(&mut h, TODAY);

    let rows: Vec<String> = h
        .view()
        .outline
        .rows
        .iter()
        .map(|r| r.item.id.clone())
        .collect();
    assert_eq!(rows, ["item-2026-02-10", "parent"]);
}

// スクロールすると、見出しは一番上に見える項目の日になる。残りの項目が少なくなると、最後の項目の日より前のジャーナルが読み込まれる。
#[test]
fn scrolling_moves_the_header_date_and_loads_more_when_the_end_comes_near() {
    let mut h = Harness::new();
    h.api.answer_next(
        "GetMostRecentJournalBefore",
        Ok(older_journal("2026-02-09")),
    );
    visit(&mut h, TODAY);
    assert_eq!(before_calls(&h), ["2026-02-10", "2026-02-09"]);

    h.send(Event::OutlineScrolled {
        scrolled_from_top: true,
        top_visible_index: 1,
        rendered_start: 0,
        rendered_end: 2,
    });
    assert_eq!(journal_view(&h).display_date, "2026年2月9日 月曜日");
    assert_eq!(before_calls(&h), ["2026-02-10", "2026-02-09", "2026-02-09"]);
}

/// The server's answer to "the journal before": the journal `name` with `parents` items
/// that have two children each. The parents are collapsed or not.
fn journal_of_parents(name: &str, parents: usize, collapsed: bool) -> ApiOutput {
    let mut items = Vec::new();
    for p in 0..parents {
        let mut parent = journal_item(&format!("parent-{name}-{p}"), name);
        parent.collapsed = collapsed;
        items.push(parent);
        for c in 0..2 {
            let mut child = journal_item(&format!("child-{name}-{p}-{c}"), name);
            child.level = 2;
            items.push(child);
        }
    }
    ApiOutput::JournalBefore(JournalBefore {
        found: true,
        page: Some(journal_page(name)),
        items,
        transcluded_trees: Default::default(),
    })
}

/// The shell shows the last rows of the outline, at most 30 of them, as after a scroll to
/// the end.
fn scroll_to_the_end(h: &mut Harness) {
    let rows = h.view().outline.rows.len();
    let first = rows.saturating_sub(30);
    h.send(Event::OutlineScrolled {
        scrolled_from_top: first > 0,
        top_visible_index: first,
        rendered_start: first,
        rendered_end: rows,
    });
}

// 終わりまでのスクロールを繰り返すと、いちばん古いジャーナルまで読み込まれる。折りたたまれて隠れた項目が多いジャーナルを挟んでも、展開されているときと同じである（#1256）。
#[test]
fn scrolling_to_the_end_again_and_again_reaches_the_oldest_journal_past_collapsed_items() {
    for collapsed in [false, true] {
        let mut h = Harness::new();
        for name in ["2026-02-09", "2026-02-08"] {
            h.api.answer_next(
                "GetMostRecentJournalBefore",
                Ok(journal_of_parents(name, 60, collapsed)),
            );
        }
        h.api.answer_next(
            "GetMostRecentJournalBefore",
            Ok(older_journal("2026-02-07")),
        );
        visit(&mut h, TODAY);

        for _ in 0..3 {
            scroll_to_the_end(&mut h);
        }

        assert!(
            h.items_ids().contains(&"item-2026-02-07".to_string()),
            "collapsed: {collapsed}, asked for the journals before {:?}",
            before_calls(&h)
        );
    }
}

// 今日のジャーナルのウィンドウのタイトルは、今日の日付に「(Today)」を付けたものである。ほかの日のジャーナルでは、その日の日付だけである。
#[test]
fn the_window_title_of_a_journal_is_its_date_with_today_marked() {
    let mut h = Harness::new();
    visit(&mut h, "today");
    assert_eq!(h.view().title, "2026年2月10日 火曜日 (Today)");

    visit(&mut h, "2026-02-09");
    assert_eq!(h.view().title, "2026年2月9日 月曜日");
}

// ジャーナルを前の日までスクロールすると、ウィンドウのタイトルはその日の日付になり、「(Today)」は付かない。今日まで戻ると、また付く。
#[test]
fn the_window_title_of_a_journal_follows_the_day_at_the_top() {
    let mut h = Harness::new();
    h.api.answer_next(
        "GetOrCreateJournal",
        Ok(ApiOutput::Journal(JournalPage {
            page: journal_page(TODAY),
            items: vec![journal_item("item-today", TODAY)],
            ..Default::default()
        })),
    );
    h.api.answer_next(
        "GetMostRecentJournalBefore",
        Ok(older_journal("2026-02-09")),
    );
    visit(&mut h, TODAY);

    let scroll_to = |h: &mut Harness, top: usize| {
        h.send(Event::OutlineScrolled {
            scrolled_from_top: top > 0,
            top_visible_index: top,
            rendered_start: 0,
            rendered_end: 2,
        })
    };
    scroll_to(&mut h, 1);
    assert_eq!(h.view().title, "2026年2月9日 月曜日");

    scroll_to(&mut h, 0);
    assert_eq!(h.view().title, "2026年2月10日 火曜日 (Today)");
}

// doc ページや空のアウトラインでは、スクロールは何も読み込まない。
#[test]
fn scrolling_a_doc_page_or_an_empty_outline_loads_nothing() {
    let scroll = || Event::OutlineScrolled {
        scrolled_from_top: true,
        top_visible_index: 0,
        rendered_start: 0,
        rendered_end: 1,
    };
    let mut h = Harness::with_items(vec![plain_item("a", 1, "")]);
    h.send(scroll());
    assert!(h.model.scrolled_from_top);

    let mut h = Harness::new();
    h.api.fail_next("GetOrCreateJournal", "offline");
    visit(&mut h, TODAY);
    h.send(scroll());
    assert_eq!(before_calls(&h), Vec::<String>::new());
}

// 日のジャーナルで前後へ移ると、隣の日が開く。日でも週でもないジャーナルでは何もしない。
#[test]
fn moving_in_a_daily_journal_opens_the_day_before_and_after() {
    let mut h = Harness::new();
    visit(&mut h, TODAY);
    h.send(Event::JournalNav(JournalNav::Prev));
    h.send(Event::JournalNav(JournalNav::Next));
    let urls: Vec<_> = h.navigations.iter().map(|n| n.url.as_str()).collect();
    assert_eq!(urls, ["/pages/2026-02-09", "/pages/2026-02-11"]);

    let mut h = Harness::new();
    visit(&mut h, "2026-02");
    h.send(Event::JournalNav(JournalNav::Prev));
    assert!(h.navigations.is_empty());
}

// 週のジャーナルで前後へ移ると、隣の週が開く。年をまたぐときはその年の週数に従う。
#[test]
fn moving_in_a_weekly_journal_opens_the_week_before_and_after_across_the_year() {
    let mut h = Harness::new();
    h.model.config.week_numbering = WeekNumbering::Iso;
    visit(&mut h, "2026-W01");
    h.send(Event::JournalNav(JournalNav::Prev));
    visit(&mut h, "2026-W53");
    h.send(Event::JournalNav(JournalNav::Next));
    let urls: Vec<_> = h.navigations.iter().map(|n| n.url.as_str()).collect();
    assert_eq!(urls, ["/pages/2025-W52", "/pages/2027-W01"]);
}

// USの週番号では、年をまたぐ週は新しい年の第1週である。前後へ移ると、その週を通る。
#[test]
fn moving_across_new_year_in_us_weeks_passes_week_1_of_the_new_year() {
    let mut h = Harness::new();
    h.model.config.week_numbering = WeekNumbering::Us;
    // 2027-W01 runs from Sunday 2026-12-27 to Saturday 2027-01-02
    visit(&mut h, "2026-W52");
    h.send(Event::JournalNav(JournalNav::Next));
    visit(&mut h, "2027-W02");
    h.send(Event::JournalNav(JournalNav::Prev));
    let urls: Vec<_> = h.navigations.iter().map(|n| n.url.as_str()).collect();
    assert_eq!(urls, ["/pages/2027-W01", "/pages/2027-W01"]);
}

// 設定の URL では設定画面になり、サーバーの設定が読み直される。
#[test]
fn the_settings_url_shows_the_settings() {
    let mut h = Harness::new();
    h.send(Event::Route {
        route: Route::Settings,
        today: TODAY.into(),
    });
    assert_eq!(h.view().screen, ScreenView::Settings);
    assert_eq!(titles(&h), ["Settings"]);
    assert!(h.api.called("GetConfig"));
}

// ヘッダーの Today は今日のジャーナルを開く。
#[test]
fn the_today_button_opens_today() {
    let mut h = Harness::new();
    h.send(Event::NavigateToToday);
    assert_eq!(h.navigations.last().unwrap().url, "/today");
}

// 週のジャーナルをスクロールして残りが少なくなると、前の週が読み込まれる。
#[test]
fn scrolling_a_weekly_journal_loads_the_week_before() {
    let mut h = Harness::new();
    h.api.answer_next(
        "GetOrCreateJournal",
        Ok(ApiOutput::Journal(JournalPage {
            page: journal_page("2026-W07"),
            items: vec![journal_item("w", "2026-W07")],
            created: false,
            transcluded_trees: Default::default(),
        })),
    );
    visit(&mut h, "2026-W07");
    assert_eq!(
        h.api.calls_named("GetMostRecentWeeklyJournalBefore").len(),
        1
    );

    h.send(Event::OutlineScrolled {
        scrolled_from_top: true,
        top_visible_index: 0,
        rendered_start: 0,
        rendered_end: 1,
    });
    assert_eq!(
        h.api.calls_named("GetMostRecentWeeklyJournalBefore").len(),
        2
    );
}

// 開いたことで作られたジャーナルは最近のページに出るので、最近のページが読み直される。既にあったジャーナルでは読み直さない。
#[test]
fn a_journal_made_by_opening_it_refreshes_the_recent_pages() {
    for created in [true, false] {
        let mut h = Harness::new();
        h.api.answer_next(
            "GetOrCreateJournal",
            Ok(ApiOutput::Journal(JournalPage {
                page: journal_page(TODAY),
                items: vec![journal_item("i", TODAY)],
                created,
                transcluded_trees: Default::default(),
            })),
        );

        visit(&mut h, "today");

        assert_eq!(
            h.api.called("GetRecentPages"),
            created,
            "created: {created}"
        );
    }
}

// 別の日へ移ったあとに届いた、前の日のジャーナルの答えは、表示されない。
#[test]
fn the_answer_for_a_day_the_user_has_moved_on_from_is_not_shown() {
    let mut h = Harness::new();
    visit(&mut h, "2026-02-09");
    let shown = h.items_ids();
    let titles_before = titles(&h);

    h.send(Event::JournalLoaded {
        date: "2026-02-08".into(),
        title: "old".into(),
        loaded: Box::new(JournalPage {
            page: journal_page("2026-02-08"),
            items: vec![journal_item("old1", "2026-02-08")],
            created: false,
            transcluded_trees: Default::default(),
        }),
    });

    assert_eq!(h.items_ids(), shown);
    assert_eq!(titles(&h), titles_before);
    assert_ne!(journal_view(&h).page_id.as_deref(), Some("page-2026-02-08"));
}

// 前のジャーナルの読み込みは、アウトラインの項目が100個に届いたところで止まる。
#[test]
fn the_journals_before_are_no_longer_loaded_once_the_outline_holds_enough_items() {
    let mut h = Harness::new();
    let name = "2026-02-09";
    h.api.answer_next(
        "GetMostRecentJournalBefore",
        Ok(ApiOutput::JournalBefore(JournalBefore {
            found: true,
            page: Some(journal_page(name)),
            items: (0..100)
                .map(|i| journal_item(&format!("item-{i}"), name))
                .collect(),
            transcluded_trees: Default::default(),
        })),
    );
    h.api.answer_next(
        "GetMostRecentJournalBefore",
        Ok(older_journal("2026-02-07")),
    );
    visit(&mut h, TODAY);

    assert_eq!(before_calls(&h), ["2026-02-10"]);
    assert_eq!(h.items_ids().len(), 101);
}
