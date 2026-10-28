//! Extension functions: the messages of auto-template.

use crate::app::Event;
use crate::effects::{ApiOp, UiOp};
use crate::harness::{create_calls, item, Harness};

fn create_item(h: &mut Harness, after_item_id: &str, level: i32, body: &str) {
    h.send(Event::ExtensionCreateItem {
        source: 1,
        request_id: Some("req-1".into()),
        page_id: "page1".into(),
        position: None,
        after_item_id: Some(after_item_id.into()),
        level,
        body: body.into(),
        transclude: None,
    });
}

#[test]
fn an_empty_after_item_id_means_first_as_in_the_angular_service() {
    let mut h = Harness::with_items(vec![]);
    create_item(&mut h, "", 1, "# AM");
    assert_eq!(
        create_calls(&h.api),
        vec![(
            "page1".to_string(),
            "first".to_string(),
            1,
            "# AM".to_string(),
            None
        )]
    );
    assert_eq!(h.bodies(), vec!["new1=# AM"]);
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::ExtensionReply { source: 1, message } if message["type"] == "CreateItemResult")));
}

#[test]
fn an_empty_position_falls_back_to_after_item_id() {
    let mut h = Harness::with_items(vec![item("a", 1, "# AM")]);
    h.send(Event::ExtensionCreateItem {
        source: 1,
        request_id: None,
        page_id: "page1".into(),
        position: Some("".into()),
        after_item_id: Some("a".into()),
        level: 2,
        body: "- [ ] Check e-mails".into(),
        transclude: None,
    });
    assert_eq!(create_calls(&h.api)[0].1, "after:a");
    assert_eq!(create_calls(&h.api)[0].2, 2);
}

#[test]
fn creating_a_journal_tells_the_extensions() {
    use crate::effects::ApiOutput;
    use crate::model::{Page, PageWithItems};
    let mut h = Harness::new();
    h.api.answer_next(
        "GetOrCreateJournal",
        Ok(ApiOutput::Page(PageWithItems {
            page: Page {
                id: "p1".into(),
                name: "2031-01-05".into(),
                page_type: "journal".into(),
                ..Default::default()
            },
            items: vec![item("e", 1, "")],
            created: true,
            ..Default::default()
        })),
    );
    h.send(Event::Route {
        route: crate::app::Route::Page {
            name: "2031-01-05".into(),
            fragment: None,
        },
        today: "2031-01-01".into(),
    });
    assert!(
        h.ui.iter().any(|op| matches!(op, UiOp::ExtensionPageCreate { file_name, .. } if file_name == "2031-01-05")),
        "ui ops: {:?}",
        h.ui
    );
}

fn set_tags(h: &mut Harness, value: &str) {
    h.send(Event::SetPageProperty {
        key: "tags".into(),
        value_type: "tags".into(),
        value: value.into(),
    });
}

fn added_tags(h: &Harness) -> Vec<Vec<String>> {
    h.ui.iter()
        .filter_map(|op| match op {
            UiOp::ExtensionPageTagAdd { page_id, tags, .. } if page_id == "page1" => {
                Some(tags.clone())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn adding_tags_to_a_page_tells_the_extensions_of_the_new_ones_only() {
    let mut h = Harness::with_items(vec![]);
    set_tags(&mut h, "a");
    set_tags(&mut h, "a, #b, c");
    assert_eq!(added_tags(&h), vec![vec!["a"], vec!["b", "c"]]);
}

#[test]
fn removing_a_tag_or_setting_another_property_tells_the_extensions_nothing() {
    let mut h = Harness::with_items(vec![]);
    set_tags(&mut h, "a, b");
    set_tags(&mut h, "a");
    h.send(Event::SetPageProperty {
        key: "owner".into(),
        value_type: "text".into(),
        value: "me".into(),
    });
    assert_eq!(added_tags(&h), vec![vec!["a", "b"]]);
}

#[test]
fn a_refused_tag_is_not_told_to_the_extensions() {
    let mut h = Harness::with_items(vec![]);
    h.api.fail_next("UpsertPageProperty", "refused");
    set_tags(&mut h, "a");
    assert!(added_tags(&h).is_empty());
}

#[test]
fn an_extension_asks_the_user_and_gets_the_answer() {
    for answer in [true, false] {
        let mut h = Harness::with_items(vec![]);
        h.confirm = answer;
        h.send(Event::ExtensionConfirm {
            source: 3,
            request_id: Some("req-9".into()),
            message: "Insert?".into(),
        });
        assert!(h
            .ui
            .iter()
            .any(|op| matches!(op, UiOp::Confirm(m) if m == "Insert?")));
        assert!(h.ui.iter().any(|op| matches!(op, UiOp::ExtensionReply { source: 3, message }
            if message["type"] == "ConfirmResult" && message["ok"] == answer && message["requestId"] == "req-9")));
    }
}

#[test]
fn an_extension_item_can_transclude_another_item() {
    let mut h = Harness::with_items(vec![item("a", 1, "# AM")]);
    h.send(Event::ExtensionCreateItem {
        source: 1,
        request_id: Some("req-1".into()),
        page_id: "page1".into(),
        position: Some("last".into()),
        after_item_id: None,
        level: 1,
        body: String::new(),
        transclude: Some("2026-10-23-12-53-28-GNSEMVA".into()),
    });
    assert!(h.api.calls.iter().any(
        |c| matches!(c, ApiOp::SetTransclusion { item_id, source_item_id }
        if item_id == "new1" && source_item_id == "2026-10-23-12-53-28-GNSEMVA")
    ));
    assert!(h.ui.iter().any(
        |op| matches!(op, UiOp::ExtensionReply { source: 1, message }
        if message["type"] == "CreateItemResult" && message["itemId"] == "new1")
    ));
}

#[test]
fn an_extension_item_without_transclude_sets_none() {
    let mut h = Harness::with_items(vec![]);
    create_item(&mut h, "", 1, "# AM");
    assert!(!h
        .api
        .calls
        .iter()
        .any(|c| matches!(c, ApiOp::SetTransclusion { .. })));
}

fn create_last(h: &mut Harness, body: &str, transclude: Option<&str>) {
    h.send(Event::ExtensionCreateItem {
        source: 1,
        request_id: Some("req-1".into()),
        page_id: "page1".into(),
        position: Some("last".into()),
        after_item_id: None,
        level: 2,
        body: body.into(),
        transclude: transclude.map(str::to_string),
    });
}

#[test]
fn an_item_for_the_second_line_overwrites_a_lone_empty_first_line() {
    let mut h = Harness::with_items(vec![item("e", 1, "")]);
    create_last(&mut h, "## App", None);
    assert!(create_calls(&h.api).is_empty());
    assert!(h.api.calls.iter().any(
        |c| matches!(c, ApiOp::UpdateItem { item_id, body: Some(b), level: Some(1), .. }
        if item_id == "e" && b == "## App")
    ));
    assert_eq!(h.bodies(), vec!["e=## App"]);
    assert!(h.ui.iter().any(
        |op| matches!(op, UiOp::ExtensionReply { source: 1, message }
        if message["type"] == "CreateItemResult" && message["itemId"] == "e")
    ));
}

#[test]
fn the_overwritten_empty_line_can_become_a_transclusion() {
    let mut h = Harness::with_items(vec![item("e", 1, "")]);
    create_last(&mut h, "", Some("src-item-id"));
    assert!(h.api.calls.iter().any(
        |c| matches!(c, ApiOp::SetTransclusion { item_id, source_item_id }
        if item_id == "e" && source_item_id == "src-item-id")
    ));
}

#[test]
fn a_first_line_with_text_or_a_second_line_is_not_overwritten() {
    for items in [
        vec![item("a", 1, "text")],
        vec![item("e", 1, ""), item("f", 1, "")],
    ] {
        let mut h = Harness::with_items(items);
        create_last(&mut h, "## App", None);
        assert_eq!(create_calls(&h.api).len(), 1);
    }
}

#[test]
fn an_item_for_the_first_line_does_not_overwrite_the_empty_one() {
    let mut h = Harness::with_items(vec![item("e", 1, "")]);
    create_item(&mut h, "", 1, "# AM");
    assert_eq!(h.bodies(), vec!["new1=# AM", "e="]);
}

#[test]
fn an_item_after_the_lone_empty_line_does_not_overwrite_it() {
    let mut h = Harness::with_items(vec![item("e", 1, "")]);
    create_item(&mut h, "e", 1, "## App");
    assert_eq!(h.bodies(), vec!["e=", "new1=## App"]);
}

#[test]
fn a_failed_transclusion_is_told_and_the_next_item_goes_after_the_empty_line() {
    let mut h = Harness::with_items(vec![item("e", 1, "")]);
    h.api
        .fail_next("SetTransclusion", "Item not found: missing-id");
    create_last(&mut h, "", Some("missing-id"));
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::Alert(m) if m.contains("missing-id"))));
    create_item(&mut h, "e", 1, "## App");
    assert_eq!(h.bodies(), vec!["e=", "new1=## App"]);
}
