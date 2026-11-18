//! A throw-away server, as the tests start it: everything it writes is under one folder
//! (ADR 002).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::common::{client, start, TestServer};

async fn config_of(server: &TestServer) -> Value {
    client()
        .get(format!("{}/api/config", server.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

fn markdown_dir_of(config: &Value) -> PathBuf {
    PathBuf::from(config["markdownDir"].as_str().unwrap())
}

// 使い捨てのサーバーは、設定ファイルが外のMarkdownフォルダーを指していても、自分のフォルダーの下を使う。
#[tokio::test]
async fn a_throw_away_server_keeps_its_pages_under_its_folder_whatever_the_config_file_says() {
    let outside = tempfile::tempdir().unwrap();
    let real_notes = outside.path().to_path_buf();
    let server = start(|config_dir: &Path| {
        std::fs::write(
            config_dir.join("config.json"),
            json!({ "markdownDir": real_notes.to_string_lossy() }).to_string(),
        )
        .unwrap();
    });

    let config = config_of(&server).await;
    assert_eq!(markdown_dir_of(&config), server.root.join("markdown"));

    // A page is written under the folder of the server, and nothing into the folder outside
    let page = server
        .state
        .page_service
        .get_or_create_doc(Some("kept-inside"), Some(""))
        .unwrap();
    server.state.export.process_export_queue();
    assert!(
        server.root.join("markdown/docs/kept-inside.md").is_file(),
        "{}",
        page.page.id
    );
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
}

// 使い捨てのサーバーは、外のMarkdownフォルダーへの設定の変更を受け付けない。
#[tokio::test]
async fn a_throw_away_server_does_not_move_its_markdown_folder_out_of_its_folder() {
    let outside = tempfile::tempdir().unwrap();
    let server = start(|_| {});

    let answer: Value = client()
        .put(format!("{}/api/config", server.base_url))
        .json(&json!({ "markdownDir": outside.path().to_string_lossy() }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(markdown_dir_of(&answer), server.root.join("markdown"));
    assert_eq!(
        markdown_dir_of(&config_of(&server).await),
        server.root.join("markdown")
    );

    // A folder under its own is taken
    let inside = server.root.join("other-notes");
    let answer: Value = client()
        .put(format!("{}/api/config", server.base_url))
        .json(&json!({ "markdownDir": inside.to_string_lossy() }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(markdown_dir_of(&answer), inside);
}
