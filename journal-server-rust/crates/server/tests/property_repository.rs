//! The property repository on a database of its own.

use journal_server::config::Settings;
use journal_server::infrastructure::persistence::PropertyRepository;
use journal_server::state::{AppState, BuildOptions};
use serde_json::json;

// 定義のないキーに値を書くことは拒否される。
#[test]
fn a_value_of_a_key_with_no_definition_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let markdown_dir = dir.path().join("markdown");
    let config_dir = dir.path().join("config");
    std::fs::create_dir_all(&markdown_dir).unwrap();
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("config.json"),
        json!({ "markdownDir": markdown_dir.to_string_lossy() }).to_string(),
    )
    .unwrap();
    let state = AppState::build(
        Settings::under(dir.path(), 0),
        BuildOptions { interactive: false },
    )
    .unwrap();
    let conn = state.db.conn().unwrap();

    assert!(PropertyRepository::write_scalar_in(&conn, "item", "x", "undefined", "1").is_err());
    PropertyRepository::upsert_scalar_in(&conn, "item", "x", "defined", "text", "1").unwrap();
    assert!(PropertyRepository::write_scalar_in(&conn, "item", "x", "defined", "2").is_ok());
}
