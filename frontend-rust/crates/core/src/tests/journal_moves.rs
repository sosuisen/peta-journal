use crate::app::{Event, JournalState, Screen};
use crate::effects::{ApiOp, ApiOutput};
use crate::harness::{create_calls, item, move_calls, Harness};
use crate::model::{Page, PageWithItems};
use crate::testing::item_with;

fn journal_item(id: &str, level: i32, page: &str, name: &str) -> crate::model::Item {
    item_with(id, level, |i| {
        i.page_id = page.into();
        i.name = name.into();
        i.item_type = "journal".into();
        i.body = id.into();
    })
}

/// The journal of 2026-02-10 shown, with 2026-02-09 loaded below it.
fn harness() -> Harness {
    let mut h = Harness::new();
    h.model.state.set_items(vec![
        journal_item("a", 1, "p10", "2026-02-10"),
        journal_item("a1", 2, "p10", "2026-02-10"),
        journal_item("b", 1, "p10", "2026-02-10"),
        journal_item("c", 1, "p09", "2026-02-09"),
        journal_item("d", 1, "p09", "2026-02-09"),
    ]);
    h.model.screen = Screen::Journal(JournalState {
        page_name: "2026-02-10".into(),
        current_date: "2026-02-10".into(),
        page: Some(Page {
            id: "p10".into(),
            name: "2026-02-10".into(),
            page_type: "journal".into(),
            ..Default::default()
        }),
        ..Default::default()
    });
    h
}

/// A move to the journal of a day, as the request carried it.
struct JournalMove {
    date: String,
    item_ids: Vec<String>,
    position: String,
    body: Option<String>,
}

fn journal_moves(h: &Harness) -> Vec<JournalMove> {
    h.api
        .calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::MoveItemsToJournal {
                file_name,
                item_ids,
                position,
                body,
            } => Some(JournalMove {
                date: file_name.clone(),
                item_ids: item_ids.clone(),
                position: position.to_api(),
                body: body.clone(),
            }),
            _ => None,
        })
        .collect()
}

// 最上位の項目を上へ動かすと、子ごと翌日のジャーナルの末尾へ、1つの要求で移動される。ジャーナルの取得や作成、空の項目の削除は別には送らない。翌日へ遷移する。
#[test]
fn moving_the_topmost_item_up_takes_it_with_its_children_to_the_end_of_the_next_day() {
    let mut h = harness();
    h.send(Event::MoveUp {
        item_id: "a".into(),
    });
    let names: Vec<&str> = h.api.calls.iter().map(crate::harness::op_name).collect();
    assert_eq!(names, ["MoveItemsToJournal"]);
    let sent = &journal_moves(&h)[0];
    assert_eq!(sent.date, "2026-02-11");
    assert_eq!(sent.item_ids, ["a", "a1"]);
    assert_eq!(sent.position, "last");
    assert_eq!(sent.body, None);
    // The item left the shown page: the journal navigates to the next day
    assert_eq!(h.navigations.last().unwrap().url, "/pages/2026-02-11");
    assert!(!h.model.move_in_progress);
}

// 最下位の項目を下へ動かすと前日のジャーナルの先頭へ移動される。遷移はせず表示中のジャーナルが再読み込みされる。
#[test]
fn moving_the_bottommost_item_down_takes_it_to_the_start_of_the_previous_day() {
    let mut h = harness();
    h.send(Event::MoveDown {
        item_id: "d".into(),
    });
    assert!(
        matches!(&h.api.calls[0], ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-08")
    );
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["d".to_string()],
            "page-2026-02-08".to_string(),
            "first".to_string()
        )]
    );
    assert!(
        create_calls(&h.api).is_empty(),
        "c stays on the source page"
    );
    // The journal reloads instead of navigating
    assert!(h.navigations.is_empty());
    assert!(h.api.calls.iter().any(
        |c| matches!(c, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-10")
    ));
}

// 移動元のページが空になる場合は、空の項目が移動元に残される。
#[test]
fn leaves_an_empty_item_behind_when_the_source_page_would_become_empty() {
    let mut h = Harness::new();
    h.model.state.set_items(vec![
        journal_item("a", 1, "p10", "2026-02-10"),
        journal_item("c", 1, "p09", "2026-02-09"),
    ]);
    h.model.screen = Screen::Journal(JournalState {
        current_date: "2026-02-10".into(),
        ..Default::default()
    });
    h.send(Event::MoveDown {
        item_id: "c".into(),
    });
    assert_eq!(move_calls(&h.api)[0].2, "first");
    assert_eq!(create_calls(&h.api)[0].0, "p09");
}

// 前日ページの最後より上にある項目を下へ動かすと、ジャーナルは作られずそのページ内で移動される。
#[test]
fn an_item_above_the_last_one_stays_on_its_page() {
    let mut h = harness();
    h.send(Event::MoveDown {
        item_id: "c".into(),
    });
    assert!(!h.api.calls.iter().any(
        |c| matches!(c, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-08")
    ));
    assert_eq!(move_calls(&h.api)[0].1, "p09");
}

// 表示中ページの最下位でない項目を下へ動かすと、ジャーナルは作られずそのページ内で移動される。
#[test]
fn an_item_in_the_middle_of_its_page_stays() {
    let mut h = harness();
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });
    assert!(!h.api.called("GetOrCreateJournal"));
    // Moved within its own page instead
    assert_eq!(move_calls(&h.api)[0].1, "p10");
}

// 2番目にあるレベル2の項目を上へ動かすと、読み込み済みの翌日の最後のルート項目の下へ移動される。ページは作られない。
#[test]
fn a_level_2_item_in_second_position_moves_up_under_the_last_root_of_the_loaded_next_day() {
    let mut h = Harness::new();
    h.model.state.set_items(vec![
        journal_item("n", 1, "p11", "2026-02-11"),
        journal_item("a", 1, "p10", "2026-02-10"),
        journal_item("a1", 2, "p10", "2026-02-10"),
    ]);
    h.model.screen = Screen::Journal(JournalState {
        current_date: "2026-02-10".into(),
        ..Default::default()
    });
    h.send(Event::MoveUp {
        item_id: "a1".into(),
    });
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["a1".to_string()],
            "p11".to_string(),
            "after:n".to_string()
        )]
    );
    // No page is created for the move; the journal only reloads itself afterwards
    assert!(!h.api.calls.iter().any(
        |c| matches!(c, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-11")
    ));
}

// 翌日へ遷移した後、動かした項目はひと息置いて編集中になり、そのエディタにフォーカスが移る。
#[test]
fn after_the_move_the_moved_item_is_edited_on_the_next_day() {
    let mut h = harness();
    h.send(Event::MoveUp {
        item_id: "a".into(),
    });
    assert!(!h.model.state.is_editing("a"));

    h.fire_timers();
    assert!(h.model.state.is_editing("a"));
    assert_eq!(h.focused_editor_requests(), [Some("a".to_string())]);
}

// 最下位にあるレベル2の項目を下へ動かすと、読み込み済みの前日の最初のルート項目の下へ移動される。
#[test]
fn a_level_2_item_at_the_bottom_moves_down_under_the_first_root_of_the_previous_day() {
    let mut h = Harness::new();
    h.model.state.set_items(vec![
        journal_item("a", 1, "p10", "2026-02-10"),
        journal_item("a1", 2, "p10", "2026-02-10"),
        journal_item("c", 1, "p09", "2026-02-09"),
        journal_item("c1", 2, "p09", "2026-02-09"),
        journal_item("d", 1, "p09", "2026-02-09"),
    ]);
    h.model.screen = Screen::Journal(JournalState {
        current_date: "2026-02-10".into(),
        ..Default::default()
    });
    h.send(Event::MoveDown {
        item_id: "a1".into(),
    });
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["a1".to_string()],
            "p09".to_string(),
            "after:c".to_string()
        )]
    );
    // No page is created for the move; the journal only reloads itself afterwards
    assert!(!h.api.calls.iter().any(
        |c| matches!(c, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-09")
    ));
}

// 日付をまたぐ移動で作られたジャーナルは最近のページに出るので、最近のページが読み直される。既にあったジャーナルでは読み直さない。
#[test]
fn a_journal_made_by_a_move_refreshes_the_recent_pages() {
    for created in [true, false] {
        let mut h = harness();
        h.api.answer_next(
            "GetOrCreateJournal",
            Ok(ApiOutput::Page(PageWithItems {
                page: Page {
                    id: "p11".into(),
                    name: "2026-02-11".into(),
                    page_type: "journal".into(),
                    ..Default::default()
                },
                items: vec![item("empty", 1, "")],
                created,
                transcluded_trees: None,
            })),
        );

        h.send(Event::MoveUp {
            item_id: "a".into(),
        });

        assert_eq!(
            h.api.called("GetRecentPages"),
            created,
            "created: {created}"
        );
    }
}

// 入力した直後に別の日へ動かすと、自動保存を待たずに本文が保存されてから移動する。別の日への移動は、まだ1つの要求になっていない。
#[test]
fn text_typed_just_before_a_move_to_another_day_is_saved_before_the_item_moves() {
    let mut h = harness();
    h.model.state.add_editing_item("d");
    h.send(Event::BodyChanged {
        item_id: "d".into(),
        body: "typed".into(),
        now_ms: h.now_ms,
    });
    h.send(Event::MoveDown {
        item_id: "d".into(),
    });
    assert_eq!(
        crate::harness::update_calls(&h.api),
        [("d".into(), Some("typed".into()))]
    );
    let names: Vec<&str> = h.api.calls.iter().map(crate::harness::op_name).collect();
    let at = |name: &str| names.iter().position(|n| *n == name).unwrap();
    assert!(at("UpdateItem") < at("MoveItems"), "{names:?}");
    assert!(
        h.pending_timer_ids().is_empty(),
        "the auto-save is not needed any more"
    );
}
