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
            "j-journal",
            "globalThis.onPageCreate = (page) => {
                if (page.pageType !== 'journal' || !page.fileName.startsWith('2031-')) return;
                peta.createItem({ position: 'last', level: 1, body: 'J ' + page.fileName });
            };",
        );
    })
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
