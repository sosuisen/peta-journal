//! Properties: the keys the user defines, the values of items and pages, the tag pages, and
//! the values written to the markdown files and taken back from them.

use journal_core::error::AppError;
use journal_core::journal::{PageAnswer, PropertyDefinitionsAnswer};
use serde_json::{json, Value};

use crate::common::items::{answer, input};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Adds a definition, as the user does in Settings
    fn add_definition(&self, request: Value) -> Result<PropertyDefinitionsAnswer, AppError> {
        self.core.journal.add_property_definition(input(request))
    }

    fn delete_definition(&self, key: &str) -> Result<PropertyDefinitionsAnswer, AppError> {
        self.core
            .journal
            .delete_property_definition(key, &by_the_client())
    }

    /// Sets a value of the page, as the client does
    fn put_page_property(
        &self,
        page_id: &str,
        key: &str,
        value_type: &str,
        value: &str,
    ) -> Result<PageAnswer, AppError> {
        self.core.journal.set_page_property(
            page_id,
            key,
            Some(value_type),
            Some(value),
            &by_the_client(),
        )
    }

    fn remove_page_property(&self, page_id: &str, key: &str) -> Result<PageAnswer, AppError> {
        self.core
            .journal
            .delete_page_property(page_id, key, &by_the_client())
    }

    /// The properties of the page, as the client reads them
    fn page_properties(&self, page_id: &str) -> Value {
        self.page(page_id)["page"]["properties"].clone()
    }

    /// The ids of the doc pages with this name in any case of its letters.
    /// Makes an empty doc page of the name in the folder, beside a page whose name differs
    /// only in case. The core finds a page ignoring case now, so the page is made under
    /// another name and renamed in the database, as such pages were made before.
    fn page_made_before_case_was_ignored(&self, dir_path: &str, name: &str) {
        let made = self.open_doc_in(&format!("{name}-made"), dir_path)["page"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        self.core
            .db
            .conn()
            .unwrap()
            .execute(
                "UPDATE pages SET name = ? WHERE id = ?",
                rusqlite::params![name, made],
            )
            .unwrap();
    }

    fn doc_pages_named_ignoring_case(&self, name: &str) -> Vec<String> {
        let recent = answer(
            self.core
                .journal
                .recent_pages(Some(2000), Some("doc"))
                .unwrap(),
        );
        recent["pages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["name"].as_str().unwrap().eq_ignore_ascii_case(name))
            .map(|p| p["id"].as_str().unwrap().to_string())
            .collect()
    }

    /// Gives the page another title, as the client does
    fn retitle(&self, page_id: &str, title: &str) -> Result<PageAnswer, AppError> {
        self.core
            .journal
            .update_page(page_id, Some(title), None, false, &by_the_client())
    }

    /// The markdown file of the doc page at the root, after the pages that wait are written
    fn file_of_doc(&self, title: &str) -> String {
        self.core.export.process_whole_queue();
        std::fs::read_to_string(self.markdown_path(title)).unwrap_or_default()
    }

    fn markdown_path(&self, title: &str) -> std::path::PathBuf {
        self.root.join("docs").join(format!("{title}.md"))
    }

    /// Edits a page file as an outside editor would, and scans the folder. The `modified`
    /// line goes, so the file's own time makes it newer than the page.
    fn edit_file_and_scan(&self, title: &str, edit: impl Fn(&str) -> String) {
        let text: String = self
            .file_of_doc(title)
            .lines()
            .filter(|l| !l.starts_with("modified: "))
            .map(|l| format!("{l}\n"))
            .collect();
        std::fs::write(self.markdown_path(title), edit(&text)).unwrap();
        self.core.reconcile.scan();
    }

    /// The first item of the page
    fn first_item(&self, page_id: &str) -> Value {
        self.items(page_id).into_iter().next().unwrap()
    }
}

fn definitions(answer_of: PropertyDefinitionsAnswer) -> Value {
    answer(answer_of)
}

fn definition<'a>(resp: &'a Value, key: &str) -> Option<&'a Value> {
    resp["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["key"] == key)
}

fn is_bad_request<T: std::fmt::Debug>(result: &Result<T, AppError>) -> bool {
    matches!(result, Err(AppError::BadRequest(_)))
}

// プロパティ定義にはシステムのキーが並び、ユーザーのキーを追加できる。
#[test]
fn property_definitions_list_the_system_keys_and_accept_user_keys() {
    let notes = Core::new();

    let resp = definitions(notes.core.journal.property_definitions().unwrap());
    assert_eq!(resp["propertyDefinitions"]["zoom"], json!("number"));
    assert_eq!(definition(&resp, "tags").unwrap()["system"], json!(true));
    assert_eq!(definition(&resp, "tags").unwrap()["page"], json!(true));
    assert_eq!(definition(&resp, "zoom").unwrap()["page"], json!(false));
    assert_eq!(
        definition(&resp, "modified").unwrap()["system"],
        json!(true)
    );
    assert_eq!(definition(&resp, "due").unwrap()["system"], json!(true));
    assert_eq!(
        definition(&resp, "cancelled").unwrap()["page"],
        json!(false)
    );
    assert_eq!(definition(&resp, "created").unwrap()["page"], json!(true));
    assert_eq!(
        definition(&resp, "published").unwrap()["system"],
        json!(true)
    );
    assert_eq!(definition(&resp, "published").unwrap()["page"], json!(true));
    assert_eq!(resp["propertyDefinitions"]["published"], json!("datetime"));
    assert_eq!(
        definition(&resp, "atlas_bounds").unwrap()["system"],
        json!(true)
    );
    assert_eq!(
        definition(&resp, "atlas_bounds").unwrap()["page"],
        json!(false)
    );
    assert_eq!(resp["propertyDefinitions"]["atlas_bounds"], json!("bounds"));

    let resp = definitions(
        notes
            .add_definition(json!({ "key": "status", "valueType": "text" }))
            .unwrap(),
    );
    let added = definition(&resp, "status").unwrap();
    assert_eq!(added["valueType"], json!("text"));
    assert_eq!(added["system"], json!(false));
    assert_eq!(added["page"], json!(true));

    for body in [
        json!({ "key": "zoom", "valueType": "text" }),
        json!({ "key": "status", "valueType": "text" }),
        json!({ "key": "has space", "valueType": "text" }),
        json!({ "key": "", "valueType": "text" }),
        json!({ "key": "kind", "valueType": "tags" }),
    ] {
        let added = notes.add_definition(body.clone());
        assert!(is_bad_request(&added), "{body} -> {added:?}");
    }
}

// キーは、名前の接尾辞から決まる型でだけ定義できる。
#[test]
fn a_key_is_defined_only_with_the_type_its_suffix_gives() {
    let notes = Core::new();
    for (key, value_type) in [
        ("defined_score_n", "number"),
        ("defined_due_d", "date"),
        ("defined_seen_dt", "datetime"),
        ("defined_read_cb", "checkbox"),
        ("defined_place_bounds", "bounds"),
        ("defined_member_list", "list"),
    ] {
        let resp = definitions(
            notes
                .add_definition(json!({ "key": key, "valueType": value_type }))
                .unwrap(),
        );
        assert_eq!(
            definition(&resp, key).unwrap()["valueType"],
            json!(value_type)
        );
    }

    for body in [
        json!({ "key": "defined_score", "valueType": "number" }),
        json!({ "key": "defined_when_d", "valueType": "text" }),
        json!({ "key": "defined_count_n", "valueType": "date" }),
        json!({ "key": "defined_members", "valueType": "list" }),
        json!({ "key": "defined_owner_list", "valueType": "text" }),
    ] {
        let added = notes.add_definition(body.clone());
        assert!(is_bad_request(&added), "{body} -> {added:?}");
    }
}

// 名前の本体が予約の接尾辞で終わるキーは、定義できない。
#[test]
fn a_key_whose_body_ends_with_a_reserved_suffix_is_not_defined() {
    let notes = Core::new();
    for body in [
        json!({ "key": "defined_score_n_n", "valueType": "number" }),
        json!({ "key": "defined_score_n_list", "valueType": "list" }),
    ] {
        let added = notes.add_definition(body.clone());
        assert!(is_bad_request(&added), "{body} -> {added:?}");
    }
}

// 名前の本体がほかのユーザーのキーと同じなら、別の型でも定義できない。消したキーの本体なら、別の型で定義できる。
#[test]
fn a_key_whose_body_another_key_has_is_not_defined_until_that_key_is_deleted() {
    let notes = Core::new();
    notes
        .add_definition(json!({ "key": "shared_body_n" }))
        .unwrap();

    let refused = notes.add_definition(json!({ "key": "shared_body_d" }));
    assert!(is_bad_request(&refused), "{refused:?}");

    notes.delete_definition("shared_body_n").unwrap();
    notes
        .add_definition(json!({ "key": "shared_body_d" }))
        .unwrap();
}

// 型を送らずに定義すると、キーは名前から決まる型になる。
#[test]
fn a_key_defined_without_a_type_takes_the_type_its_name_gives() {
    let notes = Core::new();

    let resp = definitions(
        notes
            .add_definition(json!({ "key": "untyped_score_n" }))
            .unwrap(),
    );

    assert_eq!(
        definition(&resp, "untyped_score_n").unwrap()["valueType"],
        json!("number")
    );
}

// 項目からプロパティを削除できる。
#[test]
fn a_property_can_be_removed_from_an_item() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-property-delete");
    let item = notes.fill(&page, &["task-like"]).remove(0);
    let item_id = item["id"].as_str().unwrap();
    notes
        .set_item_property(
            item_id,
            "plan",
            json!({ "valueType": "date", "value": "2026-03-01" }),
        )
        .unwrap();
    assert_eq!(
        notes.items(&page)[0]["properties"]["plan"],
        json!("2026-03-01")
    );

    let resp = answer(
        notes
            .core
            .journal
            .delete_item_property(item_id, "plan", &by_the_client())
            .unwrap(),
    );

    assert!(resp["item"]["properties"].get("plan").is_none());
    assert!(notes.items(&page)[0]["properties"].get("plan").is_none());
}

// ページのプロパティは設定と削除ができ、フロントマターに書き出される。
#[test]
fn page_properties_are_set_removed_and_written_to_the_front_matter() {
    let notes = Core::new();
    let page = notes.new_doc_page("page-properties");
    notes
        .add_definition(json!({ "key": "owner", "valueType": "text" }))
        .unwrap();

    let resp = answer(
        notes
            .put_page_property(&page, "owner", "text", "me: now")
            .unwrap(),
    );
    assert_eq!(resp["page"]["properties"]["owner"], json!("me: now"));

    let resp = answer(
        notes
            .put_page_property(&page, "tags", "tags", "a, b")
            .unwrap(),
    );
    assert_eq!(resp["page"]["properties"]["tags"], json!(["a", "b"]));

    // An item-only system key stays off the page
    let zoom = notes.put_page_property(&page, "zoom", "number", "2");
    assert!(is_bad_request(&zoom), "{zoom:?}");

    let properties = notes.page_properties(&page);
    assert_eq!(properties["owner"], json!("me: now"));
    assert_eq!(properties["tags"], json!(["a", "b"]));

    let text = notes.file_of_doc("page-properties");
    assert!(
        text.contains("owner: \"me: now\"\n") && text.contains("tags:\n  - a\n  - b\n---\n"),
        "{text}"
    );

    let resp = answer(notes.remove_page_property(&page, "owner").unwrap());
    assert!(resp["page"]["properties"].get("owner").is_none());
    assert_eq!(resp["page"]["properties"]["tags"], json!(["a", "b"]));

    let text = notes.file_of_doc("page-properties");
    assert!(!text.contains("owner:"), "{text}");
}

// ページのユーザー定義のlist型のキーは、そのキーのリストとして保存され、タグには触らない。
#[test]
fn a_list_of_a_page_is_kept_under_its_own_key_apart_from_the_tags() {
    let notes = Core::new();
    notes.define_key("page_list");
    let page = notes.new_doc_page("page-list");
    notes.set_tags(&page, "page-list-tag");

    // The second value replaces the first, as a value of a scalar does
    for value in ["list-x, list-y", "list-y,list-z"] {
        notes
            .put_page_property(&page, "page_list", "list", value)
            .unwrap();
    }

    let properties = notes.page_properties(&page);
    assert_eq!(properties["page_list"], json!(["list-y", "list-z"]));
    assert_eq!(properties["tags"], json!(["page-list-tag"]));
    assert!(notes.doc_page_named("list-y").is_none());
}

// 定義のないキーには、ページの値を書けない。名前に合う型でも、合わない型でも同じである（ADR 047）。
#[test]
fn a_key_with_no_definition_takes_no_value_of_a_page() {
    let notes = Core::new();
    let page = notes.new_doc_page("page-type-by-name");

    for (key, value_type, value) in [
        ("undefined_page_note", "text", "x"),
        ("undefined_page_score_n", "number", "1"),
        ("undefined_page_members", "list", "x"),
        ("undefined_page_note_list", "text", "x"),
    ] {
        let set = notes.put_page_property(&page, key, value_type, value);
        assert!(is_bad_request(&set), "{key} {value_type} -> {set:?}");
    }
}

// 新しいページタグにはタグページが作られ、最後の使用がなくなると消える。
#[test]
fn a_new_page_tag_gets_its_tag_page_and_the_last_use_takes_it_away() {
    let notes = Core::new();
    let tag = "page-tag-pages-new";
    let first = notes.new_doc_page("page-tag-pages-first");
    let second = notes.new_doc_page("page-tag-pages-second");

    notes.set_tags(&first, tag);
    notes.set_tags(&second, tag);
    assert!(notes.doc_page_named(tag).is_some());

    // The second page still uses the tag
    notes.set_tags(&first, "page-tag-pages-other");
    assert!(notes.doc_page_named(tag).is_some());

    notes.remove_page_property(&second, "tags").unwrap();
    assert!(notes.doc_page_named(tag).is_none());
}

// 大文字と小文字だけが違うタグは、1つのタグページを共有する。
#[test]
fn tags_that_differ_only_in_case_share_one_tag_page() {
    let notes = Core::new();
    let first = notes.new_doc_page("case-tag-pages-first");
    let second = notes.new_doc_page("case-tag-pages-second");

    notes.set_tags(&first, "Case-Tag-Pages");
    notes.set_tags(&second, "case-tag-pages");
    assert_eq!(
        notes.doc_pages_named_ignoring_case("case-tag-pages").len(),
        1
    );

    // The other case still uses the page
    notes.set_tags(&first, "case-tag-pages-other");
    assert_eq!(
        notes.doc_pages_named_ignoring_case("case-tag-pages").len(),
        1
    );

    notes.set_tags(&second, "case-tag-pages-other");
    assert!(notes
        .doc_pages_named_ignoring_case("case-tag-pages")
        .is_empty());
}

// 起動のたびに、タグページのないタグにタグページが作られ、作った数が返る。タグページができる前に付けたタグのためである。
#[test]
fn the_start_up_makes_the_missing_tag_pages_and_counts_them() {
    let notes = Core::new();
    let page = notes.new_doc_page("missing-tag-pages");
    notes.set_tags(&page, "missing-tag-a, missing-tag-b");
    for tag in ["missing-tag-a", "missing-tag-b"] {
        let tag_page = notes.doc_page_named(tag).unwrap();
        notes
            .core
            .page_service
            .delete_page(&tag_page.parse().unwrap())
            .unwrap();
    }

    assert_eq!(notes.core.page_service.ensure_all_tag_pages().unwrap(), 2);
    assert!(notes.doc_page_named("missing-tag-a").is_some());
    assert!(notes.doc_page_named("missing-tag-b").is_some());
    assert_eq!(notes.core.page_service.ensure_all_tag_pages().unwrap(), 0);
}

// 起動のたびに、ほかのタグページと大文字と小文字だけが違う空のタグページが消され、消した数が返る。
#[test]
fn the_start_up_deletes_the_empty_tag_pages_that_repeat_another_in_case_and_counts_them() {
    let notes = Core::new();
    let page = notes.new_doc_page("repeated-tag-pages");
    notes.set_tags(&page, "Repeated-Tag");
    for name in ["REPEATED-TAG", "repeated-tag"] {
        notes.page_made_before_case_was_ignored("tags/", name);
    }
    assert_eq!(notes.doc_pages_named_ignoring_case("repeated-tag").len(), 3);

    assert_eq!(
        notes
            .core
            .page_service
            .delete_case_duplicate_tag_pages()
            .unwrap(),
        2
    );
    assert_eq!(notes.doc_pages_named_ignoring_case("repeated-tag").len(), 1);
    assert_eq!(
        notes
            .core
            .page_service
            .delete_case_duplicate_tag_pages()
            .unwrap(),
        0
    );
}

// 使われなくなっても、内容のあるタグページは残る。
#[test]
fn an_unused_tag_page_with_content_stays() {
    let notes = Core::new();
    let tag = "page-tag-pages-written";
    let page = notes.new_doc_page("page-tag-pages-written-page");
    notes.set_tags(&page, tag);
    let tag_page = notes.doc_page_named(tag).unwrap();
    let first = notes.first_item(&tag_page);
    notes.update_item(first["id"].as_str().unwrap(), json!({ "body": "a note" }));

    notes.set_tags(&page, "page-tag-pages-written-other");
    assert_eq!(notes.doc_page_named(tag), Some(tag_page));
}

// 本文のハッシュタグも、ページタグと同じようにタグページを保つ。
#[test]
fn a_hashtag_in_a_body_keeps_its_tag_page_as_a_page_tag_does() {
    let notes = Core::new();
    let tag = "body-tag-pages-new";
    let page = notes.new_doc_page("body-tag-pages");
    let item = notes.first_item(&page);
    let item_id = item["id"].as_str().unwrap();

    notes.update_item(item_id, json!({ "body": format!("see #{tag}") }));
    assert!(notes.doc_page_named(tag).is_some());

    notes.update_item(item_id, json!({ "body": "no tag now" }));
    assert!(notes.doc_page_named(tag).is_none());

    let added = notes.create_item(&page, &format!("after:{item_id}"), 1, &format!("#{tag}"));
    assert!(notes.doc_page_named(tag).is_some());
    notes.delete_item(added["id"].as_str().unwrap());
    assert!(notes.doc_page_named(tag).is_none());
}

// タグページの改名で、ページタグと本文のタグも改名される。
#[test]
fn renaming_a_tag_page_renames_its_tag_in_page_tags_and_bodies() {
    let notes = Core::new();
    let old = "renamed-tag-old";
    let new = "renamed-tag-new";
    let tagged = notes.new_doc_page("renamed-tag-page");
    notes.set_tags(&tagged, &format!("{old},other"));
    let page = notes.new_doc_page("renamed-tag-body");
    let item = notes.first_item(&page);
    notes.update_item(
        item["id"].as_str().unwrap(),
        json!({ "body": format!("see #{old} and `#{old}`") }),
    );
    let tag_page = notes.doc_page_named(old).unwrap();

    notes.retitle(&tag_page, new).unwrap();

    assert_eq!(
        notes.page_properties(&tagged)["tags"],
        json!([new, "other"])
    );
    let item = notes.first_item(&page);
    assert_eq!(item["body"], json!(format!("see #{new} and `#{old}`")));
    assert_eq!(item["properties"]["tags"], json!([new]));
    assert_eq!(notes.doc_page_named(new), Some(tag_page.clone()));
    assert!(notes.doc_page_named(old).is_none());
}

// タグページをNFDのタイトルへ改名すると、本文とタグにはNFCの名前が入る。
#[test]
fn renaming_a_tag_page_to_an_nfd_title_writes_the_tag_in_nfc() {
    let notes = Core::new();
    let old = "nfd-tag-old";
    let page = notes.new_doc_page("nfd-tag-body");
    let item = notes.first_item(&page);
    notes.update_item(
        item["id"].as_str().unwrap(),
        json!({ "body": format!("see #{old}") }),
    );
    let tag_page = notes.doc_page_named(old).unwrap();

    notes.retitle(&tag_page, "\u{30ab}\u{3099}-tag").unwrap();

    let new = "\u{30ac}-tag";
    let item = notes.first_item(&page);
    assert_eq!(item["body"], json!(format!("see #{new}")));
    assert_eq!(item["properties"]["tags"], json!([new]));
}

// タグページをタグフォルダの中の別のフォルダへ動かすと、タグもそのフォルダの名前で付け直される。
#[test]
fn moving_a_tag_page_into_a_folder_renames_its_tag_after_the_folder() {
    let notes = Core::new();
    let old = "moved-tag";
    let new = format!("sub/{old}");
    let tagged = notes.new_doc_page("moved-tag-page");
    notes.set_tags(&tagged, old);
    let tag_page = notes.doc_page_named(old).unwrap();
    let tags_dir = notes.core.journal.config().config.tags_dir;

    notes
        .core
        .journal
        .update_page(
            &tag_page,
            None,
            Some(&format!("{tags_dir}sub/")),
            false,
            &by_the_client(),
        )
        .unwrap();

    assert_eq!(notes.page_properties(&tagged)["tags"], json!([new]));
}

// タグページをタグが変わらないように更新しても、タグの付いたページは触られない。
#[test]
fn updating_a_tag_page_without_changing_its_tag_leaves_the_tagged_pages_alone() {
    let notes = Core::new();
    let tag = "same-tag";
    let tagged = notes.new_doc_page("same-tag-page");
    notes.set_tags(&tagged, tag);
    let tag_page = notes.doc_page_named(tag).unwrap();
    let before = notes.page(&tagged)["page"]["modifiedDate"].clone();
    notes.clock.advance(5);

    notes.retitle(&tag_page, tag).unwrap();

    assert_eq!(notes.page(&tagged)["page"]["modifiedDate"], before);
}

// タグページには、タグでないタイトルを付けられない。
#[test]
fn a_tag_page_cannot_take_a_title_that_is_not_a_tag() {
    let notes = Core::new();
    let tag = "untitled-tag";
    let page = notes.new_doc_page("untitled-tag-page");
    notes.set_tags(&page, tag);
    let tag_page = notes.doc_page_named(tag).unwrap();

    let renamed = notes.retitle(&tag_page, "two words");

    assert!(is_bad_request(&renamed), "{renamed:?}");
    assert_eq!(notes.doc_page_named(tag), Some(tag_page));
}

// ページを削除すると、そのタグのタグページも消える。
#[test]
fn a_deleted_page_takes_the_tag_pages_of_its_tags_with_it() {
    let notes = Core::new();
    let page_tag = "deleted-page-tag-pages-page";
    let body_tag = "deleted-page-tag-pages-body";
    let page = notes.new_doc_page("deleted-page-tag-pages");
    notes.set_tags(&page, page_tag);
    let item = notes.first_item(&page);
    notes.update_item(
        item["id"].as_str().unwrap(),
        json!({ "body": format!("#{body_tag}") }),
    );
    assert!(notes.doc_page_named(page_tag).is_some());
    assert!(notes.doc_page_named(body_tag).is_some());

    notes
        .core
        .journal
        .delete_page(&page, &by_the_client())
        .unwrap();
    assert!(notes.doc_page_named(page_tag).is_none());
    assert!(notes.doc_page_named(body_tag).is_none());
}

// 外部で編集したファイルのタグにも、タグページが保たれる。
#[test]
fn the_tags_of_a_file_edited_outside_keep_their_tag_pages() {
    let notes = Core::new();
    let page_tag = "file-tag-pages-page";
    let body_tag = "file-tag-pages-body";
    let page = notes.new_doc_page("file-tag-pages");
    notes.fill(&page, &["before the edit"]);

    notes.edit_file_and_scan("file-tag-pages", |text| {
        text.replacen(
            "---\n\n",
            &format!("tags:\n  - \"#{page_tag}\"\n---\n\n"),
            1,
        )
        .replace("before the edit", &format!("see #{body_tag}"))
    });
    assert_eq!(notes.page_properties(&page)["tags"], json!([page_tag]));
    assert!(notes.doc_page_named(page_tag).is_some());
    assert!(notes.doc_page_named(body_tag).is_some());

    notes.edit_file_and_scan("file-tag-pages", |text| {
        text.lines()
            .filter(|l| !l.contains(page_tag) && *l != "tags:")
            .map(|l| {
                if l.contains(body_tag) {
                    "no tag now\n".to_string()
                } else {
                    format!("{l}\n")
                }
            })
            .collect()
    });
    let properties = notes.page_properties(&page);
    assert!(properties.get("tags").is_none(), "{properties}");
    assert!(notes.doc_page_named(page_tag).is_none());
    assert!(notes.doc_page_named(body_tag).is_none());
}

/// A page with one item whose `item_list` is `a,b` in the file
fn listed_page(notes: &Core, title: &str) -> String {
    notes.define_key("item_list");
    let page = notes.new_doc_page(title);
    let item = notes.fill(&page, &["listed"]).remove(0);
    notes
        .set_item_property(
            item["id"].as_str().unwrap(),
            "item_list",
            json!({ "valueType": "list", "value": "a, b" }),
        )
        .unwrap();
    let text = notes.file_of_doc(title);
    assert!(text.contains("item_list=\"a,b\""), "{text}");
    page
}

// ファイルで直した項目のlist型の値は、項目に取り込まれる。
#[test]
fn a_list_of_an_item_edited_in_the_file_is_merged_into_the_item() {
    let notes = Core::new();
    let page = listed_page(&notes, "file-item-list");

    notes.edit_file_and_scan("file-item-list", |text| {
        text.replace("item_list=\"a,b\"", "item_list=\"b,c\"")
    });

    assert_eq!(
        notes.items(&page)[0]["properties"]["item_list"],
        json!(["b", "c"])
    );
}

// ファイルから消した項目のlist型のキーは、項目からも消える。
#[test]
fn a_list_of_an_item_removed_from_the_file_goes_from_the_item() {
    let notes = Core::new();
    let page = listed_page(&notes, "file-item-list-gone");

    notes.edit_file_and_scan("file-item-list-gone", |text| {
        text.replace(" item_list=\"a,b\"", "")
    });

    let properties = notes.items(&page)[0]["properties"].clone();
    assert!(properties.get("item_list").is_none(), "{properties}");
}

// 古いtagキーで書かれたファイルのタグは失われない。
#[test]
fn a_file_written_with_the_old_tag_key_keeps_its_tags() {
    let notes = Core::new();
    let page = notes.new_doc_page("file-old-tag-key");
    notes.fill(&page, &["before the edit"]);

    notes.edit_file_and_scan("file-old-tag-key", |text| {
        text.replacen("---\n\n", "tag:\n  - old-key\n---\n\n", 1)
            .replace("before the edit", "after the edit")
    });

    assert_eq!(notes.page_properties(&page)["tags"], json!(["old-key"]));
    let text = notes.file_of_doc("file-old-tag-key");
    assert!(text.contains("tags:\n  - old-key\n"), "{text}");
}

// ファイルの時刻がページの最後の変更より少し前でも、前回の同期のあとにページが変わっていなければ、ファイルのタグが取り込まれる。ファイルの時刻の刻みは、ページの日付の時計より粗いことがある（#730）。
#[test]
fn the_tags_of_a_file_are_taken_in_when_its_coarse_time_is_just_before_the_page_changed() {
    let notes = Core::new();
    let page = notes.new_doc_page("file-coarse-time");
    notes.fill(&page, &["before the edit"]);
    let changed = notes.page(&page)["page"]["modifiedDate"].as_i64().unwrap();
    // Without `modified:` in it, the file is as new as its time says, as an editor saves it
    let text: String = notes
        .file_of_doc("file-coarse-time")
        .lines()
        .filter(|l| !l.starts_with("modified: "))
        .map(|l| format!("{l}\n"))
        .collect();
    let path = notes.markdown_path("file-coarse-time");
    std::fs::write(
        &path,
        text.replacen("---\n\n", "tags:\n  - from-file\n---\n\n", 1),
    )
    .unwrap();
    let just_before = std::time::UNIX_EPOCH + std::time::Duration::from_millis(changed as u64 - 1);
    let file = std::fs::File::options().write(true).open(&path).unwrap();
    file.set_modified(just_before).unwrap();
    drop(file);

    notes.core.reconcile.scan();

    assert_eq!(notes.page_properties(&page)["tags"], json!(["from-file"]));
}

// ページの日付はプロパティとして設定でき、削除はできない。
#[test]
fn the_dates_of_a_page_are_set_as_properties_and_cannot_be_removed() {
    let notes = Core::new();
    let page = notes.new_doc_page("page-dates");

    let resp = answer(
        notes
            .put_page_property(&page, "created", "datetime", "2020-01-02T03:04:05Z")
            .unwrap(),
    );
    assert_eq!(resp["page"]["createdDate"], json!(1_577_934_245_000i64));

    let resp = answer(
        notes
            .put_page_property(&page, "modified", "datetime", "2021-02-03T04:05:06Z")
            .unwrap(),
    );
    assert_eq!(resp["page"]["modifiedDate"], json!(1_612_325_106_000i64));

    let bad = notes.put_page_property(&page, "created", "datetime", "yesterday");
    assert!(is_bad_request(&bad), "{bad:?}");
    let removed = notes.remove_page_property(&page, "created");
    assert!(is_bad_request(&removed), "{removed:?}");

    let text = notes.file_of_doc("page-dates");
    assert!(text.contains("created: 2020-01-02T03:04:05Z\n"), "{text}");
}

/// A page in a folder of its own, and the index page of that folder
fn page_under_an_index(notes: &Core, dir_title: &str) -> (String, String) {
    let dir = format!("{}/", dir_title);
    notes.new_doc_page(dir_title);
    let made = notes.open_doc_in("a-page", &dir);
    let page_id = made["page"]["id"].as_str().unwrap().to_string();
    let parent_id = made["page"]["parentId"].as_str().unwrap().to_string();
    (page_id, parent_id)
}

/// The page among the pages of an answer
fn page_in(pages: &Value, page_id: &str) -> Value {
    pages["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == page_id)
        .unwrap()
        .clone()
}

// indexページの子ページは、スカラーのプロパティを持って返る。
#[test]
fn the_children_of_an_index_page_carry_their_scalar_properties() {
    let notes = Core::new();
    let (page_id, parent_id) = page_under_an_index(&notes, "children-props");
    notes
        .put_page_property(&page_id, "published", "datetime", "2026-10-20T09:00:00Z")
        .unwrap();

    let children = answer(notes.core.journal.child_pages(&parent_id).unwrap());
    assert_eq!(
        page_in(&children, &page_id)["properties"]["published"],
        json!("2026-10-20T09:00:00Z")
    );

    let recent = answer(notes.core.journal.recent_pages(Some(50), None).unwrap());
    assert_eq!(
        page_in(&recent, &page_id)["properties"]["published"],
        json!("2026-10-20T09:00:00Z")
    );
}

// indexページの子ページは、タグを持って返る。他のリストのプロパティは返らない。アトラスがカードのアイコンに使うのはタグだけである。
#[test]
fn the_children_of_an_index_page_carry_their_tags_but_no_other_list() {
    let notes = Core::new();
    notes.define_key("children_list");
    let (page_id, parent_id) = page_under_an_index(&notes, "children-tags");
    notes.set_tags(&page_id, "children-tag-b, children-tag-a");
    notes
        .put_page_property(&page_id, "children_list", "list", "x")
        .unwrap();

    let children = answer(notes.core.journal.child_pages(&parent_id).unwrap());
    let child = page_in(&children, &page_id);
    assert_eq!(
        child["properties"]["tags"],
        json!(["children-tag-b", "children-tag-a"])
    );
    assert!(
        child["properties"].get("children_list").is_none(),
        "{}",
        child["properties"]
    );
}

// プロパティ定義を削除すると、ページと項目からその値が消える。
#[test]
fn deleting_a_property_definition_removes_its_values_from_pages_and_items() {
    let notes = Core::new();
    notes.define_key("rating_n");
    let page = notes.new_doc_page("definition-delete");
    let item = notes.fill(&page, &["rated"]).remove(0);
    notes
        .set_item_property(
            item["id"].as_str().unwrap(),
            "rating_n",
            json!({ "valueType": "number", "value": "5" }),
        )
        .unwrap();
    let resp = answer(
        notes
            .put_page_property(&page, "rating_n", "number", "3")
            .unwrap(),
    );
    assert_eq!(resp["page"]["properties"]["rating_n"], json!("3"));
    let modified_before = resp["page"]["modifiedDate"].as_i64().unwrap();

    let resp = definitions(notes.delete_definition("rating_n").unwrap());
    assert!(definition(&resp, "rating_n").is_none(), "{resp}");

    assert!(notes.items(&page)[0]["properties"]
        .get("rating_n")
        .is_none());
    let got = notes.page(&page);
    assert!(got["page"]["properties"].get("rating_n").is_none(), "{got}");
    assert!(got["page"]["modifiedDate"].as_i64().unwrap() >= modified_before);

    // System keys and unknown keys are refused
    let system = notes.delete_definition("tags");
    assert!(is_bad_request(&system), "{system:?}");
    let unknown = notes.delete_definition("rating_n");
    assert!(is_bad_request(&unknown), "{unknown:?}");
}
