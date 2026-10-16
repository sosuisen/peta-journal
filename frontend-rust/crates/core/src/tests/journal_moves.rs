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
        page: Some(Page { id: "p10".into(), name: "2026-02-10".into(), page_type: "journal".into(), ..Default::default() }),
        ..Default::default()
    });
    h
}

#[test]
fn moving_the_topmost_item_up_takes_it_with_its_children_to_the_end_of_the_next_day() {
    let mut h = harness();
    h.api.answer_next(
        "GetOrCreateJournal",
        Ok(ApiOutput::Page(PageWithItems {
            page: Page { id: "p11".into(), name: "2026-02-11".into(), page_type: "journal".into(), ..Default::default() },
            items: vec![item("empty", 1, "")],
            ..Default::default()
        })),
    );
    h.send(Event::MoveUp { item_id: "a".into() });
    assert!(matches!(&h.api.calls[0], ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-11"));
    assert_eq!(move_calls(&h.api), [(vec!["a".to_string(), "a1".to_string()], "p11".to_string(), "last".to_string())]);
    // The single empty item of the new target page is replaced
    assert!(matches!(h.api.calls.iter().find(|c| matches!(c, ApiOp::DeleteItem { .. })), Some(ApiOp::DeleteItem { item_id }) if item_id == "empty"));
    // The item left the shown page: the journal navigates to the next day
    assert_eq!(h.navigations.last().unwrap().url, "/pages/2026-02-11");
}

#[test]
fn moving_the_bottommost_item_down_takes_it_to_the_start_of_the_previous_day() {
    let mut h = harness();
    h.send(Event::MoveDown { item_id: "d".into() });
    assert!(matches!(&h.api.calls[0], ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-08"));
    assert_eq!(move_calls(&h.api), [(vec!["d".to_string()], "page-2026-02-08".to_string(), "first".to_string())]);
    assert!(create_calls(&h.api).is_empty(), "c stays on the source page");
    // The journal reloads instead of navigating
    assert!(h.navigations.is_empty());
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-10")));
}

#[test]
fn leaves_an_empty_item_behind_when_the_source_page_would_become_empty() {
    let mut h = Harness::new();
    h.model.state.set_items(vec![journal_item("a", 1, "p10", "2026-02-10"), journal_item("c", 1, "p09", "2026-02-09")]);
    h.model.screen = Screen::Journal(JournalState { current_date: "2026-02-10".into(), ..Default::default() });
    h.send(Event::MoveDown { item_id: "c".into() });
    assert_eq!(move_calls(&h.api)[0].2, "first");
    assert_eq!(create_calls(&h.api)[0].0, "p09");
}

#[test]
fn an_item_above_the_last_one_stays_on_its_page() {
    let mut h = harness();
    h.send(Event::MoveDown { item_id: "c".into() });
    assert!(!h.api.calls.iter().any(|c| matches!(c, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-08")));
    assert_eq!(move_calls(&h.api)[0].1, "p09");
}

#[test]
fn an_item_in_the_middle_of_its_page_stays() {
    let mut h = harness();
    h.send(Event::MoveDown { item_id: "a".into() });
    assert!(!h.api.called("GetOrCreateJournal"));
    // Moved within its own page instead
    assert_eq!(move_calls(&h.api)[0].1, "p10");
}

#[test]
fn a_level_2_item_in_second_position_moves_up_under_the_last_root_of_the_loaded_next_day() {
    let mut h = Harness::new();
    h.model.state.set_items(vec![
        journal_item("n", 1, "p11", "2026-02-11"),
        journal_item("a", 1, "p10", "2026-02-10"),
        journal_item("a1", 2, "p10", "2026-02-10"),
    ]);
    h.model.screen = Screen::Journal(JournalState { current_date: "2026-02-10".into(), ..Default::default() });
    h.send(Event::MoveUp { item_id: "a1".into() });
    assert_eq!(move_calls(&h.api), [(vec!["a1".to_string()], "p11".to_string(), "after:n".to_string())]);
    // No page is created for the move; the journal only reloads itself afterwards
    assert!(!h.api.calls.iter().any(|c| matches!(c, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-02-11")));
}
