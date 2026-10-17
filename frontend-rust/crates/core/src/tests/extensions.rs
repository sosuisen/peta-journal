//! Extension functions: the `CreateItem` messages of auto-template.

use crate::app::Event;
use crate::effects::UiOp;
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
    });
}

#[test]
fn an_empty_after_item_id_means_first_as_in_the_angular_service() {
    let mut h = Harness::with_items(vec![]);
    create_item(&mut h, "", 1, "# AM");
    assert_eq!(
        create_calls(&h.api),
        vec![("page1".to_string(), "first".to_string(), 1, "# AM".to_string(), None)]
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
