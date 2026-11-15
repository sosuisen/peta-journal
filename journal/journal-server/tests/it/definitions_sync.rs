//! The definitions of the user keys go to the other devices in `settings/properties.json`
//! (ADR 046).

use journal_core::domain::property_definitions::Definition;
use journal_core::infrastructure::export::definitions_file::{self, DEFINITIONS_FILE};
use journal_core::journal::{AddPropertyDefinitionInput, ChangedBy};
use journal_server::config::Settings;
use journal_server::state::{AppState, BuildOptions};

use super::atlas_sync::{two_devices, Device};

impl Device {
    fn define(&self, key: &str, value_type: &str, page: bool) {
        self.state
            .journal
            .add_property_definition(AddPropertyDefinitionInput {
                key: Some(key.to_string()),
                value_type: Some(value_type.to_string()),
                page: Some(page),
            })
            .unwrap();
    }

    fn undefine(&self, key: &str) {
        self.state
            .journal
            .delete_property_definition(key, &ChangedBy::Mcp)
            .unwrap();
    }

    /// What this device wrote in `settings/properties.json` of a key
    fn written(&self, key: &str) -> Option<Definition> {
        let text = std::fs::read_to_string(self.root.join(DEFINITIONS_FILE)).ok()?;
        definitions_file::read(&text).unwrap().remove(key)
    }
}

// ユーザーが定義を足すと、端末は、定義を日時とともにsettings/properties.jsonに書く。
#[test]
fn a_definition_the_user_adds_is_written_with_its_date() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let before = journal_core::infrastructure::persistence::db::now_millis();

    a.define("score", "number", false);

    let Some(Definition::Defined {
        value_type,
        page,
        modified,
    }) = a.written("score")
    else {
        panic!("score is not written");
    };
    assert_eq!((value_type.as_str(), page), ("number", false));
    assert!(modified >= before, "{modified} < {before}");
}

// ユーザーが定義を消すと、端末は、消した日時をsettings/properties.jsonに書く。
#[test]
fn a_definition_the_user_deletes_is_written_as_deleted_with_its_date() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    a.define("score", "number", false);
    let before = journal_core::infrastructure::persistence::db::now_millis();

    a.undefine("score");

    let Some(Definition::Deleted { deleted }) = a.written("score") else {
        panic!("score is not written as deleted: {:?}", a.written("score"));
    };
    assert!(deleted >= before, "{deleted} < {before}");
}

// 消した定義をユーザーが足し直すと、端末は、消した定義の表からそのキーの行を消し、定義としてsettings/properties.jsonに書く。
#[test]
fn a_deleted_definition_the_user_adds_again_is_no_longer_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    a.define("score", "number", false);
    a.undefine("score");

    a.define("score", "text", true);

    let deleted: i64 = a
        .state
        .db
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM deleted_property_definitions WHERE key = 'score'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(deleted, 0);
    assert!(matches!(
        a.written("score"),
        Some(Definition::Defined { value_type, page: true, .. }) if value_type == "text"
    ));
}

// Markdownフォルダーを選んでいない端末でも、ユーザーは定義を足せる。書くファイルがないだけである。
#[test]
fn a_device_without_a_markdown_folder_still_takes_a_definition() {
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::build(
        Settings::under(dir.path(), 0),
        BuildOptions { interactive: false },
    )
    .unwrap();

    state
        .journal
        .add_property_definition(AddPropertyDefinitionInput {
            key: Some("score".to_string()),
            value_type: Some("number".to_string()),
            page: Some(false),
        })
        .unwrap();

    assert!(state.properties.find_definition("score").unwrap().is_some());
}

impl Device {
    /// The definition of the key in the DB of this device
    fn definition(&self, key: &str) -> Option<(String, bool)> {
        let d = self.state.properties.find_definition(key).unwrap()?;
        Some((d.value_type, d.page))
    }
}

// 端末は、ほかの端末が書いたsettings/properties.jsonを受け取ると、その端末のDBにない定義を、DBに入れる。
#[test]
fn a_definition_from_another_device_comes_into_the_db() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    b.define("score", "number", false);

    a.receive(&b, DEFINITIONS_FILE);

    assert_eq!(a.definition("score"), Some(("number".to_string(), false)));
}

// 同じキーは、新しい方の定義になる。端末のDBの定義が古ければ、届いた定義に変わる。値は、新しい型で読む。
#[test]
fn a_newer_definition_from_another_device_replaces_the_older_one() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    a.define("score", "text", true);
    std::thread::sleep(std::time::Duration::from_millis(5));
    b.define("score", "number", false);

    a.receive(&b, DEFINITIONS_FILE);

    assert_eq!(a.definition("score"), Some(("number".to_string(), false)));
}

// 端末のDBの定義のほうが新しければ、端末は定義を変えず、settings/properties.jsonを書き直す。ほかの端末に、新しい定義を届けるためである。
#[test]
fn an_older_definition_from_another_device_leaves_the_newer_one_and_the_file_is_written_again() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    b.define("score", "text", true);
    std::thread::sleep(std::time::Duration::from_millis(5));
    a.define("score", "number", false);
    let a_wrote = a.written("score");

    a.receive(&b, DEFINITIONS_FILE);

    assert_eq!(a.definition("score"), Some(("number".to_string(), false)));
    assert_eq!(a.written("score"), a_wrote);
}
