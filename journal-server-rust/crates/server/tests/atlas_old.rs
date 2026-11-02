//! The atlas of old, kept in page properties, moves into the outline of the atlas page. The
//! test builds a server state of its own, as it writes the old keys straight into the
//! database.

use std::collections::BTreeMap;

use journal_server::config::Settings;
use journal_server::state::{AppState, BuildOptions};
use serde_json::json;

// 旧形式のアトラスは、アトラスページのアウトラインに移される。
#[test]
fn the_atlas_of_old_moves_into_the_outline_of_the_atlas_page() {
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
    let pages = &state.page_service;
    let a = pages.get_or_create_doc(Some("a"), Some("d/")).unwrap().page;
    let b = pages.get_or_create_doc(Some("b"), Some("d/")).unwrap().page;
    let atlas = pages
        .get_or_create_doc(Some("index_atlas"), Some("d/"))
        .unwrap()
        .page;
    let props = &state.properties;
    props
        .upsert_scalar("page", a.id.as_str(), "atlas_bounds", "bounds", "1,2,0,40")
        .unwrap();
    props
        .upsert_scalar(
            "page",
            atlas.id.as_str(),
            "atlas_title_x",
            "text",
            "left:Tools",
        )
        .unwrap();
    props
        .upsert_scalar(
            "page",
            atlas.id.as_str(),
            "atlas_callout_y",
            "text",
            "10,20,1.5,top:Hi",
        )
        .unwrap();
    {
        let conn = state.db.conn().unwrap();
        conn.execute(
            "INSERT INTO property_definitions (key, value_type, is_system, for_page) \
             VALUES ('atlas_group_x', 'list', 1, 1)",
            [],
        )
        .unwrap();
        for (i, id) in [&a.id, &b.id].iter().enumerate() {
            conn.execute(
                "INSERT INTO property_list_items (target_type, target_id, key, sort_order, value) \
                 VALUES ('page', ?, 'atlas_group_x', ?, ?)",
                rusqlite::params![atlas.id.as_str(), i as i64, id.as_str()],
            )
            .unwrap();
        }
    }
    let a_modified = pages.get_page(&a.id).unwrap().unwrap().modified_date;

    assert_eq!(state.atlas.move_old_properties().unwrap(), 2);

    let items = pages.get_page_items(&atlas.id).unwrap();
    let outline: Vec<String> = items
        .iter()
        .map(|i| format!("{}:{}", i.level, i.body))
        .collect();
    assert_eq!(
        outline,
        ["1:Tools", "2:[[d/a]]", "2:[[d/b]]", "1:callout", "2:Hi"]
    );
    let ids: Vec<String> = items.iter().map(|i| i.id.to_string()).collect();
    let item_props = props.find_scalar_by_targets("item", &ids).unwrap();
    let of = |n: usize| item_props.get(&ids[n]).cloned().unwrap_or_default();
    assert_eq!(of(0)["atlas_side"], "left");
    assert_eq!(of(1)["atlas_bounds"], "1,2,0,40");
    assert_eq!(of(4)["atlas_bounds"], "10,20,0,60");
    assert_eq!(of(4)["atlas_side"], "top");

    let page_ids = vec![a.id.to_string(), atlas.id.to_string()];
    let left: BTreeMap<_, _> = props.find_scalar_by_targets("page", &page_ids).unwrap();
    assert!(
        left.values()
            .all(|p| p.keys().all(|k| !k.starts_with("atlas_"))),
        "{left:?}"
    );
    assert!(props
        .find_list_by_targets("page", &page_ids)
        .unwrap()
        .is_empty());
    let keys: Vec<String> = props.find_all_definitions().unwrap().into_keys().collect();
    assert!(
        !keys.iter().any(|k| k.starts_with("atlas_group_")
            || k.starts_with("atlas_title_")
            || k.starts_with("atlas_callout_")),
        "{keys:?}"
    );
    assert_eq!(
        pages.get_page(&a.id).unwrap().unwrap().modified_date,
        a_modified
    );
    // Nothing is left to move
    assert_eq!(state.atlas.move_old_properties().unwrap(), 0);
}
