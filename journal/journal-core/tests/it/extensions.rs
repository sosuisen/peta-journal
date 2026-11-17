//! Extension functions (ADR 009): a page made through the journal gets the items its
//! extensions ask for. Each test makes a core with all the extensions here installed; each
//! extension acts only on pages whose name starts with its own prefix.

use std::path::Path;

use journal_core::infrastructure::export::import_service::Outcome;
use journal_core::journal::MoveToJournalInput;
use serde_json::{json, Value};

use crate::common::extensions::by_the_client;
use crate::common::Core;

/// Installs an extension function: a folder with a `config.json` and a `main.js`.
fn install(config_dir: &Path, name: &str, main_js: &str) {
    let dir = config_dir.join("functions").join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.json"), json!({ "name": name }).to_string()).unwrap();
    std::fs::write(dir.join("main.js"), main_js).unwrap();
}

/// A core with the extensions of these tests.
fn ext_core() -> Core {
    Core::with_extensions(|config_dir| {
        install(
            config_dir,
            "a-basic",
            "globalThis.onPageCreate = (page) => {
                if (!page.fileName.startsWith('basic-')) return;
                const first = peta.createItem({ position: 'last', level: 1, body: 'A1' });
                peta.createItem({ position: 'after:' + first, level: 2, body: 'A2' });
            };",
        );
        install(
            config_dir,
            "b-second",
            "globalThis.onPageCreate = (page) => {
                if (!page.fileName.startsWith('basic-')) return;
                peta.createItem({ position: 'last', level: 1, body: 'B1' });
            };",
        );
        // Its second item goes after an item that does not exist, which fails when written
        install(
            config_dir,
            "c-broken",
            "globalThis.onPageCreate = (page) => {
                if (!page.fileName.startsWith('broken-')) return;
                peta.createItem({ position: 'last', level: 1, body: 'C1' });
                peta.createItem({ position: 'after:2001-01-01-00-00-00-AAAAAAA', level: 1, body: 'C2' });
            };",
        );
        install(
            config_dir,
            "d-after-broken",
            "globalThis.onPageCreate = (page) => {
                if (!page.fileName.startsWith('broken-')) return;
                peta.createItem({ position: 'last', level: 1, body: 'D1' });
            };",
        );
        install(
            config_dir,
            "e-deep",
            "globalThis.onPageCreate = (page) => {
                if (!page.fileName.startsWith('deep-')) return;
                peta.createItem({ position: 'last', level: 1, body: 'top' });
                peta.createItem({ position: 'last', level: 4, body: 'too deep' });
            };",
        );
        // The page name after the prefix is the id of the item to transclude
        install(
            config_dir,
            "f-transclude",
            "globalThis.onPageCreate = (page) => {
                if (!page.fileName.startsWith('tr-')) return;
                peta.createItem({ position: 'last', level: 1, body: '', transclude: page.fileName.slice(3) });
            };",
        );
        install(
            config_dir,
            "g-tag",
            "globalThis.onPageTagAdd = (page, tags) => {
                if (!tags.includes('tmpl-ext')) return;
                peta.createItem({ position: 'last', level: 1, body: 'T ' + tags.join(',') });
            };",
        );
        install(
            config_dir,
            "h-confirm",
            "globalThis.onPageTagAdd = (page, tags) => {
                if (!tags.includes('ask-ext')) return;
                if (peta.confirm('Insert into ' + page.fileName + '?')) {
                    peta.createItem({ position: 'last', level: 1, body: 'confirmed' });
                }
            };",
        );
        install(
            config_dir,
            "j-journal",
            "globalThis.onPageCreate = (page) => {
                if (page.pageType !== 'journal' || !page.fileName.startsWith('2031-')) return;
                peta.createItem({ position: 'last', level: 1, body: 'J ' + page.fileName });
            };",
        );
        // The old form, which ran in an iframe of the frontend: no main.js
        let old = config_dir.join("functions").join("k-iframe");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(
            old.join("config.json"),
            json!({ "name": "k-iframe" }).to_string(),
        )
        .unwrap();
        std::fs::write(old.join("index.html"), "<script></script>").unwrap();
    })
}

// main.jsのない古い形の拡張は、拡張の一覧に入らない。
#[test]
fn an_extension_without_main_js_is_not_listed() {
    let notes = ext_core();

    let names: Vec<&str> = notes
        .core
        .functions
        .list_functions()
        .iter()
        .map(|f| f.dir_name.as_str())
        .collect();

    assert!(names.contains(&"a-basic"), "{names:?}");
    assert!(!names.contains(&"k-iframe"), "{names:?}");
}

fn bodies(items: &Value) -> Vec<(i64, String)> {
    items
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["level"].as_i64().unwrap(),
                item["body"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

// 新しいドキュメントページは、拡張が頼んだ項目を持って返り、空の1項目は残らない。
#[test]
fn a_new_doc_page_comes_with_the_items_its_extension_asked_for() {
    let notes = ext_core();

    let created = notes.open_doc("basic-doc");

    assert_eq!(
        bodies(&created["items"]),
        vec![
            (1, "A1".to_string()),
            (2, "A2".to_string()),
            (1, "B1".to_string())
        ]
    );
}

// 既にあるページを開いても、拡張は呼ばれない。
#[test]
fn opening_a_page_that_exists_does_not_call_the_extensions() {
    let notes = ext_core();
    notes.open_doc("basic-again");

    let again = notes.open_doc("basic-again");

    assert_eq!(again["created"], false);
    assert_eq!(bodies(&again["items"]).len(), 3);
}

// 置けない階層は、置ける深さまで下げる。
#[test]
fn a_level_too_deep_for_its_place_is_lowered_until_it_fits() {
    let notes = ext_core();

    let created = notes.open_doc("deep-doc");

    assert_eq!(
        bodies(&created["items"]),
        vec![(1, "top".to_string()), (2, "too deep".to_string())]
    );
}

// トランスクルード元を指定した項目は、その項目を写す。
#[test]
fn an_item_with_a_transclusion_source_mirrors_that_item() {
    let notes = ext_core();
    let source_page = notes.new_doc_page("source");
    let source = notes.fill(&source_page, &["source text"]).remove(0);
    let source_id = source["id"].as_str().unwrap();

    let created = notes.open_doc(&format!("tr-{source_id}"));

    let item = &created["items"][0];
    assert_eq!(item["transcludeItemId"], source_id);
    assert_eq!(item["body"], "source text");
    let source_page = notes.page(&source_page);
    assert_eq!(item["ast"], source_page["items"][0]["ast"]);
}

// タグを足すと、onPageTagAddが足されたタグだけを受け取り、その項目がページの最後に入る。
#[test]
fn adding_a_tag_puts_the_extension_items_at_the_end_of_the_page() {
    let notes = ext_core();
    let page = notes.new_doc_page("tagged");
    notes.fill(&page, &["existing"]);
    notes.set_tags(&page, "old");

    notes.set_tags(&page, "old, tmpl-ext");

    assert_eq!(
        notes.bodies_once(&page, 2),
        vec!["existing".to_string(), "T tmpl-ext".to_string()]
    );
}

// タグの拡張が項目を書き終えると、そのページの変更がSSEで知らされる。
#[test]
fn the_clients_hear_of_the_page_when_the_tag_extensions_are_done() {
    let notes = ext_core();
    let page = notes.new_doc_page("tag-event");

    notes.set_tags(&page, "tmpl-ext");

    notes.wait_for_the_extensions(&page);
}

// タグの拡張がconfirmで尋ねると、クライアントに問い合わせが届き、「はい」の答えで項目が入る。
#[test]
fn a_tag_extension_asks_the_clients_and_writes_its_items_after_a_yes() {
    let notes = ext_core();
    let title = "confirm";
    let page = notes.new_doc_page(title);
    notes.fill(&page, &["existing"]);

    notes.set_tags(&page, "ask-ext");
    let question = notes.question_with(&format!("Insert into {title}?"));
    notes.answer(&question, true);

    assert_eq!(question["type"], "confirm");
    assert_eq!(
        notes.bodies_once(&page, 2),
        vec!["existing".to_string(), "confirmed".to_string()]
    );
}

// フォルダ同期で取り込んだページでは、拡張は呼ばれない。
#[test]
fn a_page_taken_in_from_the_folder_does_not_call_the_extensions() {
    let notes = ext_core();
    let name = "basic-imported";
    let rel_path = format!("docs/{name}.md");
    std::fs::create_dir_all(notes.root.join("docs")).unwrap();
    std::fs::write(notes.root.join(&rel_path), "written by hand\n").unwrap();

    let outcome = notes.core.import.sync_file(&notes.root, &rel_path);

    assert_eq!(outcome, Outcome::Imported);
    let page = notes
        .core
        .pages
        .find_by_type_dir_path_file_name("doc", "", name)
        .unwrap()
        .unwrap();
    let bodies: Vec<String> = notes
        .items(page.id.as_str())
        .iter()
        .map(|i| i["body"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(bodies, vec!["written by hand".to_string()]);
}

// 失敗した拡張の項目は取り消され、ほかの拡張の項目は残る。
#[test]
fn a_failed_extension_writes_nothing_and_the_others_still_run() {
    let notes = ext_core();

    let created = notes.open_doc("broken-doc");

    assert_eq!(bodies(&created["items"]), vec![(1, "D1".to_string())]);
}

// 別の日への移動が作ったジャーナルにも、拡張が頼んだ項目が入る。動かした項目のあとに入り、答えは動かした項目だけを持つ。
#[test]
fn a_journal_made_by_a_move_gets_the_items_its_extension_asked_for_after_the_moved_ones() {
    let notes = ext_core();
    let from = notes.open_journal("2031-02-01");
    let from_id = from["page"]["id"].as_str().unwrap();
    let rows = notes.fill(from_id, &["go", "stay"]);

    let moved = notes
        .core
        .journal
        .move_items_to_journal(
            "2031-02-02",
            MoveToJournalInput {
                item_ids: vec![rows[0]["id"].as_str().unwrap().to_string()],
                position: "last".to_string(),
                body: None,
            },
            &by_the_client(),
        )
        .unwrap();
    let moved = serde_json::to_value(moved).unwrap();

    assert_eq!(moved["created"], json!(true));
    assert_eq!(bodies(&moved["items"]), vec![(1, "go".to_string())]);
    let page = notes.page(moved["page"]["id"].as_str().unwrap());
    assert_eq!(
        bodies(&page["items"]),
        vec![(1, "go".to_string()), (1, "J 2031-02-02".to_string())]
    );
}

// 新しいジャーナルも、拡張が頼んだ項目を持って返る。
#[test]
fn a_new_journal_comes_with_the_items_its_extension_asked_for() {
    let notes = ext_core();

    let created = notes.open_journal("2031-01-05");

    assert_eq!(
        bodies(&created["items"]),
        vec![(1, "J 2031-01-05".to_string())]
    );
}
