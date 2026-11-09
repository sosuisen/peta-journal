//! The web frontend, which the journal server serves from a folder (ADR 010).

use std::sync::OnceLock;

use crate::common::*;

const STYLES: &str = "body { margin: 0 }";

/// A server of its own that serves a folder like the `dist/` Trunk writes.
fn web() -> &'static TestServer {
    static WEB: OnceLock<(TestServer, tempfile::TempDir)> = OnceLock::new();
    let (server, _dist) = WEB.get_or_init(|| {
        let dist = tempfile::tempdir().expect("temp dir");
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
