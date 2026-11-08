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
        vec![(1, "A1".to_string()), (2, "A2".to_string())]
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
