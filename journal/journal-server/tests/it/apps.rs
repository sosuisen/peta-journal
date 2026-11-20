//! The extension web apps, which the journal server serves from `apps/` of the config folder
//! (ADR 073).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::common::*;

const INDEX: &str = "<!doctype html><title>Sample app</title>";
const SCRIPT: &str = "console.log('sample');";

/// Puts an app with `index.html` and `app.js` into `apps/` of the shared server, which serves
/// no web frontend. Each app gets a folder of its own, so the tests do not see each other's.
fn put_app() -> (String, PathBuf) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = format!("sample-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let folder = server().root.join("config").join("apps").join(&name);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("index.html"), INDEX).unwrap();
    std::fs::write(folder.join("app.js"), SCRIPT).unwrap();
    (name, folder)
}

async fn fetch(path: &str) -> reqwest::Response {
    client().get(url(path)).send().await.expect("GET")
}

// アプリのフォルダにあるファイルは、`/apps/{url_fragment}/`の下のパスで返る。Web版の画面を配信していなくても返る。
#[tokio::test]
async fn a_file_of_an_app_folder_is_served_under_apps() {
    let (app, _) = put_app();

    let response = fetch(&format!("/apps/{app}/app.js")).await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "text/javascript");
    assert_eq!(response.text().await.unwrap(), SCRIPT);
}

// `/apps/{url_fragment}/`には、そのアプリの`index.html`が返る。
#[tokio::test]
async fn the_root_of_an_app_gets_its_index_html() {
    let (app, _) = put_app();

    let response = fetch(&format!("/apps/{app}/")).await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), INDEX);
}

// 拡張子のないパスでファイルがないときは、そのアプリの`index.html`が返る。アプリが画面をパスで表すためである。
#[tokio::test]
async fn a_path_without_an_extension_and_a_file_gets_the_index_html_of_the_app() {
    let (app, _) = put_app();

    for path in ["tasks", "tasks/1"] {
        let response = fetch(&format!("/apps/{app}/{path}")).await;

        assert_eq!(response.status(), 200, "{path}");
        assert_eq!(response.headers()["content-type"], "text/html", "{path}");
        assert_eq!(response.text().await.unwrap(), INDEX, "{path}");
    }
}
