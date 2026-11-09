//! The web frontend, which the journal server serves from a folder (ADR 010).

use std::sync::OnceLock;

use crate::common::*;

const INDEX: &str = "<!doctype html><title>Frontend</title>";
const STYLES: &str = "body { margin: 0 }";
const FROM_THE_FOLDER: &str = "a file of the frontend folder";

/// A server of its own that serves a folder like the `dist/` Trunk writes.
fn web() -> &'static TestServer {
    static WEB: OnceLock<(TestServer, tempfile::TempDir)> = OnceLock::new();
    let (server, _dist) = WEB.get_or_init(|| {
        let dist = tempfile::tempdir().expect("temp dir");
        std::fs::write(dist.path().join("index.html"), INDEX).unwrap();
        std::fs::write(dist.path().join("styles-9508fe9cb1262d3b.css"), STYLES).unwrap();
        // Files at the paths of the server's own routes, which must never be served
        for shadowed in ["api/health", "assets/logo.png", "mcp"] {
            let file = dist.path().join(shadowed);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, FROM_THE_FOLDER).unwrap();
        }
        let server = start_with(
            |_| {},
            |settings| settings.frontend_dir = Some(dist.path().to_path_buf()),
        );
        (server, dist)
    });
    server
}

async fn fetch(path: &str) -> reqwest::Response {
    client()
        .get(format!("{}{path}", web().base_url))
        .send()
        .await
        .expect("GET")
}

// 画面のフォルダにあるファイルは、その名前のパスで返る。
#[tokio::test]
async fn a_file_of_the_frontend_folder_is_served_at_its_path() {
    let response = fetch("/styles-9508fe9cb1262d3b.css").await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "text/css");
    assert_eq!(response.text().await.unwrap(), STYLES);
}

// `/`には、`index.html`が返る。
#[tokio::test]
async fn the_root_path_gets_index_html() {
    let response = fetch("/").await;

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), INDEX);
}

// ファイルのないパスには、`index.html`が返る。アプリがページをパスで表すためである。
#[tokio::test]
async fn a_path_without_a_file_gets_index_html() {
    for path in ["/pages/some-page", "/today"] {
        let response = fetch(path).await;

        assert_eq!(response.status(), 200, "{path}");
        assert_eq!(response.headers()["content-type"], "text/html", "{path}");
        assert_eq!(response.text().await.unwrap(), INDEX, "{path}");
    }
}

// `/api`、`/assets`、`/mcp`のルートは、同じパスにある画面のファイルより優先される。
#[tokio::test]
async fn the_routes_of_the_server_come_before_the_files_of_the_frontend() {
    let health = fetch("/api/health").await;
    assert_eq!(health.status(), 200);
    assert!(health.text().await.unwrap().contains("peta-journal-server"));

    // The assets are the images of the pages, and no page has this one
    let asset = fetch("/assets/logo.png").await;
    assert_eq!(asset.status(), 404);

    let mcp = fetch("/mcp").await;
    let status = mcp.status();
    assert_ne!(mcp.text().await.unwrap(), FROM_THE_FOLDER, "{status}");
}

// `/api/`で始まるルートのないパスには、`index.html`ではなく404が返る。APIを呼ぶ側が、画面のHTMLを答えと取り違えないためである。
#[tokio::test]
async fn an_unknown_api_path_is_not_found_instead_of_index_html() {
    let response = fetch("/api/no-such-route").await;

    assert_eq!(response.status(), 404);
}

// 画面のフォルダが設定されていなければ、画面は配信されない。
#[tokio::test]
async fn no_frontend_is_served_without_a_frontend_folder() {
    let response = client().get(url("/today")).send().await.expect("GET");

    assert_eq!(response.status(), 404);
}

// ---- Caching ----

// `index.html`は、キャッシュされずに毎回確かめられる。作り直したアプリが、読み直しで必ず届く。
#[tokio::test]
async fn index_html_is_checked_again_on_every_load() {
    for path in ["/", "/index.html", "/pages/some-page"] {
        let response = fetch(path).await;

        assert_eq!(response.headers()["cache-control"], "no-cache", "{path}");
    }
}
