use crate::app::{Event, JournalState, Screen};
use crate::effects::{ApiOp, ApiOutput};
use crate::harness::{create_calls, item, move_calls, typed, Harness};
use crate::model::{MovedToJournal, Page};
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
                position: position.to_string(),
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

// 最下位の項目を下へ動かすと、前日のジャーナルの先頭へ1つの要求で移動される。遷移はせず、表示中のジャーナルが再読み込みされる。
#[test]
fn moving_the_bottommost_item_down_takes_it_to_the_start_of_the_previous_day() {
    let mut h = harness();
    h.send(Event::MoveDown {
        item_id: "d".into(),
    });
    let sent = &journal_moves(&h)[0];
    assert_eq!(sent.date, "2026-02-08");
    assert_eq!(sent.item_ids, ["d"]);
    assert_eq!(sent.position, "first");
    assert!(
        create_calls(&h.api).is_empty(),
        "the server keeps the page non-empty"
    );
    // The journal reloads instead of navigating
    assert!(h.navigations.is_empty());
    assert!(h.api.calls.iter().any(
        |c| matches!(c, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-10")
    ));
    assert!(!h.model.move_in_progress);
}

// 前日ページの最後より上にある項目を下へ動かすと、ジャーナルは作られずそのページ内で移動される。
#[test]
fn an_item_above_the_last_one_stays_on_its_page() {
    let mut h = harness();
    h.send(Event::MoveDown {
        item_id: "c".into(),
    });
    assert!(journal_moves(&h).is_empty());
    assert_eq!(move_calls(&h.api)[0].1, "p09");
}

// 表示中ページの最下位でない項目を下へ動かすと、ジャーナルは作られずそのページ内で移動される。
#[test]
fn an_item_in_the_middle_of_its_page_stays() {
    let mut h = harness();
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });
    assert!(journal_moves(&h).is_empty());
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
    let sent = &journal_moves(&h)[0];
    assert_eq!(sent.date, "2026-02-11");
    assert_eq!(sent.item_ids, ["a1"]);
    assert_eq!(sent.position, "after:n");
    assert!(move_calls(&h.api).is_empty());
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
    let sent = &journal_moves(&h)[0];
    assert_eq!(sent.date, "2026-02-09");
    assert_eq!(sent.item_ids, ["a1"]);
    assert_eq!(sent.position, "after:c");
    assert!(move_calls(&h.api).is_empty());
}

// 日付をまたぐ移動で作られたジャーナルは最近のページに出るので、最近のページが読み直される。既にあったジャーナルでは読み直さない。
#[test]
fn a_journal_made_by_a_move_refreshes_the_recent_pages() {
    for created in [true, false] {
        let mut h = harness();
        h.api.answer_next(
            "MoveItemsToJournal",
            Ok(ApiOutput::MovedToJournal(MovedToJournal {
                page: Page {
                    id: "p11".into(),
                    name: "2026-02-11".into(),
                    page_type: "journal".into(),
                    ..Default::default()
                },
                items: vec![item("a", 1, ""), item("a1", 2, "")],
                created,
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

// 入力した直後に別の日へ動かすと、打ったばかりの文は移動の要求に入って届く。保存を別には送らず、その自動保存は取り消される。
#[test]
fn text_typed_just_before_a_move_to_another_day_travels_in_the_move_itself() {
    let mut h = harness();
    typed(&mut h, "d", "typed");
    h.send(Event::MoveDown {
        item_id: "d".into(),
    });
    assert_eq!(journal_moves(&h)[0].body, Some("typed".to_string()));
    assert!(
        h.pending_timer_ids().is_empty(),
        "the auto-save is not needed any more"
    );
    h.fire_timers();
    assert!(crate::harness::update_calls(&h.api).is_empty());
}
// トランスクルージョンのルートに打って、まだ保存していない文も、別の日への移動の要求に入って届く。サーバーはその文を元の項目に書く。保存を別には送らない（#734）。
#[test]
fn text_typed_into_a_transclusion_root_travels_in_a_move_to_another_day() {
    let mut h = harness();
    h.model.state.update_item("d", |i| {
        i.transclude_item_id = Some("s".into());
        i.body = "shown".into();
    });
    let mut trees = crate::model::TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, "shown")]);
    h.model.state.set_transcluded_trees(trees);
    typed(&mut h, "d", "shown typed");

    h.send(Event::MoveDown {
        item_id: "d".into(),
    });

    assert_eq!(journal_moves(&h)[0].body, Some("shown typed".to_string()));
    h.fire_timers();
    assert!(crate::harness::update_calls(&h.api).is_empty());
}

// 別の日への移動が断られると、画面は変わらず、要求に入れた文はもう一度自動保存にかけられる。次の移動を始められる。
#[test]
fn after_a_refused_move_to_another_day_nothing_changes_and_the_typed_text_is_auto_saved_again() {
    let mut h = harness();
    typed(&mut h, "d", "typed");
    h.api.fail_next("MoveItemsToJournal", "offline");
    h.send(Event::MoveDown {
        item_id: "d".into(),
    });
    assert_eq!(h.items_ids(), ["a", "a1", "b", "c", "d"]);
    assert!(h.navigations.is_empty());
    assert!(
        !h.api.called("GetOrCreateJournal"),
        "the journal is not read again"
    );
    assert!(!h.model.move_in_progress);
    h.fire_timers();
    assert_eq!(
        crate::harness::update_calls(&h.api),
        [("d".into(), Some("typed".into()))]
    );
}
