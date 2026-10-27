use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::Harness;

#[test]
fn adding_a_property_definition_sends_it_and_shows_the_server_list() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::AddPropertyDefinition {
        key: " status ".into(),
        value_type: "text".into(),
    });
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::AddPropertyDefinition { key, value_type } if key == "status" && value_type == "text")));
    let defs = &h.model.config.property_definitions;
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].key, "status");
    assert!(!defs[0].system);
    assert!(h.model.config.definition_error.is_none());
}

#[test]
fn a_refused_definition_shows_the_reason_and_an_empty_key_is_not_sent() {
    let mut h = Harness::with_items(vec![]);
    h.api
        .fail_next("AddPropertyDefinition", "Property key 'zoom' is a system key");
    h.send(Event::AddPropertyDefinition {
        key: "zoom".into(),
        value_type: "number".into(),
    });
    assert_eq!(
        h.model.config.definition_error.as_deref(),
        Some("Property key 'zoom' is a system key")
    );

    h.send(Event::AddPropertyDefinition {
        key: "  ".into(),
        value_type: "text".into(),
    });
    assert_eq!(h.api.calls_named("AddPropertyDefinition").len(), 1);
    assert_eq!(
        h.model.config.definition_error.as_deref(),
        Some("Property key is required")
    );
}

#[test]
fn deleting_a_property_definition_sends_it_and_shows_the_server_list() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::AddPropertyDefinition {
        key: "status".into(),
        value_type: "text".into(),
    });
    assert_eq!(h.model.config.property_definitions.len(), 1);

    h.send(Event::DeletePropertyDefinition { key: "status".into() });
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::DeletePropertyDefinition { key } if key == "status")));
    assert!(h.model.config.property_definitions.is_empty());
    assert!(h.model.config.definition_error.is_none());

    h.api.fail_next("DeletePropertyDefinition", "Property key 'tags' is a system key");
    h.send(Event::DeletePropertyDefinition { key: "tags".into() });
    assert_eq!(
        h.model.config.definition_error.as_deref(),
        Some("Property key 'tags' is a system key")
    );
}

#[test]
fn setting_the_home_link_sends_it_and_keeps_the_servers_answer() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::SetHomeLink("a/Foo".into()));
    assert!(h
        .api
        .calls
        .iter()
        .any(|c| matches!(c, ApiOp::UpdateConfig { home_link: Some(l), .. } if l == "a/Foo")));
    assert_eq!(h.model.config.home_link, "a/Foo");
    assert_eq!(h.view().config.home_link, "a/Foo");
}

#[test]
fn home_opens_today_without_a_link() {
    let mut h = Harness::new();
    h.send(Event::NavigateHome);
    assert_eq!(h.navigations.last().unwrap().url, "/today");
}

#[test]
fn home_opens_the_page_of_its_link() {
    let mut h = Harness::new();
    h.model.config.home_link = "sosuisen.net/article/Foo".into();
    h.send(Event::NavigateHome);
    assert_eq!(
        h.navigations.last().unwrap().url,
        "/pages/sosuisen.net/article/Foo"
    );
}

#[test]
fn home_opens_a_folder_as_it_was_last_seen() {
    let mut h = Harness::new();
    h.model.sidebar.atlas_dirs.insert("a/".into());
    h.model.config.home_link = "a/index".into();
    h.send(Event::NavigateHome);
    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/index_atlas");
}

#[test]
fn setting_the_tags_dir_sends_it_and_keeps_the_servers_normalized_form() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::SetTagsDir("tags/".into()));
    assert!(h
        .api
        .calls
        .iter()
        .any(|c| matches!(c, ApiOp::UpdateConfig { tags_dir: Some(t), .. } if t == "tags/")));
    assert_eq!(h.model.config.tags_dir, "tags/");
}
