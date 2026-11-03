//! The startup checks. The test builds a server state of its own, as it takes the tags away
//! straight from the database before the backfill runs.

use journal_server::application::startup;
use journal_server::config::Settings;
use journal_server::domain::model::ItemPosition;
use journal_server::state::{AppState, BuildOptions};
use serde_json::json;

// 起動時に、タグが1つもなければ、本文のハッシュタグから項目のタグが付け直される。
#[test]
fn the_tags_are_backfilled_from_the_hashtags_when_none_exists() {
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
    let page = state
        .page_service
        .get_or_create_doc(Some("notes"), Some(""))
        .unwrap()
        .page;
    let item = state
        .item_service
        .create_item(
            &page.id,
            &ItemPosition::Last,
            1,
            Some("remember #urgent #later"),
            None,
        )
        .unwrap();
    state
        .db
        .conn()
        .unwrap()
        .execute("DELETE FROM property_list_items WHERE key = 'tags'", [])
        .unwrap();
    let tags = |id: &str| state.properties.find_tags("item", id).unwrap();
    assert!(tags(item.id.as_str()).is_empty());

    startup::backfill_hashtags(&state.pages, &state.items, &state.properties);

    assert_eq!(tags(item.id.as_str()), vec!["urgent", "later"]);
}
