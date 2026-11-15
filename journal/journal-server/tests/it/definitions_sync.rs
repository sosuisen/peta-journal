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

    a.define("score_n", "number", false);

    let Some(Definition::Defined { page, modified }) = a.written("score_n") else {
        panic!("score_n is not written");
    };
    assert!(!page);
    assert!(modified >= before, "{modified} < {before}");
}

// ユーザーが定義を消すと、端末は、消した日時をsettings/properties.jsonに書く。
#[test]
fn a_definition_the_user_deletes_is_written_as_deleted_with_its_date() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    a.define("score_n", "number", false);
    let before = journal_core::infrastructure::persistence::db::now_millis();

    a.undefine("score_n");

    let Some(Definition::Deleted { deleted }) = a.written("score_n") else {
        panic!(
            "score is not written as deleted: {:?}",
            a.written("score_n")
        );
    };
    assert!(deleted >= before, "{deleted} < {before}");
}

// 消した定義をユーザーが足し直すと、端末は、消した定義の表からそのキーの行を消し、定義としてsettings/properties.jsonに書く。
#[test]
fn a_deleted_definition_the_user_adds_again_is_no_longer_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    a.define("score_n", "number", false);
    a.undefine("score_n");

    a.define("score_n", "number", true);

    let deleted: i64 = a
        .state
        .db
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM deleted_property_definitions WHERE key = 'score_n'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(deleted, 0);
    assert!(matches!(
        a.written("score_n"),
        Some(Definition::Defined { page: true, .. })
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
            key: Some("score_n".to_string()),
            value_type: Some("number".to_string()),
            page: Some(false),
        })
        .unwrap();

    assert!(state
        .properties
        .find_definition("score_n")
        .unwrap()
        .is_some());
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
    b.define("score_n", "number", false);

    a.receive(&b, DEFINITIONS_FILE);

    assert_eq!(a.definition("score_n"), Some(("number".to_string(), false)));
}

// 同じキーは、新しい方の定義になる。端末のDBの定義が古ければ、届いた定義に変わる。
#[test]
fn a_newer_definition_from_another_device_replaces_the_older_one() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    a.define("score_n", "number", true);
    std::thread::sleep(std::time::Duration::from_millis(5));
    b.define("score_n", "number", false);

    a.receive(&b, DEFINITIONS_FILE);

    assert_eq!(a.definition("score_n"), Some(("number".to_string(), false)));
}

// 端末のDBの定義のほうが新しければ、端末は定義を変えず、settings/properties.jsonを書き直す。ほかの端末に、新しい定義を届けるためである。
#[test]
fn an_older_definition_from_another_device_leaves_the_newer_one_and_the_file_is_written_again() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    b.define("score_n", "number", true);
    std::thread::sleep(std::time::Duration::from_millis(5));
    a.define("score_n", "number", false);
    let a_wrote = a.written("score_n");

    a.receive(&b, DEFINITIONS_FILE);

    assert_eq!(a.definition("score_n"), Some(("number".to_string(), false)));
    assert_eq!(a.written("score_n"), a_wrote);
}

// 合わせた結果が、届いたsettings/properties.jsonと同じなら、端末はファイルを書き直さない。だから、2台がファイルを送り合い続けることはない。
#[test]
fn a_file_that_holds_what_the_device_has_is_not_written_again() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    b.define("score_n", "number", false);
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
    a.define("score_n", "number", false);
    b.receive(&a, DEFINITIONS_FILE);
    b.undefine("score_n");

    a.receive(&b, DEFINITIONS_FILE);

    assert_eq!(a.definition("score_n"), None);
    assert!(matches!(
        a.written("score_n"),
        Some(Definition::Deleted { .. })
    ));
}

// ほかの端末が消した定義が届いても、その端末のDBにそのキーの値があれば、端末は定義を仮の定義にして残し、値も残す。仮の定義はsettings/properties.jsonに書かない。
#[test]
fn a_definition_deleted_on_another_device_stays_provisional_while_a_value_holds_it() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score_n", "number", true);
    a.state
        .page_service
        .set_property(&page, "score_n", "number", "12")
        .unwrap();
    b.receive(&a, DEFINITIONS_FILE);
    b.undefine("score_n");

    assert_eq!(a.receive(&b, DEFINITIONS_FILE), Outcome::Imported);

    assert_eq!(a.definition("score_n"), Some(("number".to_string(), true)));
    assert!(a.is_provisional("score_n"));
    let (scalars, _) = a.state.page_service.page_properties(&page).unwrap();
    assert_eq!(scalars[page.as_str()]["score_n"], "12");
    assert!(matches!(
        a.written("score_n"),
        Some(Definition::Deleted { .. })
    ));
}

// 消した定義より新しい定義が届くと、端末は、その定義を本物の定義としてDBに入れ、消した定義の表からそのキーの行を消す。仮の定義も、本物の定義になる。
#[test]
fn a_definition_newer_than_its_deletion_makes_the_key_real_again() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score_n", "number", true);
    a.state
        .page_service
        .set_property(&page, "score_n", "number", "12")
        .unwrap();
    b.receive(&a, DEFINITIONS_FILE);
    b.undefine("score_n");
    a.receive(&b, DEFINITIONS_FILE);
    std::thread::sleep(std::time::Duration::from_millis(5));
    b.define("score_n", "number", true);

    a.receive(&b, DEFINITIONS_FILE);

    assert_eq!(a.definition("score_n"), Some(("number".to_string(), true)));
    assert!(!a.is_provisional("score_n"));
    let deleted: i64 = a
        .state
        .db
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM deleted_property_definitions WHERE key = 'score_n'",
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
    b.define("score_n", "number", false);
    let copy = "settings/properties (PC's conflicted copy 2026-11-15).json";
    std::fs::copy(b.root.join(DEFINITIONS_FILE), a.root.join(copy)).unwrap();

    assert_eq!(a.sync(copy), Outcome::Imported);

    assert_eq!(a.definition("score_n"), Some(("number".to_string(), false)));
    assert!(a.written("score_n").is_some());
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
    b.define("score_n", "number", false);
    a.deliver(&b, DEFINITIONS_FILE);
    let copy = "settings/properties (PC's conflicted copy 2026-11-15).json";
    std::fs::write(
        a.root.join(copy),
        r#"{ "definitions": { "owner": { "type": "text", "page": true, "modified": "2025-11-15T01:00:00Z" } } }"#,
    )
    .unwrap();

    a.state.reconcile.scan();

    assert_eq!(a.definition("score_n"), Some(("number".to_string(), false)));
    assert_eq!(a.definition("owner"), Some(("text".to_string(), true)));
    assert!(!a.root.join(copy).exists());
}

// 端末は、settings/properties.jsonを取り込んでも、ページのファイルを書き直さず、ページの更新日時も進めない。値も変わらない。
#[test]
fn taking_in_the_definitions_leaves_the_pages_and_their_files_alone() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score_n", "number", true);
    a.state
        .page_service
        .set_property(&page, "score_n", "number", "12")
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
    b.define("score_n", "number", false);

    a.receive(&b, DEFINITIONS_FILE);
    a.export();

    assert_eq!(a.definition("score_n"), Some(("number".to_string(), false)));
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
    assert_eq!(scalars[page.as_str()]["score_n"], "12");
}

// 読めないsettings/properties.jsonは取り込まない。端末のDBの定義も、ファイルも変わらない。conflicted copyも同じで、読めなければ消さずに残す。
#[test]
fn a_file_of_definitions_that_cannot_be_read_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    a.define("owner", "text", true);
    let broken = r#"{ "definitions": { "score_n": "#;
    let copy = "settings/properties (PC's conflicted copy 2026-11-15).json";
    std::fs::write(a.root.join(DEFINITIONS_FILE), broken).unwrap();
    std::fs::write(a.root.join(copy), broken).unwrap();

    assert_eq!(a.sync(DEFINITIONS_FILE), Outcome::Error);
    assert_eq!(a.sync(copy), Outcome::Error);

    assert_eq!(a.definition("owner"), Some(("text".to_string(), true)));
    assert_eq!(a.definition("score_n"), None);
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
    a.define("score_n", "number", true);
    a.define("author_list", "list", true);
    a.state
        .page_service
        .set_property(&page, "score_n", "number", "12")
        .unwrap();
    a.state
        .page_service
        .set_property(&page, "author_list", "list", "x, y")
        .unwrap();

    b.receive(&a, "docs/d/a.md");
    b.define("owner", "text", true);

    assert_eq!(b.definition("score_n"), Some(("number".to_string(), true)));
    assert_eq!(
        b.definition("author_list"),
        Some(("list".to_string(), true))
    );
    assert!(b.is_provisional("score_n"));
    assert!(b.is_provisional("author_list"));
    assert_eq!(b.written("score_n"), None);
    assert_eq!(b.written("author_list"), None);
}

// ページのファイルを取り込んで、仮の定義のキーの値が端末のDBからなくなると、端末は、その仮の定義を消す。
#[test]
fn a_provisional_definition_goes_with_the_last_value_of_its_key() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score_n", "number", true);
    a.state
        .page_service
        .set_property(&page, "score_n", "number", "12")
        .unwrap();
    b.receive(&a, "docs/d/a.md");
    std::thread::sleep(std::time::Duration::from_millis(5));
    a.state
        .page_service
        .delete_property(&page, "score_n")
        .unwrap();

    b.receive(&a, "docs/d/a.md");

    assert_eq!(b.definition("score_n"), None);
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
    a.define("score_n", "number", false);
    a.state
        .item_service
        .set_property(&item, "score_n", "number", "12")
        .unwrap();

    b.receive(&a, "docs/d/a.md");

    assert_eq!(b.definition("score_n"), Some(("number".to_string(), true)));
    assert!(b.is_provisional("score_n"));

    std::thread::sleep(std::time::Duration::from_millis(5));
    a.state
        .item_service
        .delete_property(&item, "score_n")
        .unwrap();
    b.receive(&a, "docs/d/a.md");

    assert_eq!(b.definition("score_n"), None);
}

// ユーザーがRESTやMCPで定義のないキーに項目の値を書いても、端末は定義を作らず、settings/properties.jsonにも書かない。型は定義するときだけ決まるからである（ADR 047）。
#[test]
fn a_value_the_user_writes_to_a_key_with_no_definition_makes_no_definition() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
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

    let written = a.state.journal.set_item_property(
        item.as_str(),
        "member_list",
        journal_core::journal::PropertyInput {
            value_type: Some("list".to_string()),
            value: Some("p, q".to_string()),
        },
        &ChangedBy::Mcp,
    );

    assert!(written.is_err());
    assert_eq!(a.definition("member_list"), None);
    assert_eq!(a.written("member_list"), None);
}

// ほかの端末が消した定義が届いたあと、ユーザーがRESTやMCPでそのキーに値を書いても、端末は書き込みを断り、消したことを残す。キーを使い直すには、定義し直す（ADR 047）。
#[test]
fn a_value_the_user_writes_to_a_key_deleted_on_another_device_is_refused_and_the_deletion_stays() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score_n", "number", false);
    b.receive(&a, DEFINITIONS_FILE);
    b.undefine("score_n");
    a.receive(&b, DEFINITIONS_FILE);

    let written = a.state.journal.set_page_property(
        page.as_str(),
        "score_n",
        Some("number"),
        Some("12"),
        &ChangedBy::Mcp,
    );

    assert!(written.is_err());
    let deleted: i64 = a
        .state
        .db
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM deleted_property_definitions WHERE key = 'score_n'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(deleted, 1);
    assert!(matches!(
        a.written("score_n"),
        Some(Definition::Deleted { .. })
    ));
}

// 値があって仮の定義になったキーに、ユーザーがRESTやMCPで値を書いても、端末は、仮の定義のまま残し、消したことも残す。ユーザーは型を選んでいないからである。
#[test]
fn a_value_the_user_writes_to_a_provisional_key_leaves_it_provisional_and_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score_n", "number", true);
    a.set_score(&page, "12");
    b.receive(&a, DEFINITIONS_FILE);
    b.undefine("score_n");
    a.receive(&b, DEFINITIONS_FILE);

    a.set_score(&page, "13");

    assert!(a.is_provisional("score_n"));
    let deleted: i64 = a
        .state
        .db
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM deleted_property_definitions WHERE key = 'score_n'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(deleted, 1);
    assert!(matches!(
        a.written("score_n"),
        Some(Definition::Deleted { .. })
    ));
}

impl Device {
    fn set_score(&self, page: &hmtid::Hmtid, value: &str) {
        self.state
            .journal
            .set_page_property(
                page.as_str(),
                "score_n",
                Some("text"),
                Some(value),
                &ChangedBy::Mcp,
            )
            .unwrap();
    }
}

/// What the clients of a device are told, from when it begins to listen.
struct Told(tokio::sync::broadcast::Receiver<String>);

impl Told {
    /// Whether the clients were told, since the last look, that the definitions changed.
    fn of_definitions(&mut self) -> bool {
        let mut told = false;
        while let Ok(message) = self.0.try_recv() {
            told |= message.contains("\"propertyDefinitionsUpdated\"");
        }
        told
    }
}

impl Device {
    fn listen(&self) -> Told {
        Told(self.state.sse.subscribe())
    }
}

// ほかの端末の定義がsettings/properties.jsonで届き、端末のDBの定義が変わると、端末は、開いているクライアントに定義が変わったことを知らせる。
#[test]
fn the_clients_are_told_when_a_definition_from_another_device_comes_in() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    b.define("score_n", "number", false);
    let mut told = a.listen();

    a.receive(&b, DEFINITIONS_FILE);

    assert!(told.of_definitions());
}

// 届いたファイルを取り込んでも端末のDBの定義が変わらなければ、端末はクライアントに知らせない。ページのファイルも、settings/properties.jsonも同じである。
#[test]
fn the_clients_are_not_told_when_a_file_leaves_the_definitions_alone() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    b.define("score_n", "number", false);
    a.receive(&b, DEFINITIONS_FILE);
    b.edit(&page, "edited on B");
    a.receive(&b, "docs/d/a.md");
    let mut told = a.listen();

    a.receive(&b, DEFINITIONS_FILE);
    b.edit(&page, "edited on B again");
    a.receive(&b, "docs/d/a.md");

    assert!(!told.of_definitions());
    assert!(a.file("docs/d/a.md").contains("edited on B again"));
}

// ページのファイルで仮の定義ができたときも、なくなったときも、端末はクライアントに知らせる。設定の画面の一覧には、仮の定義も出るからである。
#[test]
fn the_clients_are_told_when_a_page_file_makes_or_drops_a_provisional_definition() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.define("score_n", "number", true);
    a.state
        .page_service
        .set_property(&page, "score_n", "number", "12")
        .unwrap();
    let mut told = b.listen();

    b.receive(&a, "docs/d/a.md");

    assert!(told.of_definitions());
    std::thread::sleep(std::time::Duration::from_millis(5));
    a.state
        .page_service
        .delete_property(&page, "score_n")
        .unwrap();
    b.receive(&a, "docs/d/a.md");
    assert!(told.of_definitions());
}

// ユーザーがRESTやMCPで定義を足したときと消したときも、端末はクライアントに知らせる。
#[test]
fn the_clients_are_told_when_the_user_changes_the_definitions() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let mut told = a.listen();

    a.define("score_n", "number", false);
    assert!(told.of_definitions());
    a.undefine("score_n");
    assert!(told.of_definitions());
}

// ユーザーがいくつものキーを同時に定義しても、端末は、どの定義も受け付け、すべての定義をsettings/properties.jsonに書く。端末は、定義のファイルを1つずつ書くからである。
#[test]
fn keys_defined_at_once_all_get_their_definitions_in_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let keys = |t: usize| (0..20).map(move |i| format!("key_{t}_{i}"));

    let writers: Vec<_> = (0..8)
        .map(|t| {
            let state = a.state.clone();
            std::thread::spawn(move || {
                for key in keys(t) {
                    state
                        .journal
                        .add_property_definition(AddPropertyDefinitionInput {
                            key: Some(key),
                            value_type: None,
                            page: None,
                        })
                        .unwrap();
                }
            })
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }

    for key in (0..8).flat_map(keys) {
        assert!(a.written(&key).is_some(), "{key}");
    }
}
