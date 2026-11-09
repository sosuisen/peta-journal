use crate::app::Event;
use crate::effects::{ApiOp, ApiOutput, FolderChoice};
use crate::harness::Harness;
use crate::model::{ServerConfig, WeekNumbering};

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

// 起動でサーバーの設定が読み込まれ、作業フォルダ・週番号の方式・論理アウトデント・タグフォルダ・ホームリンクが取り込まれる。
#[test]
fn starting_takes_the_settings_of_the_server() {
    let mut h = Harness::new();
    h.api.answer_next(
        "GetConfig",
        Ok(ApiOutput::Config(ServerConfig {
            markdown_dir: "/md".into(),
            week_numbering: "iso".into(),
            logical_outdent: true,
            tags_dir: "tags/".into(),
            home_link: "Home".into(),
        })),
    );
    h.send(Event::Start {
        client_id: String::new(),
    });

    let c = &h.model.config;
    assert_eq!(c.markdown_dir, "/md");
    assert_eq!(c.week_numbering, WeekNumbering::Iso);
    assert!(c.logical_outdent);
    assert_eq!(c.tags_dir, "tags/");
    assert_eq!(c.home_link, "Home");
}

// 設定やプロパティ定義を読み込めなければ、既定の設定のままである。
#[test]
fn settings_that_cannot_be_loaded_stay_at_their_defaults() {
    let mut h = Harness::new();
    h.api.fail_next("GetConfig", "offline");
    h.api.fail_next("GetPropertyDefinitions", "offline");
    h.send(Event::Start {
        client_id: String::new(),
    });

    assert_eq!(h.model.config.week_numbering, WeekNumbering::Us);
    assert!(h.model.config.property_definitions.is_empty());
}

// 週番号の方式と論理アウトデントは、すぐに画面に反映され、サーバーに保存される。
#[test]
fn the_week_numbering_and_the_logical_outdent_are_saved() {
    let mut h = Harness::new();
    h.send(Event::SetWeekNumbering(WeekNumbering::Iso));
    h.send(Event::SetLogicalOutdent(true));

    assert_eq!(h.model.config.week_numbering, WeekNumbering::Iso);
    assert!(h.model.config.logical_outdent);
    assert!(h.api.calls.iter().any(|c| matches!(
        c,
        ApiOp::UpdateConfig { week_numbering: Some(w), .. } if w == "iso"
    )));
    assert!(h.api.calls.iter().any(|c| matches!(
        c,
        ApiOp::UpdateConfig {
            logical_outdent: Some(true),
            ..
        }
    )));
}

// 保存に失敗しても、画面は入力した値のままである。
#[test]
fn a_failed_save_keeps_the_typed_value() {
    let mut h = Harness::new();
    h.api.fail_next("UpdateConfig", "offline");
    h.send(Event::SetTagsDir("tags/".into()));
    assert_eq!(h.model.config.tags_dir, "tags/");
}

// フォルダの選択で選んだフォルダが作業フォルダになり、サーバーに保存される。取り消しや失敗では何も変わらない。
#[test]
fn browsing_takes_the_chosen_folder_as_the_markdown_dir() {
    let mut h = Harness::new();
    h.api.answer_next(
        "BrowseFolder",
        Ok(ApiOutput::Folder(FolderChoice {
            selected: true,
            path: "/chosen".into(),
        })),
    );
    h.send(Event::BrowseFolder);
    assert_eq!(h.model.config.markdown_dir, "/chosen");
    assert!(h.api.calls.iter().any(|c| matches!(
        c,
        ApiOp::UpdateConfig { markdown_dir: Some(d), .. } if d == "/chosen"
    )));

    h.send(Event::BrowseFolder);
    h.api.fail_next("BrowseFolder", "no dialog");
    h.send(Event::BrowseFolder);
    assert_eq!(h.api.calls_named("UpdateConfig").len(), 1);
}
