//! The properties of a page taken in from the front matter of its file. A device that got a
//! page only from its file holds them, so the file keeps them when that device writes it.

use hmtid::Hmtid;
use journal_core::infrastructure::export::import_service::Outcome;

use super::devices::{two_devices, Device};

const PAGE: &str = "docs/d/a.md";

impl Device {
    fn set_page_property(&self, page_id: &Hmtid, key: &str, value_type: &str, value: &str) {
        self.ensure_defined(key);
        self.state
            .page_service
            .set_property(page_id, key, value_type, value)
            .unwrap();
    }

    /// The scalar property of the page, as the database holds it
    fn page_property(&self, page_id: &Hmtid, key: &str) -> Option<String> {
        let (scalars, _) = self.state.page_service.page_properties(page_id).unwrap();
        scalars.get(page_id.as_str())?.get(key).cloned()
    }

    /// Adds the lines to the front matter of the file by hand, and takes the file in
    fn add_to_front_matter(&self, lines: &str) -> Outcome {
        let before = self.file(PAGE);
        let edited = before.replacen("---\n\n", &format!("{lines}---\n\n"), 1);
        assert_ne!(before, edited);
        std::fs::write(self.root.join(PAGE), edited).unwrap();
        self.sync(PAGE)
    }

    /// The list property of the page, as the database holds it
    fn page_list(&self, page_id: &Hmtid, key: &str) -> Option<Vec<String>> {
        let (_, lists) = self.state.page_service.page_properties(page_id).unwrap();
        lists.get(page_id.as_str())?.get(key).cloned()
    }

    fn type_of(&self, key: &str) -> Option<String> {
        let definition = self.state.properties.find_definition(key).unwrap()?;
        Some(definition.value_type)
    }
}

// ファイルだけからできたページは、front matterのプロパティを持ち、書き直してもファイルに残す。
#[test]
fn a_page_made_from_a_file_has_the_properties_of_its_front_matter_and_writes_them_again() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.set_page_property(&page, "slug", "text", "my-page");

    b.receive(&a, PAGE);

    assert_eq!(b.page_property(&page, "slug").as_deref(), Some("my-page"));
    b.edit(&page, "edited on B");
    assert!(b.file(PAGE).contains("slug: my-page\n"));
}

// 新しいファイルにないプロパティは、ページから消える。
#[test]
fn a_property_missing_from_a_newer_file_goes_from_the_page() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.set_page_property(&page, "slug", "text", "my-page");
    b.receive(&a, PAGE);
    a.state.page_service.delete_property(&page, "slug").unwrap();

    b.receive(&a, PAGE);

    assert_eq!(b.page_property(&page, "slug"), None);
}

// ページより古いファイルは、ページのプロパティを変えない。
#[test]
fn an_older_file_leaves_the_properties_of_the_page() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.set_page_property(&page, "slug", "text", "from-a");
    b.receive(&a, PAGE);
    a.edit(&page, "edited on A");
    a.export();
    b.set_page_property(&page, "slug", "text", "from-b");
    b.set_page_property(&page, "author", "text", "b");

    b.receive(&a, PAGE);

    assert_eq!(b.page_property(&page, "slug").as_deref(), Some("from-b"));
    assert_eq!(b.page_property(&page, "author").as_deref(), Some("b"));
}

// 手でfront matterに足したプロパティは取り込まれ、ページの日付が新しくなる。
#[test]
fn a_property_added_by_hand_is_taken_in_and_gives_the_page_a_new_date() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let page = a.doc("a");
    let before = a.file(PAGE);

    a.add_to_front_matter("slug: by-hand\n");

    assert_eq!(a.page_property(&page, "slug").as_deref(), Some("by-hand"));
    let date = |file: &str| {
        file.lines()
            .find(|l| l.starts_with("modified: "))
            .map(str::to_string)
    };
    assert_ne!(date(&a.file(PAGE)), date(&before));
}

// 定義のないキーは、名前から決まる型で定義される。
#[test]
fn a_key_with_no_definition_is_defined_with_the_type_its_name_gives() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.set_page_property(&page, "rating_n", "number", "5");

    b.receive(&a, PAGE);

    assert_eq!(b.type_of("rating_n").as_deref(), Some("number"));
    assert_eq!(b.page_property(&page, "rating_n").as_deref(), Some("5"));
}

// ページに付けられないシステム管理キーは取り込まれず、ほかのプロパティは取り込まれる。
#[test]
fn a_key_a_page_cannot_have_is_left_out_and_the_others_are_taken_in() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let page = a.doc("a");

    let outcome = a.add_to_front_matter("due: 2026-01-02\nslug: by-hand\n");

    assert_eq!(outcome, Outcome::Imported);
    assert_eq!(a.page_property(&page, "due"), None);
    assert_eq!(a.page_property(&page, "slug").as_deref(), Some("by-hand"));
}

// 項目だけに使うと定義したユーザーのキーも、ページのファイルから取り込まれる。ほかの端末では、ページに付けられる定義かもしれないからである。
#[test]
fn a_user_key_defined_for_items_only_is_taken_in_on_a_page() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let page = a.doc("a");
    a.state
        .properties
        .insert_definition("effort_n", "number", false)
        .unwrap();

    let outcome = a.add_to_front_matter("effort_n: 3\n");

    assert_eq!(outcome, Outcome::Imported);
    assert_eq!(a.page_property(&page, "effort_n").as_deref(), Some("3"));
}

// キーにできない名前の行は取り込まれず、ほかのプロパティは取り込まれる。
#[test]
fn a_line_whose_name_cannot_be_a_key_is_left_out_and_the_others_are_taken_in() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let page = a.doc("a");

    let outcome = a.add_to_front_matter("my key: x\nslug: by-hand\n");

    assert_eq!(outcome, Outcome::Imported);
    assert_eq!(a.type_of("my key"), None);
    assert_eq!(a.page_property(&page, "slug").as_deref(), Some("by-hand"));
}

// ユーザーのキーの値は、型に合わなくても、ファイルに書かれた文字列のまま取り込まれる。型は値の読み方なので、型が変わっても値が消えないようにするためである。
#[test]
fn a_value_of_a_user_key_that_does_not_fit_the_type_is_taken_in_as_written() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let page = a.doc("a");
    a.set_page_property(&page, "rating_n", "number", "5");
    let file = a
        .file(PAGE)
        .replacen("rating_n: 5\n", "rating_n: many\nslug: by-hand\n", 1);
    std::fs::write(a.root.join(PAGE), file).unwrap();

    let outcome = a.sync(PAGE);

    assert_eq!(outcome, Outcome::Imported);
    assert_eq!(a.page_property(&page, "rating_n").as_deref(), Some("many"));
    assert_eq!(a.page_property(&page, "slug").as_deref(), Some("by-hand"));
}

// リストのプロパティは、手で値を足しても、リストとして取り込まれる。
#[test]
fn a_list_property_edited_by_hand_is_taken_in_as_a_list() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let page = a.doc("a");
    a.set_page_property(&page, "author_list", "list", "x,y");
    let file = a.file(PAGE).replacen("  - y\n", "  - y\n  - z\n", 1);
    std::fs::write(a.root.join(PAGE), file).unwrap();

    a.sync(PAGE);

    let authors = a.page_list(&page, "author_list").unwrap();
    assert_eq!(authors, ["x", "y", "z"]);
    assert_eq!(a.page_property(&page, "author_list"), None);
}

// 定義のない端末を通ったリストは、定義のある端末に戻ると、リストのままである。次に書くファイルでも、リストである。
#[test]
fn a_list_that_went_through_a_device_without_its_definition_is_still_a_list_back_home() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.set_page_property(&page, "author_list", "list", "x,y");
    b.receive(&a, PAGE);
    b.edit(&page, "edited on B");

    a.receive(&b, PAGE);

    assert_eq!(a.page_list(&page, "author_list").unwrap(), ["x", "y"]);
    a.edit(&page, "edited on A");
    assert!(a.file(PAGE).contains("author_list:\n  - x\n  - y\n"));
}

// 定義のない端末でも、名前が_listで終わるキーは、リストとして取り込まれる。
#[test]
fn a_key_that_ends_with_list_is_taken_in_as_a_list_on_a_device_without_its_definition() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.set_page_property(&page, "author_list", "list", "x,y");

    b.receive(&a, PAGE);

    assert_eq!(b.page_list(&page, "author_list").unwrap(), ["x", "y"]);
    assert_eq!(b.page_property(&page, "author_list"), None);
}

// 新しいファイルにないリストのプロパティは、ページから消える。タグは別に扱う。
#[test]
fn a_list_property_missing_from_a_newer_file_goes_and_the_tags_stay() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let page = a.doc("a");
    a.set_page_property(&page, "author_list", "list", "x,y");
    a.set_page_property(&page, "tags", "tags", "kept");
    let file = a.file(PAGE).replacen("author_list:\n  - x\n  - y\n", "", 1);
    std::fs::write(a.root.join(PAGE), file).unwrap();

    a.sync(PAGE);

    assert_eq!(a.page_list(&page, "author_list"), None);
    assert_eq!(a.state.page_service.page_tags(&page).unwrap(), ["kept"]);
}
