//! Extension functions on the server (ADR 009): a page made over the API gets the items its
//! extensions ask for.
//!
//! These tests are a binary of their own. Its server has extensions installed, and they would
//! put items into the pages of the other tests. Each extension here acts only on pages whose
//! name starts with its own prefix, so the tests do not see each other's items.

#[path = "../common/mod.rs"]
mod common;

use std::path::Path;

use common::*;
use journal_core::infrastructure::export::import_service::Outcome;
use serde_json::{json, Value};

/// Installs an extension function: a folder with a `config.json` and a `main.js`.
fn install(config_dir: &Path, name: &str, main_js: &str) {
    let dir = config_dir.join("functions").join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.json"), json!({ "name": name }).to_string()).unwrap();
    std::fs::write(dir.join("main.js"), main_js).unwrap();
}

/// The server with the extensions of these tests.
fn ext_server() -> &'static TestServer {
    server_with(|config_dir| {
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
#[tokio::test]
async fn an_extension_without_main_js_is_not_listed() {
    let server = ext_server();

    let names: Vec<&str> = server
        .state
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
#[tokio::test]
async fn a_new_doc_page_comes_with_the_items_its_extension_asked_for() {
    ext_server();
    let title = unique_name("basic-doc");

    let (status, created) = post("/api/pages/doc", json!({ "title": title, "dirPath": "" })).await;

    assert_eq!(status, 200);
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
#[tokio::test]
async fn opening_a_page_that_exists_does_not_call_the_extensions() {
    ext_server();
    let title = unique_name("basic-again");
    post("/api/pages/doc", json!({ "title": title, "dirPath": "" })).await;

    let (_, again) = post("/api/pages/doc", json!({ "title": title, "dirPath": "" })).await;

    assert_eq!(again["created"], false);
    assert_eq!(bodies(&again["items"]).len(), 3);
}

// 置けない階層は、置ける深さまで下げる。
#[tokio::test]
async fn a_level_too_deep_for_its_place_is_lowered_until_it_fits() {
    ext_server();
    let title = unique_name("deep-doc");

    let (_, created) = post("/api/pages/doc", json!({ "title": title, "dirPath": "" })).await;

    assert_eq!(
        bodies(&created["items"]),
        vec![(1, "top".to_string()), (2, "too deep".to_string())]
    );
}

// トランスクルード元を指定した項目は、その項目を写す。
#[tokio::test]
async fn an_item_with_a_transclusion_source_mirrors_that_item() {
    ext_server();
    let source_page = new_doc_page("source").await;
    let source = fill(&source_page.id, &["source text"]).await.remove(0);

    let (_, created) = post(
        "/api/pages/doc",
        json!({ "title": format!("tr-{}", source.id), "dirPath": "" }),
    )
    .await;

    let item = &created["items"][0];
    assert_eq!(item["transcludeItemId"], source.id.as_str());
    assert_eq!(item["body"], "source text");
    let (_, source_page) = get(&format!("/api/pages/{}", source_page.id)).await;
    assert_eq!(item["ast"], source_page["items"][0]["ast"]);
}

// タグを足すと、onPageTagAddが足されたタグだけを受け取り、その項目がページの最後に入る。
#[tokio::test]
async fn adding_a_tag_puts_the_extension_items_at_the_end_of_the_page() {
    ext_server();
    let page = new_doc_page("tagged").await;
    fill(&page.id, &["existing"]).await;
    set_tags(&page.id, "old").await;

    set_tags(&page.id, "old, tmpl-ext").await;

    assert_eq!(
        bodies_once(&page.id, 2).await,
        vec!["existing".to_string(), "T tmpl-ext".to_string()]
    );
}

// タグの拡張が項目を書き終えると、そのページの変更がSSEで知らされる。
#[tokio::test]
async fn the_clients_hear_of_the_page_when_the_tag_extensions_are_done() {
    ext_server();
    let page = new_doc_page("tag-event").await;
    let mut events = client().get(url("/api/events")).send().await.unwrap();

    set_tags(&page.id, "tmpl-ext").await;

    let expected = format!("\"pageId\":\"{}\",\"source\":\"extension\"", page.id);
    event_with(&mut events, &expected).await;
}

// タグの拡張がconfirmで尋ねると、クライアントに問い合わせが届き、「はい」の答えで項目が入る。
#[tokio::test]
async fn a_tag_extension_asks_the_clients_and_writes_its_items_after_a_yes() {
    ext_server();
    let page = new_doc_page("confirm").await;
    fill(&page.id, &["existing"]).await;
    let mut events = client().get(url("/api/events")).send().await.unwrap();

    set_tags(&page.id, "ask-ext").await;
    let question = event_with(&mut events, &format!("Insert into {}?", page.title)).await;
    let request_id = question["requestId"].as_str().unwrap();
    let (status, _) = post(&format!("/api/confirm/{request_id}"), json!({ "ok": true })).await;

    assert_eq!(question["type"], "confirm");
    assert_eq!(status, 200);
    assert_eq!(
        bodies_once(&page.id, 2).await,
        vec!["existing".to_string(), "confirmed".to_string()]
    );
}

// 待っていない問い合わせへの答えは、404になる。
#[tokio::test]
async fn an_answer_to_a_question_that_no_one_waits_for_is_not_found() {
    ext_server();

    let (status, _) = post("/api/confirm/unknown", json!({ "ok": true })).await;

    assert_eq!(status, 404);
}

// フォルダ同期で取り込んだページでは、拡張は呼ばれない。
#[tokio::test]
async fn a_page_taken_in_from_the_folder_does_not_call_the_extensions() {
    let server = ext_server();
    let name = unique_name("basic-imported");
    let rel_path = format!("docs/{name}.md");
    std::fs::create_dir_all(server.markdown_dir.join("docs")).unwrap();
    std::fs::write(server.markdown_dir.join(&rel_path), "written by hand\n").unwrap();

    let outcome = server
        .state
        .import
        .sync_file(&server.markdown_dir, &rel_path);

    assert_eq!(outcome, Outcome::Imported);
    let page = server
        .state
        .pages
        .find_by_type_dir_path_file_name("doc", "", &name)
        .unwrap()
        .unwrap();
    let bodies: Vec<String> = items(page.id.as_str())
        .await
        .into_iter()
        .map(|r| r.body)
        .collect();
    assert_eq!(bodies, vec!["written by hand".to_string()]);
}

// 失敗した拡張の項目は取り消され、ほかの拡張の項目は残る。
#[tokio::test]
async fn a_failed_extension_writes_nothing_and_the_others_still_run() {
    ext_server();
    let title = unique_name("broken-doc");

    let (status, created) = post("/api/pages/doc", json!({ "title": title, "dirPath": "" })).await;

    assert_eq!(status, 200);
    assert_eq!(bodies(&created["items"]), vec![(1, "D1".to_string())]);
}

// 別の日への移動が作ったジャーナルにも、拡張が頼んだ項目が入る。動かした項目のあとに入り、答えは動かした項目だけを持つ。
#[tokio::test]
async fn a_journal_made_by_a_move_gets_the_items_its_extension_asked_for_after_the_moved_ones() {
    ext_server();
    let (_, from) = post_empty("/api/journals/2031-02-01").await;
    let from_id = from["page"]["id"].as_str().unwrap();
    let rows = fill(from_id, &["go", "stay"]).await;

    let (status, moved) = post(
        "/api/journals/2031-02-02/move",
        json!({ "itemIds": [rows[0].id], "position": "last" }),
    )
    .await;

    assert_eq!(status, 200, "{moved}");
    assert_eq!(moved["created"], json!(true));
    assert_eq!(bodies(&moved["items"]), vec![(1, "go".to_string())]);
    let (_, page) = get(&format!(
        "/api/pages/{}",
        moved["page"]["id"].as_str().unwrap()
    ))
    .await;
    assert_eq!(
        bodies(&page["items"]),
        vec![(1, "go".to_string()), (1, "J 2031-02-02".to_string())]
    );
}

// 新しいジャーナルも、拡張が頼んだ項目を持って返る。
#[tokio::test]
async fn a_new_journal_comes_with_the_items_its_extension_asked_for() {
    ext_server();

    let (status, created) = post_empty("/api/journals/2031-01-05").await;

    assert_eq!(status, 200);
    assert_eq!(
        bodies(&created["items"]),
        vec![(1, "J 2031-01-05".to_string())]
    );
}
