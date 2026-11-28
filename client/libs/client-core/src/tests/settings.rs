use crate::app::Event;
use crate::effects::{ApiOp, ApiOutput, FolderChoice};
use crate::harness::Harness;
use crate::model::{shown_name, PropertyDefinition, ServerConfig, WeekNumbering};

// 本体と型でプロパティ定義を追加すると、前後の空白を除いた本体に型の接尾辞を付けた名前がサーバーに送られ、サーバーの一覧が表示される。
#[test]
fn adding_a_property_definition_sends_the_body_with_the_suffix_of_its_type() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::AddPropertyDefinition {
        body: " rating ".into(),
        value_type: "number".into(),
    });
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::AddPropertyDefinition { key, value_type } if key == "rating_n" && value_type == "number")));
    let defs = &h.model.config.property_definitions;
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].key, "rating_n");
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
        body: "zoom".into(),
        value_type: "number".into(),
    });
    assert_eq!(
        h.model.config.definition_error.as_deref(),
        Some("Property key 'zoom' is a system key")
    );

    h.send(Event::AddPropertyDefinition {
        body: "  ".into(),
        value_type: "text".into(),
    });
    assert_eq!(h.api.calls_named("AddPropertyDefinition").len(), 1);
    assert_eq!(
        h.model.config.definition_error.as_deref(),
        Some("Property key is required")
    );
}

// サーバーがキーを受け入れると、受け入れた数が増え、シェルはフォームを空にする。拒否されたキーと、送られなかったキーでは増えない。
#[test]
fn an_accepted_key_counts_and_a_refused_one_does_not() {
    let mut h = Harness::with_items(vec![]);
    let added = |h: &Harness| h.view().config.definitions_added;
    h.send(Event::AddPropertyDefinition {
        body: "rating".into(),
        value_type: "number".into(),
    });
    assert_eq!(added(&h), 1);
    h.api.fail_next("AddPropertyDefinition", "defined already");
    h.send(Event::AddPropertyDefinition {
        body: "rating".into(),
        value_type: "number".into(),
    });
    h.send(Event::AddPropertyDefinition {
        body: " ".into(),
        value_type: "text".into(),
    });
    assert_eq!(added(&h), 1);
}

// 予約の接尾辞で終わる本体は、サーバーに送られず、その理由が示される。
#[test]
fn a_body_that_ends_with_a_reserved_suffix_is_not_sent() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::AddPropertyDefinition {
        body: "rating_n".into(),
        value_type: "number".into(),
    });
    assert!(h.api.calls_named("AddPropertyDefinition").is_empty());
    assert_eq!(
        h.model.config.definition_error.as_deref(),
        Some("The name cannot end with _n, _d, _dt, _cb, _bounds or _list")
    );
}

// ユーザーが選べない型の定義は、サーバーに送られず、その理由が示される。
#[test]
fn a_definition_of_a_type_the_user_cannot_choose_is_not_sent() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::AddPropertyDefinition {
        body: "topics".into(),
        value_type: "tags".into(),
    });
    assert!(h.api.calls_named("AddPropertyDefinition").is_empty());
    assert_eq!(
        h.model.config.definition_error.as_deref(),
        Some("A key cannot be of type tags")
    );
}

// 定義は、ユーザーのキーを名前の本体で示し、システムのキーを名前のまま示す。
#[test]
fn a_definition_shows_the_body_of_a_user_key_and_the_whole_name_of_a_system_key() {
    let user = PropertyDefinition {
        key: "rating_n".into(),
        value_type: "number".into(),
        system: false,
        page: true,
    };
    let system = PropertyDefinition {
        key: "atlas_bounds".into(),
        value_type: "bounds".into(),
        system: true,
        page: false,
    };
    let definitions = [user, system];
    assert_eq!(shown_name("rating_n", &definitions), "rating");
    assert_eq!(shown_name("atlas_bounds", &definitions), "atlas_bounds");
}

// 本体が同じユーザーのキーが2つ以上あれば、それぞれの名前に型が添えられる。本体がほかと重ならないキーは、本体だけで示される。
#[test]
fn keys_that_share_a_body_are_shown_with_their_types() {
    let definition = |key: &str, value_type: &str| PropertyDefinition {
        key: key.into(),
        value_type: value_type.into(),
        system: false,
        page: true,
    };
    let definitions = [
        definition("rating_n", "number"),
        definition("rating_d", "date"),
        definition("status", "text"),
    ];
    assert_eq!(shown_name("rating_n", &definitions), "rating (number)");
    assert_eq!(shown_name("rating_d", &definitions), "rating (date)");
    assert_eq!(shown_name("status", &definitions), "status");
}

// プロパティ定義を削除すると、それがサーバーに送られ、サーバーの一覧が表示される。拒否された削除はその理由を示す。
#[test]
fn deleting_a_property_definition_sends_it_and_shows_the_server_list() {
    let mut h = Harness::with_items(vec![]);
    h.send(Event::AddPropertyDefinition {
        body: "status".into(),
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
        client_id: crate::rest::ClientId::new(),
        opens_new_day: true,
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
        client_id: crate::rest::ClientId::new(),
        opens_new_day: true,
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
