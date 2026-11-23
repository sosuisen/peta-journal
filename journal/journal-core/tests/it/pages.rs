//! Pages: making and finding them, renaming and moving them, the index pages of folders,
//! and the pages that link to them.

use journal_core::dto::ItemFields;
use journal_core::error::AppError;
use journal_core::journal::{NarrowPage, PageAnswer};
use serde_json::{json, Value};

use crate::common::items::{answer, field_of_each, id, input};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Sets a property of the page, as the client does, and gives the page of the answer
    fn set_page_property(
        &self,
        page_id: &str,
        key: &str,
        value_type: &str,
        value: &str,
    ) -> serde_json::Value {
        let changed = self
            .core
            .journal
            .set_page_property(
                page_id,
                key,
                Some(value_type),
                Some(value),
                &by_the_client(),
            )
            .unwrap();
        answer(changed)["page"].clone()
    }

    /// Gives the page a modified date in the past, so a later change shows as a new date.
    fn date_the_page_in_the_past(&self, page_id: &str) -> i64 {
        let page = self.set_page_property(page_id, "modified", "datetime", "2021-02-03T04:05:06Z");
        page["modifiedDate"].as_i64().unwrap()
    }

    /// Gives the page another title or folder, as the client does
    fn rename(
        &self,
        page_id: &str,
        title: Option<&str>,
        dir_path: Option<&str>,
        overwrite: bool,
    ) -> Result<PageAnswer, AppError> {
        self.core
            .journal
            .update_page(page_id, title, dir_path, overwrite, &by_the_client())
    }

    /// Demotes an index page the way the app does
    fn demote(&self, page_id: &str, overwrite: bool) -> Result<PageAnswer, AppError> {
        self.core
            .journal
            .demote_page(page_id, overwrite, &by_the_client())
    }

    /// Whether the page is there
    fn is_there(&self, page_id: &str) -> bool {
        match self.core.journal.get_page(page_id) {
            Ok(_) => true,
            Err(AppError::NotFound(_)) => false,
            Err(e) => panic!("{e}"),
        }
    }
}

/// The page of an answer
fn page_of(changed: Result<PageAnswer, AppError>) -> Value {
    answer(changed.unwrap())["page"].clone()
}

// ページにプロパティを設定すると、ページの変更日が新しくなる。
#[test]
fn setting_a_property_counts_as_a_change_of_the_page() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-property-date");
    let old = notes.date_the_page_in_the_past(&page);

    let changed = notes.set_page_property(&page, "tags", "tags", "dated");

    assert_ne!(changed["modifiedDate"], json!(old));
}

// 新しいdocページは空の項目1つで始まり、同じタイトルで再び見つかる。
#[test]
fn a_new_doc_page_starts_with_one_empty_item_and_is_found_again_by_its_title() {
    let notes = Core::new();

    let first = notes.open_doc_in("pages-new", "");
    let again = notes.open_doc_in("pages-new", "");

    assert_eq!(first["created"], json!(true));
    assert_eq!(field_of_each(&first, "items", "body"), vec![json!("")]);
    assert_eq!(first["page"]["type"], json!("doc"));
    assert_eq!(again["created"], json!(false));
    assert_eq!(first["page"]["id"], again["page"]["id"]);
}

// ページを探すだけの要求は、あるdocページとジャーナルを答え、ないページは作らずに、ないと答える。戻る・進むが、同期で消えたページを作らないためである。
#[test]
fn finding_a_page_answers_the_page_that_is_there_and_makes_none() {
    let notes = Core::new();
    let doc = notes.open_doc_in("pages-find", "d/");
    let day = notes.open_journal("2026-11-02");
    let find = |query: Value| answer(notes.core.journal.find_page(input(query)).unwrap());

    let found_doc = find(json!({ "title": "pages-find", "dirPath": "d/" }));
    let found_day = find(json!({ "journal": "2026-11-02" }));
    let no_doc = find(json!({ "title": "pages-gone", "dirPath": "d/" }));
    let no_day = find(json!({ "journal": "2026-11-03" }));

    assert_eq!(found_doc["page"]["id"], doc["page"]["id"]);
    assert_eq!(found_day["page"]["id"], day["page"]["id"]);
    assert_eq!(no_doc["page"], Value::Null);
    assert_eq!(no_day["page"], Value::Null);
    let again = notes.open_doc_in("pages-gone", "d/");
    assert_eq!(again["created"], json!(true), "finding made no page");
}

/// Titles no page may take: blank, padded with spaces, and not a file name on every OS.
const BAD_TITLES: [&str; 5] = ["", "  ", " padded", "padded ", "a:b"];

// 空、空白だけ、前後に空白がある、どのOSでもファイル名になるとは限らないタイトルでは、ページを作れない。
#[test]
fn a_page_is_not_made_with_a_title_no_page_may_take() {
    let notes = Core::new();

    for title in BAD_TITLES {
        let made = notes.core.journal.get_or_create_doc(Some(title), Some(""));
        assert!(
            matches!(made, Err(AppError::BadRequest(_))),
            "{title:?}: {made:?}"
        );
    }
}

// ページを作れないタイトルには、改名もできない。
#[test]
fn a_page_is_not_renamed_to_a_title_no_page_may_take() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-bad-rename");

    for title in BAD_TITLES {
        let renamed = notes.rename(&page, Some(title), None, false);
        assert!(
            matches!(renamed, Err(AppError::BadRequest(_))),
            "{title:?}: {renamed:?}"
        );
    }
    assert_eq!(notes.page(&page)["page"]["name"], json!("pages-bad-rename"));
}

// ページの取得は、ページとその項目を順番どおりに返す。
#[test]
fn get_page_returns_the_page_with_its_items_in_order() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-get");
    notes.fill(&page, &["one", "  two", "three"]);

    let got = notes.page(&page);

    assert_eq!(got["page"]["name"], json!("pages-get"));
    assert_eq!(
        field_of_each(&got, "items", "body"),
        vec![json!("one"), json!("two"), json!("three")]
    );
    assert_eq!(
        field_of_each(&got, "items", "level"),
        vec![json!(1), json!(2), json!(1)]
    );
}

// ページと項目の答えは、符号化したURLを持つ。項目のURLは、ページのURLの後ろに#と項目のidを付けたものである。
#[test]
fn a_page_and_its_items_are_answered_with_their_encoded_urls() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-url C#");
    let rows = notes.fill(&page, &["one"]);

    let got = notes.page(&page);

    assert_eq!(got["page"]["url"], json!("/pages/pages-url%20C%23"));
    assert_eq!(
        got["items"][0]["url"],
        json!(format!("/pages/pages-url%20C%23#{}", id(&rows[0])))
    );
}

// ページの取得は、fieldsで頼まれたフィールドだけを各項目に持つ。トランスクルードの木の項目も同じである。ページの情報はそのまま返る。
#[test]
fn get_page_returns_its_items_with_only_the_fields_asked_for() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("pages-get-fields-source");
    let source = id(&notes.fill(&source_page, &["source"])[0]);
    let page = notes.new_doc_page("pages-get-fields");
    let rows = notes.fill(&page, &["one", "  host"]);
    notes.transclude(&id(&rows[1]), &source);

    let fields = ItemFields::parse(Some("id,level")).unwrap();
    let got = answer(
        notes
            .core
            .journal
            .get_page(&page)
            .unwrap()
            .with_fields(&fields),
    );

    assert_eq!(got["page"]["name"], json!("pages-get-fields"));
    assert_eq!(
        got["items"],
        json!([
            { "id": id(&rows[0]), "level": 1 },
            { "id": id(&rows[1]), "level": 2 },
        ])
    );
    assert_eq!(
        got["transcludedTrees"],
        json!({ source.as_str(): [{ "id": source, "level": 1 }] })
    );
}

// ページの改名で、タイトルとパスが変わる。
#[test]
fn renaming_a_page_changes_its_title_and_path() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-rename");
    let old = notes.date_the_page_in_the_past(&page);

    let renamed = page_of(notes.rename(&page, Some("pages-rename-renamed"), Some("notes/"), false));

    assert_eq!(renamed["name"], json!("pages-rename-renamed"));
    assert_eq!(renamed["dirPath"], json!("notes/"));
    assert_ne!(renamed["modifiedDate"], json!(old));
    assert_eq!(
        notes.page(&page)["page"]["name"],
        json!("pages-rename-renamed")
    );
}

// NFDのタイトルとフォルダで開いても、ページはNFCの名前になり、NFCで開いたときと同じページである。
#[test]
fn a_doc_opened_with_an_nfd_title_and_folder_has_nfc_names_and_is_the_same_page() {
    let notes = Core::new();

    let first = notes.open_doc_in("\u{30ab}\u{3099}", "\u{30c6}\u{3099}/");
    let again = notes.open_doc_in("\u{30ac}", "\u{30c7}/");

    assert_eq!(first["page"]["name"], json!("\u{30ac}"));
    assert_eq!(first["page"]["dirPath"], json!("\u{30c7}/"));
    assert_eq!(again["created"], json!(false));
    assert_eq!(first["page"]["id"], again["page"]["id"]);
}

// NFDのタイトルとフォルダへ改名すると、ページはNFCの名前になる。
#[test]
fn renaming_a_page_to_an_nfd_title_and_folder_gives_nfc_names() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-rename-nfd");

    let renamed = page_of(notes.rename(
        &page,
        Some("\u{30ab}\u{3099}"),
        Some("\u{30c6}\u{3099}/"),
        false,
    ));

    assert_eq!(renamed["name"], json!("\u{30ac}"));
    assert_eq!(renamed["dirPath"], json!("\u{30c7}/"));
}

// ページの下にページを作ると、そのページはディレクトリのindexページになり、変更日が新しくなる。
#[test]
fn a_page_becomes_the_index_page_of_its_directory_when_a_page_is_made_under_it() {
    let notes = Core::new();
    let root = notes.new_doc_page("pages-promote");
    let old = notes.date_the_page_in_the_past(&root);

    notes.open_doc_in("child", "pages-promote/");

    let promoted = notes.page(&root);
    assert_eq!(promoted["page"]["name"], json!("index"));
    assert_eq!(promoted["page"]["dirPath"], json!("pages-promote/"));
    assert_ne!(promoted["page"]["modifiedDate"], json!(old));
}

// 子ページを持つindexページは、普通のページに改名できず、削除もできない。
#[test]
fn an_index_page_with_children_is_neither_renamed_to_a_plain_page_nor_deleted() {
    let notes = Core::new();
    let root = notes.new_doc_page("pages-index-guard");
    notes.open_doc_in("child", "pages-index-guard/");

    let renamed = notes.rename(&root, Some("plain"), None, false);
    assert!(
        matches!(renamed, Err(AppError::BadRequest(_))),
        "{renamed:?}"
    );
    let deleted = notes.core.journal.delete_page(&root, &by_the_client());
    assert!(
        matches!(deleted, Err(AppError::BadRequest(_))),
        "{deleted:?}"
    );
}

// 降格は、1つの要求で、アトラスページを消し、indexページをディレクトリの名前のページに変える。
#[test]
fn demoting_an_index_page_deletes_its_atlas_and_renames_it_in_one_request() {
    let notes = Core::new();
    let root = notes.new_doc_page("pages-demote");
    let atlas_id = notes.atlas_of("pages-demote/");
    assert_eq!(notes.page(&root)["page"]["name"], json!("index"));

    let demoted = page_of(notes.demote(&root, false));

    assert_eq!(demoted["id"], json!(root));
    assert_eq!(demoted["name"], json!("pages-demote"));
    assert_eq!(demoted["dirPath"], json!(""));
    assert!(!notes.is_there(&atlas_id));
}

// 入れ子のディレクトリのindexページは、親のディレクトリの中の、ディレクトリの名前のページになる。
#[test]
fn demoting_a_nested_index_page_puts_it_in_the_directory_above() {
    let notes = Core::new();
    notes.new_doc_page("pages-demote-nested");
    let dir = "pages-demote-nested/";
    let sub_id = notes.open_doc_in("sub", dir)["page"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    notes.atlas_of(&format!("{dir}sub/"));

    let demoted = page_of(notes.demote(&sub_id, false));

    assert_eq!(demoted["name"], json!("sub"));
    assert_eq!(demoted["dirPath"], json!(dir));
}

// アトラス以外の子ページがあるindexページの降格は断られ、アトラスも残る。
#[test]
fn demoting_an_index_page_with_a_child_page_is_refused_and_keeps_the_atlas() {
    let notes = Core::new();
    let root = notes.new_doc_page("pages-demote-child");
    let dir = "pages-demote-child/";
    let atlas_id = notes.atlas_of(dir);
    notes.open_doc_in("child", dir);

    let demoted = notes.demote(&root, false);
    assert!(
        matches!(demoted, Err(AppError::BadRequest(_))),
        "{demoted:?}"
    );

    assert!(notes.is_there(&atlas_id));
    assert_eq!(notes.page(&root)["page"]["name"], json!("index"));
}

// 降格先の名前を別のページが持っていると、降格は断られ、アトラスも残る。上書きを指定すると、そのページを消して降格する。
#[test]
fn demoting_to_the_name_of_another_page_is_refused_unless_it_is_overwritten() {
    let notes = Core::new();
    let root = notes.new_doc_page("pages-demote-taken");
    let atlas_id = notes.atlas_of("pages-demote-taken/");
    let other = notes.new_doc_page("pages-demote-taker");
    notes
        .rename(&other, Some("pages-demote-taken"), None, false)
        .unwrap();

    let refused = notes.demote(&root, false);
    assert!(matches!(refused, Err(AppError::Conflict(_))), "{refused:?}");
    assert!(notes.is_there(&atlas_id));
    assert!(notes.is_there(&other));

    let demoted = page_of(notes.demote(&root, true));
    assert_eq!(demoted["name"], json!("pages-demote-taken"));
    assert!(!notes.is_there(&atlas_id));
    assert!(!notes.is_there(&other));
}

// indexページでないページは、ルートにあってもディレクトリの中にあっても降格できない。ルートのindexページも降格できない。
#[test]
fn a_page_that_is_not_an_index_page_of_a_directory_is_not_demoted() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-demote-plain");

    let plain = notes.demote(&page, false);
    assert!(matches!(plain, Err(AppError::BadRequest(_))), "{plain:?}");

    let root = notes.open_doc_in("index", "");
    let top = notes.demote(root["page"]["id"].as_str().unwrap(), false);
    assert!(matches!(top, Err(AppError::BadRequest(_))), "{top:?}");

    // A page in a directory that is not its index page keeps its name
    let child = notes.open_doc_in("child", "pages-demote-plain/");
    let child_id = child["page"]["id"].as_str().unwrap();
    let in_dir = notes.demote(child_id, false);
    assert!(matches!(in_dir, Err(AppError::BadRequest(_))), "{in_dir:?}");
    assert_eq!(notes.page(child_id)["page"]["name"], json!("child"));
}

// indexページを動かすと、その下のページも一緒に動く。
#[test]
fn moving_an_index_page_moves_the_pages_under_it() {
    let notes = Core::new();
    // Making a page under it turns this page into the index page of its directory
    let root = notes.new_doc_page("pages-move-dir");
    let deep = notes.open_doc_in("deep", "pages-move-dir/sub/");
    let deep_id = deep["page"]["id"].as_str().unwrap();

    notes
        .rename(&root, Some("index"), Some("pages-move-dir-moved/"), false)
        .unwrap();

    assert_eq!(
        notes.page(deep_id)["page"]["dirPath"],
        json!("pages-move-dir-moved/sub/")
    );
}

// 別のページと同じ名前への改名は、上書きを指定しない限り拒否される。
#[test]
fn renaming_to_the_name_of_another_page_is_refused_unless_it_is_overwritten() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-overwrite");
    let other = notes.new_doc_page("pages-overwritten");
    notes.fill(&page, &["mine"]);
    notes.fill(&other, &["theirs"]);

    let refused = notes.rename(&page, Some("pages-overwritten"), None, false);
    assert!(matches!(refused, Err(AppError::Conflict(_))), "{refused:?}");
    assert!(notes.is_there(&other));

    let renamed = page_of(notes.rename(&page, Some("PAGES-OVERWRITTEN"), None, true));
    assert_eq!(renamed["name"], json!("PAGES-OVERWRITTEN"));
    assert!(!notes.is_there(&other));
    assert!(notes
        .outline_of(&page)
        .iter()
        .any(|line| line.ends_with(":mine")));
}

// 最近のページでは、最後に触ったページが先頭に来る。
#[test]
fn the_most_recently_touched_page_comes_first_in_recent_pages() {
    let notes = Core::new();
    let older = notes.new_doc_page("pages-recent-older");
    notes.clock.advance(5);
    let newer = notes.new_doc_page("pages-recent-newer");

    let recent = answer(
        notes
            .core
            .journal
            .recent_pages(Some(2000), Some("doc"))
            .unwrap(),
    );

    // The pages the core made when it started come after the two
    assert_eq!(
        field_of_each(&recent, "pages", "id")[..2],
        [json!(newer), json!(older)]
    );
}

// 削除したページは見つからなくなる。
#[test]
fn a_deleted_page_is_gone() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-delete");

    notes
        .core
        .journal
        .delete_page(&page, &by_the_client())
        .unwrap();

    assert!(!notes.is_there(&page));
}

// ページを削除すると、その項目もIDで取れなくなり、データベースにも残らない。
#[test]
fn the_items_of_a_deleted_page_are_gone_too() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-delete-items");
    let item = id(&notes.fill(&page, &["on the page"])[0]);

    notes
        .core
        .journal
        .delete_page(&page, &by_the_client())
        .unwrap();

    assert!(notes.core.journal.get_item(&item).is_err());
    // No client can see rows left behind, as every read joins the page, but they would fill
    // the database
    let left: i64 = notes
        .core
        .db
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM items WHERE page_id = ?",
            [&page],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(left, 0);
}

// ページへのウィキリンクは、そのページのバックリンクに現れる。
#[test]
fn a_wiki_link_to_a_page_shows_up_in_its_backlinks() {
    let notes = Core::new();
    let target = notes.new_doc_page("pages-target");
    let source = notes.new_doc_page("pages-source");
    notes.fill(&source, &["see [[pages-target]]"]);

    let backlinks = answer(notes.core.journal.backlinks(&target).unwrap());

    assert_eq!(
        backlinks["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["sourcePage"]["id"].clone())
            .collect::<Vec<_>>(),
        vec![json!(source)]
    );
    assert_eq!(
        backlinks["entries"][0]["sourceItem"]["body"],
        json!("see [[pages-target]]")
    );
}

// タグページのバックリンクには、そのタグを使うページと項目が並ぶ。
#[test]
fn a_tag_page_lists_the_pages_and_items_that_use_its_tag() {
    let notes = Core::new();
    let tag = "backlink-tag";
    let tag_path = format!("{}{tag}", notes.core.journal.config().config.tags_dir);
    let by_page_tag = notes.new_doc_page("backlinks-by-page-tag");
    notes.set_tags(&by_page_tag, tag);
    let target = notes.doc_page_named(tag).unwrap();
    let by_items = notes.new_doc_page("backlinks-by-items");
    let rows = notes.fill(
        &by_items,
        &[
            &format!("a hashtag #{}", tag.to_uppercase()),
            "no link here",
            &format!("a wikilink [[{tag_path}#20260426035700_QZ998PX|label]]"),
            &format!("a longer path [[{tag_path}/more]]"),
        ],
    );

    let backlinks = answer(notes.core.journal.backlinks(&target).unwrap());
    let entries: Vec<(Value, Value)> = backlinks["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["sourcePage"]["id"].clone(), e["sourceItem"]["id"].clone()))
        .collect();
    // The newest source page first; its items in page order
    assert_eq!(
        entries,
        vec![
            (json!(by_items), json!(id(&rows[0]))),
            (json!(by_items), json!(id(&rows[2]))),
            (json!(by_page_tag), Value::Null),
        ]
    );
}
