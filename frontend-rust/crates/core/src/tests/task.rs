//! Turning an item into a task with Ctrl+Enter.

use crate::app::Event;
use crate::effects::{ApiOp, ApiOutput};
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

// 日次ジャーナルの項目をタスクにするとき、要求は切り替えの1つだけである。予定日（plan）はサーバーが決め、答えの値が項目に出る。
#[test]
fn an_item_that_becomes_a_task_on_a_daily_journal_shows_the_plan_the_server_answered() {
    let mut h = Harness::with_items(vec![item_with("a", 1, |i| {
        i.item_type = "journal".into();
        i.name = "2026-02-10".into();
    })]);
    h.api.answer_next(
        "CycleTask",
        Ok(ApiOutput::Item(item_with("a", 1, |i| {
            i.types = vec!["task".into()];
            i.properties
                .insert("plan".into(), serde_json::json!("2026-02-10"));
        }))),
    );
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });
    assert_eq!(
        h.api.calls,
        [ApiOp::CycleTask {
            item_id: "a".into()
        }]
    );
    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().prop_str("plan"),
        Some("2026-02-10")
    );
}
