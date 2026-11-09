//! The web frontend, which the journal server serves from a folder (ADR 010).

use std::sync::OnceLock;

use crate::common::*;

const INDEX: &str = "<!doctype html><title>Frontend</title>";
const STYLES: &str = "body { margin: 0 }";

/// A server of its own that serves a folder like the `dist/` Trunk writes.
fn web() -> &'static TestServer {
    static WEB: OnceLock<(TestServer, tempfile::TempDir)> = OnceLock::new();
    let (server, _dist) = WEB.get_or_init(|| {
        let dist = tempfile::tempdir().expect("temp dir");
        std::fs::write(dist.path().join("index.html"), INDEX).unwrap();
        std::fs::write(dist.path().join("styles-9508fe9cb1262d3b.css"), STYLES).unwrap();
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
