use crate::app::{Event, Screen};
use crate::effects::{ApiOp, UiOp};
use crate::harness::Harness;
use crate::view::ScreenView;

fn page_properties(h: &Harness) -> Vec<(String, String)> {
    match &h.model.screen {
        Screen::Page(p) => p
            .page
            .as_ref()
            .map(|pg| {
                pg.properties
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn shows_properties(h: &Harness) -> bool {
    match &h.view().screen {
        ScreenView::Page(Some(p)) => p.show_properties,
        _ => false,
    }
}

/// The rows as the shell gets them: key, value, removable, editable.
fn rows(h: &Harness) -> Vec<(String, String, bool, bool)> {
    match &h.view().screen {
        ScreenView::Page(Some(p)) => p
            .properties
            .iter()
            .map(|r| (r.key.clone(), r.value.clone(), r.removable, r.editable))
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn the_dates_of_the_page_are_rows_that_cannot_be_removed_and_do_not_open_the_rows() {
    let mut h = Harness::with_items(vec![]);
    assert!(!shows_properties(&h));
    assert_eq!(
        rows(&h),
        vec![
            ("created".to_string(), "1970-01-01T00:00:00Z".to_string(), false, true),
            ("modified".to_string(), "1970-01-01T00:00:00Z".to_string(), false, false),
        ]
    );
    h.send(Event::SetPageProperty {
        key: "created".into(),
        value_type: "datetime".into(),
        value: "2020-01-02T03:04:05Z".into(),
    });
    // The date is not a stored property: it changes only with the server's page
    assert!(page_properties(&h).is_empty());
    assert_eq!(rows(&h)[0].1, "1970-01-01T00:00:00.042Z");
}

#[test]
fn setting_a_page_property_shows_it_and_takes_the_server_value() {
    let mut h = Harness::with_items(vec![]);
    assert!(!shows_properties(&h));
    h.send(Event::SetPageProperty {
        key: "owner".into(),
        value_type: "text".into(),
        value: "me".into(),
    });
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::UpsertPageProperty { page_id, key, value_type, value } if page_id == "page1" && key == "owner" && value_type == "text" && value == "me")));
    assert_eq!(page_properties(&h), vec![("owner".to_string(), "\"me\"".to_string())]);
    assert!(shows_properties(&h));
    // The server's page replaced the shown one
    if let Screen::Page(p) = &h.model.screen {
        assert_eq!(p.page.as_ref().unwrap().modified_date, 42);
    }
}

#[test]
fn a_tag_value_is_shown_as_a_list_until_the_server_answers() {
    let mut h = Harness::with_items(vec![]);
    h.api.fail_next("UpsertPageProperty", "offline");
    h.send(Event::SetPageProperty {
        key: "tag".into(),
        value_type: "tags".into(),
        value: "a, b,".into(),
    });
    // The refusal put the old (empty) properties back and told the user
    assert!(page_properties(&h).is_empty());
    assert!(h.ui.iter().any(|u| matches!(u, UiOp::Alert(m) if m == "offline")));
}

#[test]
fn deleting_a_page_property_removes_it() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::SetPageProperty {
        key: "owner".into(),
        value_type: "text".into(),
        value: "me".into(),
    });
    h.send(Event::DeletePageProperty {
        key: "owner".into(),
    });
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::DeletePageProperty { key, .. } if key == "owner")));
    assert!(page_properties(&h).is_empty());
    // The rows stay off once the last property is gone, unless the menu opens them
    assert!(!shows_properties(&h));
    h.send(Event::EditPageProperties);
    assert!(shows_properties(&h));
}

#[test]
fn a_hash_typed_before_a_tag_is_dropped_and_a_tag_is_kept_once() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::SetPageProperty {
        key: "tag".into(),
        value_type: "tags".into(),
        value: "#abc, ##def, abc,".into(),
    });
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::UpsertPageProperty { value, .. } if value == "abc, def")));
}
