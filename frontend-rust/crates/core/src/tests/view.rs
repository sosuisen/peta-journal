//! What the rows of the view model say about their items, and the zoom of an item.

use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::{item, Harness};
use crate::view::{Row, TaskState};

fn rows(h: &Harness) -> Vec<Row> {
    h.view().outline.rows
}

fn with_props(
    mut it: crate::model::Item,
    types: &[&str],
    props: &[(&str, &str)],
) -> crate::model::Item {
    it.types = types.iter().map(|t| t.to_string()).collect();
    for (k, v) in props {
        it.properties
            .insert(k.to_string(), serde_json::Value::String(v.to_string()));
    }
    it
}

// タスクは begin と end から自分の状態を報告する。
#[test]
fn a_task_reports_its_state_from_begin_and_end() {
    let h = Harness::with_items(vec![
        item("plain", 1, "text"),
        with_props(item("todo", 1, "text"), &["task"], &[]),
        with_props(
            item("running", 1, "text"),
            &["task"],
            &[("begin", "2026-01-01T00:00:00Z")],
        ),
        with_props(
            item("done", 1, "text"),
            &["task"],
            &[
                ("begin", "2026-01-01T00:00:00Z"),
                ("end", "2026-01-01T01:00:00Z"),
            ],
        ),
    ]);
    let states: Vec<Option<TaskState>> = rows(&h).iter().map(|r| r.task).collect();
    assert_eq!(
        states,
        [
            None,
            Some(TaskState::NotStarted),
            Some(TaskState::Running),
            Some(TaskState::Done)
        ]
    );
}

// 一行だけの見出しは見出し行であるが、続きの行がある見出しはそうではない。
#[test]
fn a_one_line_heading_is_a_heading_line_but_a_heading_with_more_lines_is_not() {
    let h = Harness::with_items(vec![
        item("one", 1, "## Title"),
        item("many", 1, "## Title\nmore"),
        item("text", 1, "plain"),
    ]);
    let r = rows(&h);
    assert_eq!((r[0].heading_level, r[0].heading_line), (2, true));
    assert_eq!((r[1].heading_level, r[1].heading_line), (2, false));
    assert_eq!((r[2].heading_level, r[2].heading_line), (0, false));
}

// ズームの変更で倍率が表示に反映されて保存され、リセットで 1.0 に戻る。
#[test]
fn set_zoom_shows_and_saves_the_factor() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::ZoomStep {
        item_id: "a".into(),
        up: true,
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().zoom(), 1.1);
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::UpsertItemProperty { key, value, .. } if key == "zoom" && value == "1.1")));
    h.send(Event::ResetZoom {
        item_id: "a".into(),
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().zoom(), 1.0);
}

// 自前の zoom を持つ項目だけが zoom を持つとされる。子は zoom を継承するが自前のものではない。
#[test]
fn only_an_item_with_a_zoom_of_its_own_has_one() {
    let h = Harness::with_items(vec![
        with_props(item("zoomed", 1, "a"), &[], &[("zoom", "1.5")]),
        item("child", 2, "b"),
    ]);
    let r = rows(&h);
    assert!(r[0].has_own_zoom);
    assert!(
        !r[1].has_own_zoom,
        "the child inherits the zoom but has none of its own"
    );
    assert_eq!(r[1].effective_zoom, 1.5);
}
