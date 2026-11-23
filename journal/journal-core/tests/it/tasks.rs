//! Tasks: the cycle of their states, the day they are planned for, and the lists of them by
//! type.

use serde_json::{json, Value};

use crate::common::items::{answer, has_type, id};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Takes the item to its next task state, and gives the item of the answer
    fn cycle(&self, item_id: &str) -> Value {
        answer(
            self.core
                .journal
                .cycle_task(item_id, &by_the_client())
                .unwrap()
                .item,
        )
    }

    /// The ids of the items that have the types, without those that have the property
    fn ids_by_types(&self, types: &[&str], without_property: Option<&str>) -> Vec<String> {
        let types: Vec<String> = types.iter().map(|t| t.to_string()).collect();
        self.core
            .journal
            .items_by_types(&types, without_property)
            .unwrap()
            .items
            .into_iter()
            .map(|item| item.item.id)
            .collect()
    }
}

fn has_property(item: &Value, key: &str) -> bool {
    item["properties"].get(key).is_some()
}

// cycle-taskは、タスク、実行中、完了を経て通常の項目に戻る。
#[test]
fn cycle_task_goes_through_task_running_done_and_back_to_normal() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-task");
    let item = id(&notes.fill(&page, &["buy milk"])[0]);

    notes.cycle(&item);
    let task = notes.items(&page).remove(0);
    assert!(has_type(&task, "task"));
    assert!(!has_property(&task, "begin"));

    // The answer carries the times it just set, so the frontend can show them at once
    let running = notes.cycle(&item);
    assert!(has_property(&running, "begin"), "answer: {running}");
    assert!(has_property(&notes.items(&page)[0], "begin"));

    let done = notes.cycle(&item);
    assert!(has_property(&done, "end"), "answer: {done}");
    assert!(has_property(&notes.items(&page)[0], "end"));

    notes.cycle(&item);
    let normal = notes.items(&page).remove(0);
    assert!(!has_type(&normal, "task"));
    assert!(!has_property(&normal, "begin"));
    assert!(!has_property(&normal, "end"));
}

// 日のジャーナルの項目をタスクにすると、同じ要求で、その日が予定日（plan）になる。
#[test]
fn an_item_that_becomes_a_task_on_a_day_is_planned_for_that_day() {
    let notes = Core::new();
    let day = notes.open_journal("2050-04-10");
    let item = id(&day["items"][0]);

    let task = notes.cycle(&item);

    assert!(has_type(&task, "task"));
    assert_eq!(task["properties"]["plan"], json!("2050-04-10"));
    let stored = notes.items(day["page"]["id"].as_str().unwrap()).remove(0);
    assert_eq!(stored["properties"]["plan"], json!("2050-04-10"));
}

// ページの項目と、週のジャーナルの項目は、タスクにしても予定日が付かない。
#[test]
fn an_item_that_becomes_a_task_on_a_page_or_a_week_is_not_planned() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-task-unplanned");
    let on_page = id(&notes.fill(&page, &["on a page"])[0]);
    let week = notes.open_journal("2050-W30");
    let on_week = id(&week["items"][0]);

    for item in [on_page, on_week] {
        let task = notes.cycle(&item);
        assert!(has_type(&task, "task"));
        assert!(!has_property(&task, "plan"), "{}", task["properties"]);
    }
}

// 予定日が書かれるのは、タスクになるときだけである。あとで変えた予定日は、開始や完了では書き直されない。もう一度タスクにすると、その日に戻る。
#[test]
fn the_plan_is_written_only_when_the_item_becomes_a_task() {
    let notes = Core::new();
    let day = notes.open_journal("2050-04-11");
    let item = id(&day["items"][0]);
    let plan_of = |item: &Value| item["properties"]["plan"].clone();
    notes.cycle(&item);
    notes
        .set_item_property(
            &item,
            "plan",
            json!({ "valueType": "date", "value": "2050-05-01" }),
        )
        .unwrap();

    let running = notes.cycle(&item);
    let done = notes.cycle(&item);
    let normal = notes.cycle(&item);
    let task_again = notes.cycle(&item);

    assert_eq!(plan_of(&running), json!("2050-05-01"));
    assert_eq!(plan_of(&done), json!("2050-05-01"));
    assert_eq!(plan_of(&normal), json!("2050-05-01"));
    assert_eq!(plan_of(&task_again), json!("2050-04-11"));
}

// 型での一覧は、指定したプロパティを持つ項目を除ける。終わったタスク（end）を除いた一覧に使う。
#[test]
fn items_listed_by_type_can_leave_out_those_with_a_property() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-task-list-open");
    let rows = notes.fill(&page, &["running task", "done task"]);
    let (running, done) = (id(&rows[0]), id(&rows[1]));
    // A cycle makes a task of an item, starts it, and ends it, in this order
    for (item, cycles) in [(&running, 2), (&done, 3)] {
        for _ in 0..cycles {
            notes.cycle(item);
        }
    }

    let ids = notes.ids_by_types(&["task"], Some("end"));

    assert!(ids.contains(&running));
    assert!(!ids.contains(&done));
}

// 除く条件のプロパティは、list型でもよい。list型の値は、別のテーブルに保存されている。
#[test]
fn items_listed_by_type_can_leave_out_those_with_a_list_property() {
    let notes = Core::new();
    notes.define_key("left_out_list");
    let page = notes.new_doc_page("items-task-list-without-list");
    let rows = notes.fill(&page, &["plain task", "task with a list"]);
    let (plain, listed) = (id(&rows[0]), id(&rows[1]));
    for item in [&plain, &listed] {
        notes.cycle(item);
    }
    notes
        .set_item_property(
            &listed,
            "left_out_list",
            json!({ "valueType": "list", "value": "x, y" }),
        )
        .unwrap();

    let ids = notes.ids_by_types(&["task"], Some("left_out_list"));

    assert!(ids.contains(&plain));
    assert!(!ids.contains(&listed));
}
