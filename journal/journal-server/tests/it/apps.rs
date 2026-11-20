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

// 拡張子のあるパスでファイルがないときは、`index.html`ではなく404が返る。置き忘れたファイルの代わりに、HTMLが読まれないためである。
#[tokio::test]
async fn a_path_with_an_extension_and_no_file_is_not_found() {
    let (app, _) = put_app();

    for path in ["missing.js", "assets/missing.css"] {
        let response = fetch(&format!("/apps/{app}/{path}")).await;

        assert_eq!(response.status(), 404, "{path}");
    }
}

// アプリのフォルダがないときは、どのパスにも404が返る。
#[tokio::test]
async fn a_path_of_an_app_without_a_folder_is_not_found() {
    for path in [
        "/apps/no-such-app/",
        "/apps/no-such-app/tasks",
        "/apps/no-such-app/app.js",
    ] {
        let response = fetch(path).await;

        assert_eq!(response.status(), 404, "{path}");
    }
}

// 末尾の`/`がない`/apps/{url_fragment}`は、`/apps/{url_fragment}/`へリダイレクトされる。アプリの相対URLが、アプリのフォルダを指すためである。
#[tokio::test]
async fn the_path_of_an_app_without_the_last_slash_is_redirected_to_the_one_with_it() {
    let (app, _) = put_app();
    let no_redirects = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let response = no_redirects
        .get(url(&format!("/apps/{app}?port=9000")))
        .send()
        .await
        .expect("GET");

    assert!(response.status().is_redirection(), "{}", response.status());
    assert_eq!(
        response.headers()["location"],
        format!("/apps/{app}/?port=9000").as_str()
    );
}

// 消したアプリのフォルダは、サーバーを起動し直さなくても配信されなくなる。
#[tokio::test]
async fn an_app_folder_taken_away_is_no_longer_served_without_a_restart() {
    let (app, folder) = put_app();
    assert_eq!(fetch(&format!("/apps/{app}/")).await.status(), 200);

    std::fs::remove_dir_all(folder).unwrap();

    assert_eq!(fetch(&format!("/apps/{app}/")).await.status(), 404);
}

// ---- Caching ----

// アプリのファイルは、名前にハッシュがあってもなくても、キャッシュされずに毎回確かめられる。サーバーは、ビルドツールごとのハッシュの形を知らないためである。
#[tokio::test]
async fn every_file_of_an_app_is_checked_again_on_every_load() {
    let (app, folder) = put_app();
    std::fs::create_dir_all(folder.join("assets")).unwrap();
    std::fs::write(folder.join("assets/index-9508fe9c.js"), SCRIPT).unwrap();
    std::fs::write(folder.join("styles-9508fe9cb1262d3b.css"), "").unwrap();

    for path in [
        "",
        "index.html",
        "tasks/1",
        "app.js",
        "assets/index-9508fe9c.js",
        "styles-9508fe9cb1262d3b.css",
    ] {
        let response = fetch(&format!("/apps/{app}/{path}")).await;

        assert_eq!(response.status(), 200, "{path}");
        assert_eq!(response.headers()["cache-control"], "no-cache", "{path}");
    }
}
