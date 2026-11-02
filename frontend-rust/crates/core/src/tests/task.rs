//! Turning an item into a task with Ctrl+Enter.

use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::{item, Harness};
use crate::testing::item_with;

// タスク状態の切り替えで次の状態が保存され、Undo に記録される。
#[test]
fn cycle_task_saves_the_next_task_state_and_records_it_for_undo() {
    let mut h = Harness::with_items(vec![item("a", 1, "do it")]);
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });
    assert!(h.api.called("CycleTask"));
    assert!(h.model.state.get_item_by_id("a").unwrap().is_task());
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "cycle-task");
}

// 日次ジャーナル上の項目がタスクになると、その日の日付が plan プロパティとして保存される。
#[test]
fn an_item_that_becomes_a_task_on_a_daily_journal_is_planned_for_that_day() {
    let mut h = Harness::with_items(vec![item_with("a", 1, |i| {
        i.item_type = "journal".into();
        i.name = "2026-02-10".into();
    })]);
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::UpsertItemProperty { key, value, .. } if key == "plan" && value == "2026-02-10")));
    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().prop_str("plan"),
        Some("2026-02-10")
    );
}
