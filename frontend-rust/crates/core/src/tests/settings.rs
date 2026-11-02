use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::Harness;

// プロパティ定義を追加すると、前後の空白を除いたキーがサーバーに送られ、サーバーの一覧が表示される。
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

// 拒否された定義はその理由を示し、空のキーは送られずに必須である旨が示される。
#[test]
fn a_refused_definition_shows_the_reason_and_an_empty_key_is_not_sent() {
    let mut h = Harness::with_items(vec![]);
    h.api.fail_next(
        "AddPropertyDefinition",
        "Property key 'zoom' is a system key",
    );
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

// プロパティ定義を削除すると、それがサーバーに送られ、サーバーの一覧が表示される。拒否された削除はその理由を示す。
#[test]
fn deleting_a_property_definition_sends_it_and_shows_the_server_list() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::AddPropertyDefinition {
        key: "status".into(),
        value_type: "text".into(),
    });
    assert_eq!(h.model.config.property_definitions.len(), 1);

    h.send(Event::DeletePropertyDefinition {
        key: "status".into(),
    });
    assert!(h
        .api
        .calls
        .iter()
        .any(|c| matches!(c, ApiOp::DeletePropertyDefinition { key } if key == "status")));
    assert!(h.model.config.property_definitions.is_empty());
    assert!(h.model.config.definition_error.is_none());

    h.api.fail_next(
        "DeletePropertyDefinition",
        "Property key 'tags' is a system key",
    );
    h.send(Event::DeletePropertyDefinition { key: "tags".into() });
    assert_eq!(
        h.model.config.definition_error.as_deref(),
        Some("Property key 'tags' is a system key")
    );
}

// ホームリンクを設定すると、それがサーバーに送られ、サーバーの答えが保持される。
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

// タグフォルダを設定すると、それがサーバーに送られ、サーバーが正規化した形が保持される。
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
