//! The definitions of the user keys go to the other devices in `settings/properties.json`
//! (ADR 046).

use journal_core::domain::property_definitions::Definition;
use journal_core::infrastructure::export::definitions_file::{self, DEFINITIONS_FILE};
use journal_core::infrastructure::export::import_service::Outcome;
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
    fn is_provisional(&self, key: &str) -> bool {
        self.state
            .db
            .conn()
            .unwrap()
            .query_row(
                "SELECT provisional FROM property_definitions WHERE key = ?",
                [key],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
            == 1
    }

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

// 合わせた結果が、届いたsettings/properties.jsonと同じなら、端末はファイルを書き直さない。だから、2台がファイルを送り合い続けることはない。
#[test]
fn a_file_that_holds_what_the_device_has_is_not_written_again() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    b.define("score", "number", false);
    a.deliver(&b, DEFINITIONS_FILE);
    let file = a.root.join(DEFINITIONS_FILE);
    let long_ago = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(long_ago)
        .unwrap();

    a.sync(DEFINITIONS_FILE);

    assert_eq!(
        std::fs::metadata(&file).unwrap().modified().unwrap(),
        long_ago
    );
}

// ほかの端末が消した定義が届き、その端末のDBにそのキーの値がなければ、端末はDBから定義を消す。
#[test]
fn a_definition_deleted_on_another_device_goes_when_no_value_holds_it() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    a.define("score", "number", false);
    b.receive(&a, DEFINITIONS_FILE);
    b.undefine("score");

    a.receive(&b, DEFINITIONS_FILE);

    assert_eq!(a.definition("score"), None);
    assert!(matches!(
        a.written("score"),
        Some(Definition::Deleted { .. })
    ));
}

// ほかの端末が消した定義が届いても、その端末のDBにそのキーの値があれば、端末は定義を仮の定義にして残し、値も残す。仮の定義はsettings/properties.jsonに書かない。
#[test]
fn a_definition_deleted_on_another_device_stays_provisional_while_a_value_holds_it() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score", "number", true);
    a.state
        .page_service
        .set_property(&page, "score", "number", "12")
        .unwrap();
    b.receive(&a, DEFINITIONS_FILE);
    b.undefine("score");

    assert_eq!(a.receive(&b, DEFINITIONS_FILE), Outcome::Imported);

    assert_eq!(a.definition("score"), Some(("number".to_string(), true)));
    assert!(a.is_provisional("score"));
    let (scalars, _) = a.state.page_service.page_properties(&page).unwrap();
    assert_eq!(scalars[page.as_str()]["score"], "12");
    assert!(matches!(
        a.written("score"),
        Some(Definition::Deleted { .. })
    ));
}

// 消した定義より新しい定義が届くと、端末は、その定義を本物の定義としてDBに入れ、消した定義の表からそのキーの行を消す。仮の定義も、本物の定義になる。
#[test]
fn a_definition_newer_than_its_deletion_makes_the_key_real_again() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score", "number", true);
    a.state
        .page_service
        .set_property(&page, "score", "number", "12")
        .unwrap();
    b.receive(&a, DEFINITIONS_FILE);
    b.undefine("score");
    a.receive(&b, DEFINITIONS_FILE);
    std::thread::sleep(std::time::Duration::from_millis(5));
    b.define("score", "text", true);

    a.receive(&b, DEFINITIONS_FILE);

    assert_eq!(a.definition("score"), Some(("text".to_string(), true)));
    assert!(!a.is_provisional("score"));
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
}

// settings/properties.jsonのconflicted copyが届くと、端末は、それをもう1つの版として合わせてから消す。どちらの版の定義も残る。
#[test]
fn a_conflicted_copy_of_the_definitions_is_merged_and_then_removed() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    a.define("owner", "text", true);
    b.define("score", "number", false);
    let copy = "settings/properties (PC's conflicted copy 2026-11-15).json";
    std::fs::copy(b.root.join(DEFINITIONS_FILE), a.root.join(copy)).unwrap();

    assert_eq!(a.sync(copy), Outcome::Imported);

    assert_eq!(a.definition("score"), Some(("number".to_string(), false)));
    assert!(a.written("score").is_some());
    assert!(a.written("owner").is_some());
    assert!(!a.root.join(copy).exists());
}

// 端末は、Markdownフォルダーの走査でも、settings/properties.jsonとそのconflicted copyを取り込む。
#[test]
fn a_scan_of_the_folder_takes_in_the_definitions_and_their_copies() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    a.doc("a");
    a.export();
    b.define("score", "number", false);
    a.deliver(&b, DEFINITIONS_FILE);
    let copy = "settings/properties (PC's conflicted copy 2026-11-15).json";
    std::fs::write(
        a.root.join(copy),
        r#"{ "definitions": { "owner": { "type": "text", "page": true, "modified": "2025-11-15T01:00:00Z" } } }"#,
    )
    .unwrap();

    a.state.reconcile.scan();

    assert_eq!(a.definition("score"), Some(("number".to_string(), false)));
    assert_eq!(a.definition("owner"), Some(("text".to_string(), true)));
    assert!(!a.root.join(copy).exists());
}

// 端末は、settings/properties.jsonを取り込んでも、ページのファイルを書き直さず、ページの更新日時も進めない。型が変わっても、値は元の文字列のままである。
#[test]
fn taking_in_the_definitions_leaves_the_pages_and_their_files_alone() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score", "text", true);
    a.state
        .page_service
        .set_property(&page, "score", "text", "1.50")
        .unwrap();
    a.export();
    let file_before = a.file("docs/d/a.md");
    let modified_before = a
        .state
        .pages
        .find_by_id(&page)
        .unwrap()
        .unwrap()
        .modified_date;
    std::thread::sleep(std::time::Duration::from_millis(5));
    b.define("score", "number", true);

    a.receive(&b, DEFINITIONS_FILE);
    a.export();

    assert_eq!(a.definition("score"), Some(("number".to_string(), true)));
    assert_eq!(a.file("docs/d/a.md"), file_before);
    assert_eq!(
        a.state
            .pages
            .find_by_id(&page)
            .unwrap()
            .unwrap()
            .modified_date,
        modified_before
    );
    let (scalars, _) = a.state.page_service.page_properties(&page).unwrap();
    assert_eq!(scalars[page.as_str()]["score"], "1.50");
}

// 読めないsettings/properties.jsonは取り込まない。端末のDBの定義も、ファイルも変わらない。conflicted copyも同じで、読めなければ消さずに残す。
#[test]
fn a_file_of_definitions_that_cannot_be_read_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    a.define("owner", "text", true);
    let broken = r#"{ "definitions": { "score": "#;
    let copy = "settings/properties (PC's conflicted copy 2026-11-15).json";
    std::fs::write(a.root.join(DEFINITIONS_FILE), broken).unwrap();
    std::fs::write(a.root.join(copy), broken).unwrap();

    assert_eq!(a.sync(DEFINITIONS_FILE), Outcome::Error);
    assert_eq!(a.sync(copy), Outcome::Error);

    assert_eq!(a.definition("owner"), Some(("text".to_string(), true)));
    assert_eq!(a.definition("score"), None);
    assert_eq!(
        std::fs::read_to_string(a.root.join(DEFINITIONS_FILE)).unwrap(),
        broken
    );
    assert!(a.root.join(copy).exists());
}

// 定義のないキーの値がページのファイルで届くと、端末は、名前に合う型の仮の定義を作る。仮の定義は、settings/properties.jsonに書かない。
#[test]
fn a_key_with_no_definition_from_a_page_file_gets_a_provisional_definition() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score", "number", true);
    a.define("author_list", "list", true);
    a.state
        .page_service
        .set_property(&page, "score", "number", "12")
        .unwrap();
    a.state
        .page_service
        .set_property(&page, "author_list", "list", "x, y")
        .unwrap();

    b.receive(&a, "docs/d/a.md");
    b.define("owner", "text", true);

    assert_eq!(b.definition("score"), Some(("text".to_string(), true)));
    assert_eq!(
        b.definition("author_list"),
        Some(("list".to_string(), true))
    );
    assert!(b.is_provisional("score"));
    assert!(b.is_provisional("author_list"));
    assert_eq!(b.written("score"), None);
    assert_eq!(b.written("author_list"), None);
}

// ページのファイルを取り込んで、仮の定義のキーの値が端末のDBからなくなると、端末は、その仮の定義を消す。
#[test]
fn a_provisional_definition_goes_with_the_last_value_of_its_key() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score", "number", true);
    a.state
        .page_service
        .set_property(&page, "score", "number", "12")
        .unwrap();
    b.receive(&a, "docs/d/a.md");
    std::thread::sleep(std::time::Duration::from_millis(5));
    a.state
        .page_service
        .delete_property(&page, "score")
        .unwrap();

    b.receive(&a, "docs/d/a.md");

    assert_eq!(b.definition("score"), None);
}

// 項目のメタデータ行で定義のないキーの値が届いても、端末は仮の定義を作る。値がなくなれば、仮の定義を消す。
#[test]
fn a_key_of_an_item_with_no_definition_gets_a_provisional_definition_until_its_value_goes() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    let item = a
        .state
        .item_service
        .create_item(
            &page,
            &journal_core::domain::model::ItemPosition::Last,
            1,
            "x",
            None,
        )
        .unwrap()
        .id;
    a.define("score", "number", false);
    a.state
        .item_service
        .set_property(&item, "score", "number", "12")
        .unwrap();

    b.receive(&a, "docs/d/a.md");

    assert_eq!(b.definition("score"), Some(("text".to_string(), true)));
    assert!(b.is_provisional("score"));

    std::thread::sleep(std::time::Duration::from_millis(5));
    a.state
        .item_service
        .delete_property(&item, "score")
        .unwrap();
    b.receive(&a, "docs/d/a.md");

    assert_eq!(b.definition("score"), None);
}
