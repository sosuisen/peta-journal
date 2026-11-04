//! The API tests of the Java server, run against the Rust server.

use crate::common::*;
use serde_json::{json, Value};

fn vs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// ---- Items: creating ----

// 項目は、先頭、末尾、または別の項目の後ろに作られる。
#[tokio::test]
async fn items_go_first_last_or_after_another_item() {
    let page = new_doc_page("items-create").await;
    let rows = fill(&page.id, &["a", "b"]).await;

    create_item(&page.id, "first", 1, "x").await;
    create_item(&page.id, "last", 1, "y").await;
    create_item(&page.id, &format!("after:{}", rows[0].id), 2, "z").await;

    assert_eq!(
        outline_of(&page.id).await,
        vs(&["1:x", "1:a", "2:z", "1:b", "1:y"])
    );
}

/// The indexes of the items of a page, in order
async fn idxs_of(page_id: &str) -> Vec<String> {
    let (_, resp) = get(&format!("/api/pages/{page_id}")).await;
    resp["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["idx"].as_str().unwrap().to_string())
        .collect()
}

/// Writes the indexes straight into the database, as a database from before would have them
fn set_indexes(rows: &[ItemRow], indexes: &[&str]) {
    let conn = server().state.db.conn().unwrap();
    for (row, idx) in rows.iter().zip(indexes) {
        conn.execute(
            "UPDATE items SET idx = ? WHERE id = ?",
            rusqlite::params![idx, row.id],
        )
        .unwrap();
    }
}

/// Gives the items the indexes of the scheme before this one
fn give_old_indexes(rows: &[ItemRow]) {
    set_indexes(rows, &["z0", "z1", "z2", "z3"]);
}

/// An index at the limit, and one right after it: nothing short fits between the two
fn tight_gap() -> (String, String) {
    let at_limit = format!("a0{}", "V".repeat(14));
    assert_eq!(at_limit.len(), 16);
    (at_limit.clone(), format!("{at_limit}W"))
}

// 狭い隙間に項目を動かしてインデックスが長くなると、その周りだけが付け直される。
#[tokio::test]
async fn moving_an_item_into_a_tight_gap_reindexes_around_the_gap() {
    let page = new_doc_page("items-move-tight").await;
    let rows = fill(&page.id, &["a", "b", "c"]).await;
    let (a, b) = tight_gap();
    set_indexes(&rows, &[&a, &b, "a1"]);

    move_items(
        &[&rows[2].id],
        &page.id,
        &format!("after:{}", rows[0].id),
        None,
    )
    .await;

    assert_eq!(outline_of(&page.id).await, vs(&["1:a", "1:c", "1:b"]));
    let idxs = idxs_of(&page.id).await;
    assert!(idxs.iter().all(|idx| idx.len() <= 16), "{idxs:?}");
}

// 長いインデックスの隣に旧い形のインデックスがあれば、ページ全体が付け直される。
#[tokio::test]
async fn an_index_of_the_old_scheme_next_to_a_long_one_makes_the_whole_page_reindex() {
    let page = new_doc_page("items-old-next-to-long").await;
    let rows = fill(&page.id, &["a", "b", "c"]).await;
    let (a, b) = tight_gap();
    set_indexes(&rows, &[&a, &b, "z5"]);

    create_item(&page.id, &format!("after:{}", rows[0].id), 1, "x").await;

    assert_eq!(
        outline_of(&page.id).await,
        vs(&["1:a", "1:x", "1:b", "1:c"])
    );
    assert_eq!(idxs_of(&page.id).await, vs(&["a0", "a1", "a2", "a3"]));
}

// 旧い形のインデックスを持つページに項目を足すか動かすと、ページの全項目が付け直される。
#[tokio::test]
async fn a_page_with_indexes_of_the_old_scheme_is_reindexed_when_an_item_is_added_or_moved() {
    let page = new_doc_page("items-old-idx").await;
    let rows = fill(&page.id, &["a", "b"]).await;
    give_old_indexes(&rows);

    create_item(&page.id, &format!("after:{}", rows[0].id), 1, "x").await;

    assert_eq!(outline_of(&page.id).await, vs(&["1:a", "1:x", "1:b"]));
    assert_eq!(idxs_of(&page.id).await, vs(&["a0", "a0V", "a1"]));

    let rows = items(&page.id).await;
    give_old_indexes(&rows);
    move_items(&[&rows[2].id], &page.id, "first", None).await;

    assert_eq!(outline_of(&page.id).await, vs(&["1:b", "1:a", "1:x"]));
    assert_eq!(idxs_of(&page.id).await, vs(&["Zz", "a0", "a1"]));
}

// 同じ隙間に入れ続けてインデックスが長くなると、その周りだけが付け直される。
#[tokio::test]
async fn indexes_grown_long_in_one_gap_are_reindexed_around_that_gap_only() {
    let page = new_doc_page("items-long-idx").await;
    let rows = fill(&page.id, &["a", "b"]).await;
    let before = idxs_of(&page.id).await;
    let (_, b_before) = get(&format!("/api/items/{}", rows[1].id)).await;

    // Each new item goes right under "a", between it and the one before
    for n in 0..90 {
        create_item(
            &page.id,
            &format!("after:{}", rows[0].id),
            1,
            &format!("x{n}"),
        )
        .await;
    }

    let outline = outline_of(&page.id).await;
    assert_eq!(outline.len(), 92);
    assert_eq!(outline[0], "1:a");
    assert_eq!(outline[1], "1:x89");
    assert_eq!(outline[90], "1:x0");
    assert_eq!(outline[91], "1:b");
    let idxs = idxs_of(&page.id).await;
    assert_eq!(idxs[0], before[0]);
    assert_eq!(idxs[91], before[1]);
    assert!(idxs.iter().all(|idx| idx.len() <= 16), "{idxs:?}");
    // The items reindexed were not changed: b keeps its index and its modified date
    let (_, b_after) = get(&format!("/api/items/{}", rows[1].id)).await;
    assert_eq!(
        b_after["item"]["modifiedDate"],
        b_before["item"]["modifiedDate"]
    );
}

// クライアントが付けたIDがそのまま使われる。
#[tokio::test]
async fn a_client_id_is_kept() {
    let page = new_doc_page("items-client-id").await;
    // What the frontend makes up for an optimistic item (npm hmtid default layout)
    let id = hmtid::Generator::new('_', false).generate().to_string();

    let (status, resp) = post(
        "/api/items",
        json!({ "pageId": page.id, "position": "last", "level": 1, "body": "mine", "id": id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["item"]["id"].as_str(), Some(id.as_str()));
}

// 本文のハッシュタグがtagsプロパティとして返る。
#[tokio::test]
async fn a_hashtag_in_the_body_comes_back_as_a_tag_property() {
    let page = new_doc_page("items-hashtag").await;

    let created = create_item(&page.id, "last", 1, "remember #urgent").await;
    let updated = update_item(&created.id, json!({ "body": "remember #urgent #later" })).await;

    assert_eq!(created.properties["tags"], json!(["urgent"]));
    assert_eq!(updated.properties["tags"], json!(["urgent", "later"]));
}

// ページにプロパティを設定すると、ページの変更日が新しくなる。
#[tokio::test]
async fn setting_a_property_counts_as_a_change_of_the_page() {
    let page = new_doc_page("pages-property-date").await;
    let old = date_the_page_in_the_past(&page.id).await;

    let (status, resp) = put(
        &format!("/api/pages/{}/properties/tags", page.id),
        json!({ "valueType": "tags", "value": "dated" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_ne!(resp["page"]["modifiedDate"], json!(old));
}

// ---- Items: updating ----

// 本文はASTと型に解析される。
#[tokio::test]
async fn the_body_is_parsed_into_an_ast_and_types() {
    let page = new_doc_page("items-parse").await;
    let plain = fill(&page.id, &["plain"]).await.remove(0);

    let heading = update_item(&plain.id, json!({ "body": "## Title" })).await;

    assert!(
        heading.types.contains(&"heading".to_string()),
        "types: {:?}",
        heading.types
    );
    let (_, item) = get(&format!("/api/items/{}", plain.id)).await;
    assert!(item["item"]["ast"]
        .as_str()
        .unwrap()
        .contains("\"heading\""));
}

// レベルと折りたたみ状態が保存される。
#[tokio::test]
async fn level_and_collapsed_state_are_saved() {
    let page = new_doc_page("items-flags").await;
    let rows = fill(&page.id, &["p", "  c"]).await;

    update_item(&rows[0].id, json!({ "collapsed": true })).await;
    update_item(&rows[1].id, json!({ "level": 3 })).await;

    let after = items(&page.id).await;
    assert!(after[0].collapsed);
    assert_eq!(after[1].level, 3);
}

// 項目にプロパティを設定できる。
#[tokio::test]
async fn a_property_can_be_set_on_an_item() {
    let page = new_doc_page("items-property").await;
    let item = fill(&page.id, &["task-like"]).await.remove(0);

    let (status, _) = put(
        &format!("/api/items/{}/properties/plan", item.id),
        json!({ "valueType": "date", "value": "2026-03-01" }),
    )
    .await;
    assert_eq!(status, 200);

    assert_eq!(
        items(&page.id).await[0].properties["plan"],
        json!("2026-03-01")
    );
}

// checkbox型のプロパティは、trueとfalseだけを持つ。それ以外の値はfalseになる。
#[tokio::test]
async fn a_checkbox_property_holds_true_or_false() {
    let page = new_doc_page("items-checkbox").await;
    let item = fill(&page.id, &["checked"]).await.remove(0);

    for (sent, held) in [("true", "true"), ("false", "false"), ("yes", "false")] {
        let (status, _) = put(
            &format!("/api/items/{}/properties/done", item.id),
            json!({ "valueType": "checkbox", "value": sent }),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(items(&page.id).await[0].properties["done"], json!(held));
    }
}

// 項目のユーザー定義のlist型のキーは、そのキーのリストとして保存され、タグには触らない。
#[tokio::test]
async fn a_list_of_an_item_is_kept_under_its_own_key_apart_from_the_tags() {
    let page = new_doc_page("items-list").await;
    let item = fill(&page.id, &["listed #items-list-tag"]).await.remove(0);

    // The second value replaces the first, as a value of a scalar does
    let mut last = Value::Null;
    for value in ["x, y", "y,z"] {
        let (status, resp) = put(
            &format!("/api/items/{}/properties/item_list", item.id),
            json!({ "valueType": "list", "value": value }),
        )
        .await;
        assert_eq!(status, 200, "{resp}");
        last = resp;
    }

    assert_eq!(last["item"]["properties"]["item_list"], json!(["y", "z"]));
    let after = items(&page.id).await.remove(0);
    assert_eq!(after.properties["item_list"], json!(["y", "z"]));
    assert_eq!(after.properties["tags"], json!(["items-list-tag"]));
}

// 削除した項目はページから消える。
#[tokio::test]
async fn a_deleted_item_is_gone_from_its_page() {
    let page = new_doc_page("items-delete").await;
    let rows = fill(&page.id, &["keep", "drop"]).await;

    assert_eq!(delete(&format!("/api/items/{}", rows[1].id)).await, 204);

    assert_eq!(outline_of(&page.id).await, vs(&["1:keep"]));
}

// ---- Items: levels ----

// shift-levelは、複数の項目のレベルを相対量だけ動かす。
#[tokio::test]
async fn shift_level_moves_a_group_by_a_relative_amount() {
    let page = new_doc_page("items-shift").await;
    let rows = fill(&page.id, &["a", "b", "  b1", "c"]).await;

    let (status, _) = post(
        "/api/items/shift-level",
        json!({ "itemIds": [rows[1].id, rows[2].id], "delta": 1 }),
    )
    .await;
    assert_eq!(status, 200);

    assert_eq!(
        outline_of(&page.id).await,
        vs(&["1:a", "2:b", "3:b1", "1:c"])
    );
}

// ---- Items: moving ----

// 部分木は、同じページの兄弟を越えて移動する。
#[tokio::test]
async fn a_subtree_moves_past_its_sibling_on_the_same_page() {
    let page = new_doc_page("items-move").await;
    let rows = fill(&page.id, &["a", "b", "  b1", "c"]).await;

    move_items(
        &[&rows[1].id, &rows[2].id],
        &page.id,
        &format!("after:{}", rows[3].id),
        None,
    )
    .await;
    assert_eq!(
        outline_of(&page.id).await,
        vs(&["1:a", "1:c", "1:b", "2:b1"])
    );

    move_items(&[&rows[1].id, &rows[2].id], &page.id, "first", None).await;
    assert_eq!(
        outline_of(&page.id).await,
        vs(&["1:b", "2:b1", "1:a", "1:c"])
    );
}

// 項目は、レベルをずらして別のページへ移動する。
#[tokio::test]
async fn items_move_to_another_page_with_their_levels_shifted() {
    let from = new_doc_page("items-move-from").await;
    let to = new_doc_page("items-move-to").await;
    let rows = fill(&from.id, &["stay", "go", "  go-child"]).await;
    let target = fill(&to.id, &["parent"]).await.remove(0);

    let moved = move_items(
        &[&rows[1].id, &rows[2].id],
        &to.id,
        &format!("after:{}", target.id),
        Some(1),
    )
    .await;

    assert_eq!(
        moved.iter().map(ItemRow::outline).collect::<Vec<_>>(),
        vs(&["2:go", "3:go-child"])
    );
    assert_eq!(outline_of(&from.id).await, vs(&["1:stay"]));
    assert_eq!(
        outline_of(&to.id).await,
        vs(&["1:parent", "2:go", "3:go-child"])
    );
}

// ---- Items: tasks ----

// cycle-taskは、タスク、実行中、完了を経て通常の項目に戻る。
#[tokio::test]
async fn cycle_task_goes_through_task_running_done_and_back_to_normal() {
    let page = new_doc_page("items-task").await;
    let item = fill(&page.id, &["buy milk"]).await.remove(0);
    let cycle = format!("/api/items/{}/cycle-task", item.id);

    assert_eq!(post_empty(&cycle).await.0, 200);
    let task = items(&page.id).await.remove(0);
    assert!(task.types.contains(&"task".to_string()));
    assert!(!task.properties.contains_key("begin"));

    // The response carries the times it just set, so the frontend can show them at once
    let (status, resp) = post_empty(&cycle).await;
    assert_eq!(status, 200);
    let running = ItemRow::of(&resp["item"]);
    assert!(
        running.properties.contains_key("begin"),
        "response: {:?}",
        running.properties
    );
    assert!(items(&page.id).await[0].properties.contains_key("begin"));

    let (_, resp) = post_empty(&cycle).await;
    let done = ItemRow::of(&resp["item"]);
    assert!(
        done.properties.contains_key("end"),
        "response: {:?}",
        done.properties
    );
    assert!(items(&page.id).await[0].properties.contains_key("end"));

    assert_eq!(post_empty(&cycle).await.0, 200);
    let normal = items(&page.id).await.remove(0);
    assert!(!normal.types.contains(&"task".to_string()));
    assert!(!normal.properties.contains_key("begin"));
    assert!(!normal.properties.contains_key("end"));
}

// タスクは型で一覧できる。
#[tokio::test]
async fn tasks_can_be_listed_by_type() {
    let page = new_doc_page("items-task-list").await;
    let item = fill(&page.id, &["listed task"]).await.remove(0);
    assert_eq!(
        post_empty(&format!("/api/items/{}/cycle-task", item.id))
            .await
            .0,
        200
    );

    let (status, resp) = get("/api/items?types=task").await;
    assert_eq!(status, 200);
    let ids: Vec<Value> = field_of_each(&resp, "items", "id");
    assert!(ids.contains(&json!(item.id)));
}

// ---- Items: transclusion ----

// トランスクルージョンは、ページとともに参照元の木を返す。
#[tokio::test]
async fn a_transclusion_shows_the_source_tree_with_the_page() {
    let source_page = new_doc_page("items-transclusion-source").await;
    let source = fill(&source_page.id, &["root", "  child"]).await;
    let page = new_doc_page("items-transclusion").await;
    let host = fill(&page.id, &["placeholder"]).await.remove(0);

    let (status, resp) = post(
        &format!("/api/items/{}/transclude", host.id),
        json!({ "sourceItemId": source[0].id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let linked = ItemRow::of(&resp["item"]);
    assert_eq!(
        linked.transclude_item_id.as_deref(),
        Some(source[0].id.as_str())
    );

    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    let tree = &resp["transcludedTrees"][&source[0].id];
    assert_eq!(
        field_of_each(&json!({ "t": tree }), "t", "body"),
        vec![json!("root"), json!("child")]
    );

    assert_eq!(
        delete(&format!("/api/items/{}/transclude", host.id)).await,
        200
    );
    assert!(items(&page.id).await[0].transclude_item_id.is_none());
}

// 削除の前に、どの項目が他の項目からトランスクルードされているかを尋ねられる。
#[tokio::test]
async fn the_items_other_items_transclude_are_told_apart_before_a_deletion() {
    let source_page = new_doc_page("items-transcluded-source").await;
    let source = fill(&source_page.id, &["shown elsewhere", "kept here"]).await;
    let page = new_doc_page("items-transcluded").await;
    let host = fill(&page.id, &["placeholder"]).await.remove(0);
    post(
        &format!("/api/items/{}/transclude", host.id),
        json!({ "sourceItemId": source[0].id }),
    )
    .await;

    let (status, resp) = post(
        "/api/items/transcluded",
        json!({ "itemIds": [source[0].id, source[1].id, host.id] }),
    )
    .await;

    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["itemIds"], json!([source[0].id]));
}

// トランスクルージョンを外すと、項目の本文は空になり、保存済みのASTも空になる。
#[tokio::test]
async fn removing_a_transclusion_leaves_an_empty_body_and_an_empty_ast() {
    let source_page = new_doc_page("items-transclusion-remove-source").await;
    let source = fill(&source_page.id, &["**root**"]).await;
    let page = new_doc_page("items-transclusion-remove").await;
    let host = fill(&page.id, &["placeholder"]).await.remove(0);
    post(
        &format!("/api/items/{}/transclude", host.id),
        json!({ "sourceItemId": source[0].id }),
    )
    .await;
    let (_, resp) = get(&format!("/api/items/{}", host.id)).await;
    assert_ne!(resp["item"]["ast"], json!(""), "{resp}");

    assert_eq!(
        delete(&format!("/api/items/{}/transclude", host.id)).await,
        200
    );

    let (_, resp) = get(&format!("/api/items/{}", host.id)).await;
    assert_eq!(resp["item"]["body"], json!(""));
    assert_eq!(resp["item"]["ast"], json!(""));
}

// トランスクルージョンの設定は項目の変更として数えられ、日付での同期がそれを拾う。
#[tokio::test]
async fn setting_a_transclusion_counts_as_a_change_of_the_item() {
    let source_page = new_doc_page("items-transclusion-change-source").await;
    let source = fill(&source_page.id, &["root"]).await;
    let page = new_doc_page("items-transclusion-change").await;
    let host = fill(&page.id, &["placeholder"]).await.remove(0);
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    let modified_before = field_of_each(&resp, "items", "modifiedDate").remove(0);
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let (status, resp) = post(
        &format!("/api/items/{}/transclude", host.id),
        json!({ "sourceItemId": source[0].id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    let modified_after = field_of_each(&resp, "items", "modifiedDate").remove(0);
    assert!(
        modified_after.as_i64() > modified_before.as_i64(),
        "{modified_before} -> {modified_after}"
    );
}

// 参照元の項目を編集すると、トランスクルードしている側の保存済みの写しも更新される。
#[tokio::test]
async fn editing_a_source_item_updates_the_stored_copy_in_its_transcluders() {
    let source_page = new_doc_page("items-transclusion-edit-source").await;
    let source = fill(&source_page.id, &["before"]).await.remove(0);
    let page = new_doc_page("items-transclusion-edit").await;
    let host = fill(&page.id, &["placeholder"]).await.remove(0);
    let (status, resp) = post(
        &format!("/api/items/{}/transclude", host.id),
        json!({ "sourceItemId": source.id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    update_item(&source.id, json!({ "body": "after" })).await;

    // The exported file of the host page is built from the stored copy, not from the source
    assert_eq!(items(&page.id).await[0].body, "after");
    let exported: Value = client()
        .post(url("/api/items/export-page"))
        .query(&[("pageId", page.id.as_str())])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        exported["flatMarkdown"].as_str().unwrap().contains("after"),
        "{}",
        exported["flatMarkdown"]
    );
}

// 参照元の木を削除すると、トランスクルージョンは解除され、そのときの子孫もトランスクルードしている側に普通の項目として残る。
// 子孫の本文、AST、折りたたみ、型、ハッシュタグ、入れ子のトランスクルージョンは、そのまま写される。
#[tokio::test]
async fn deleting_a_source_item_leaves_its_descendants_as_a_copy_in_its_transcluders() {
    let other_page = new_doc_page("items-transclusion-delete-other").await;
    let other = fill(&other_page.id, &["other"]).await.remove(0);
    let source_page = new_doc_page("items-transclusion-delete-source").await;
    let source = fill(
        &source_page.id,
        &["root", "  # child #kept", "    grandchild"],
    )
    .await;
    update_item(&source[1].id, json!({ "collapsed": true })).await;
    let (status, resp) = post(
        &format!("/api/items/{}/transclude", source[2].id),
        json!({ "sourceItemId": other.id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let page = new_doc_page("items-transclusion-delete").await;
    let host = fill(&page.id, &["intro", "  host", "    own child", "after"]).await;
    let (status, resp) = post(
        &format!("/api/items/{}/transclude", host[1].id),
        json!({ "sourceItemId": source[0].id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let (_, resp) = get(&format!("/api/pages/{}", source_page.id)).await;
    let child_ast = field_of_each(&resp, "items", "ast").remove(1);

    // The whole tree goes in one group: the copies are made before any of it is deleted
    let (status, resp) = post(
        "/api/items/delete",
        json!({ "itemIds": [source[0].id, source[1].id, source[2].id] }),
    )
    .await;
    assert_eq!(status, 204, "{resp}");

    // The copies follow the host item and come before its own children
    assert_eq!(
        outline_of(&page.id).await,
        [
            "1:intro",
            "2:root",
            "3:# child #kept",
            "4:other",
            "3:own child",
            "1:after"
        ]
    );
    let copies = items(&page.id).await;
    assert!(copies[1].transclude_item_id.is_none());
    assert_eq!(copies[2].types, ["heading"]);
    assert!(copies[2].collapsed);
    assert_eq!(copies[2].properties["tags"], json!(["kept"]));
    assert_eq!(
        copies[3].transclude_item_id.as_deref(),
        Some(other.id.as_str())
    );
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(field_of_each(&resp, "items", "ast")[2], child_ast);
}

// ---- Items: merge ----

// 結合すると、消える項目の本文が受け取る項目の末尾につながり、消える項目の子は受け取る項目の子になる。
#[tokio::test]
async fn merging_appends_the_body_and_hands_the_children_to_the_receiving_item() {
    let page = new_doc_page("items-merge").await;
    let rows = fill(
        &page.id,
        &["first", "  first child", "second", "  second child"],
    )
    .await;

    let (status, resp) = post(
        "/api/items/merge",
        json!({ "itemId": rows[2].id, "intoId": rows[0].id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(resp["item"]["body"], json!("firstsecond"));
    assert_eq!(
        outline_of(&page.id).await,
        ["1:firstsecond", "2:first child", "2:second child"]
    );
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(
        field_of_each(&resp, "items", "parentItemId")[2],
        json!(rows[0].id)
    );
}

// トランスクルージョンは結合できない。消える側でも受け取る側でも断られる。
#[tokio::test]
async fn a_transclusion_is_not_merged_on_either_side() {
    let source_page = new_doc_page("items-merge-refuse-source").await;
    let source = fill(&source_page.id, &["source"]).await.remove(0);
    let page = new_doc_page("items-merge-refuse").await;
    let rows = fill(&page.id, &["plain", "host", "below"]).await;
    let (status, resp) = post(
        &format!("/api/items/{}/transclude", rows[1].id),
        json!({ "sourceItemId": source.id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (status, _) = post(
        "/api/items/merge",
        json!({ "itemId": rows[1].id, "intoId": rows[0].id }),
    )
    .await;
    assert_eq!(status, 400);
    let (status, _) = post(
        "/api/items/merge",
        json!({ "itemId": rows[2].id, "intoId": rows[1].id }),
    )
    .await;
    assert_eq!(status, 400);
    assert_eq!(
        outline_of(&page.id).await,
        ["1:plain", "1:source", "1:below"]
    );
}

// トランスクルードされている項目を結合で消すと、埋め込み側には本文と子孫の写しが普通の項目として残る。
#[tokio::test]
async fn merging_away_a_source_item_leaves_a_copy_in_its_transcluders() {
    let source_page = new_doc_page("items-merge-source").await;
    let source = fill(&source_page.id, &["above", "source", "  child"]).await;
    let page = new_doc_page("items-merge-host").await;
    let host = fill(&page.id, &["host"]).await.remove(0);
    let (status, resp) = post(
        &format!("/api/items/{}/transclude", host.id),
        json!({ "sourceItemId": source[1].id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (status, resp) = post(
        "/api/items/merge",
        json!({ "itemId": source[1].id, "intoId": source[0].id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(
        outline_of(&source_page.id).await,
        ["1:abovesource", "2:child"]
    );
    assert_eq!(outline_of(&page.id).await, ["1:source", "2:child"]);
    assert!(items(&page.id)
        .await
        .iter()
        .all(|i| i.transclude_item_id.is_none()));
}

// 受け取る項目がトランスクルードされていれば、結合した本文が埋め込み側の写しにも入る。
#[tokio::test]
async fn merging_into_a_transcluded_item_updates_the_stored_copy_in_its_transcluders() {
    let page = new_doc_page("items-merge-into-source").await;
    let rows = fill(&page.id, &["first", "second"]).await;
    let host_page = new_doc_page("items-merge-into-host").await;
    let host = fill(&host_page.id, &["placeholder"]).await.remove(0);
    let (status, resp) = post(
        &format!("/api/items/{}/transclude", host.id),
        json!({ "sourceItemId": rows[0].id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let (_, resp) = get(&format!("/api/pages/{}", host_page.id)).await;
    let modified_before = field_of_each(&resp, "items", "modifiedDate").remove(0);
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let (status, resp) = post(
        "/api/items/merge",
        json!({ "itemId": rows[1].id, "intoId": rows[0].id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(items(&host_page.id).await[0].body, "firstsecond");
    // The copy carries the parsed form too, and counts as changed, so a sync by date picks it up
    let (_, source) = get(&format!("/api/pages/{}", page.id)).await;
    let (_, resp) = get(&format!("/api/pages/{}", host_page.id)).await;
    assert_eq!(
        field_of_each(&resp, "items", "ast")[0],
        field_of_each(&source, "items", "ast")[0]
    );
    let modified_after = field_of_each(&resp, "items", "modifiedDate").remove(0);
    assert!(
        modified_after.as_i64() > modified_before.as_i64(),
        "{modified_before} -> {modified_after}"
    );
}

// ---- Items: group delete ----

// グループ削除は、項目を子孫ごと1回のリクエストで消す。
#[tokio::test]
async fn a_group_of_items_is_deleted_with_its_descendants_in_one_request() {
    let page = new_doc_page("items-group-delete").await;
    let rows = fill(
        &page.id,
        &[
            "keep",
            "parent",
            "  child",
            "    grandchild",
            "sibling",
            "last",
        ],
    )
    .await;

    let (status, resp) = post(
        "/api/items/delete",
        json!({ "itemIds": [rows[1].id, rows[2].id, rows[3].id, rows[4].id] }),
    )
    .await;
    assert_eq!(status, 204, "{resp}");

    assert_eq!(outline_of(&page.id).await, ["1:keep", "1:last"]);
}

// 子孫がそろっていないグループは断られ、何も消えない。
#[tokio::test]
async fn a_group_that_leaves_a_descendant_behind_is_refused_and_nothing_is_deleted() {
    let page = new_doc_page("items-group-delete-refuse").await;
    let rows = fill(
        &page.id,
        &["parent", "  child", "    grandchild", "sibling"],
    )
    .await;

    let (status, _) = post(
        "/api/items/delete",
        json!({ "itemIds": [rows[0].id, rows[1].id, rows[3].id] }),
    )
    .await;
    assert_eq!(status, 400);
    let (status, _) = post("/api/items/delete", json!({ "itemIds": [] })).await;
    assert_eq!(status, 400);

    assert_eq!(
        outline_of(&page.id).await,
        ["1:parent", "2:child", "3:grandchild", "1:sibling"]
    );
}

// 単体の削除は、子を持つ項目を断る。子のない項目は消える。
#[tokio::test]
async fn a_single_delete_refuses_an_item_with_children() {
    let page = new_doc_page("items-single-delete").await;
    let rows = fill(&page.id, &["parent", "  child"]).await;

    assert_eq!(delete(&format!("/api/items/{}", rows[0].id)).await, 400);
    assert_eq!(outline_of(&page.id).await, ["1:parent", "2:child"]);

    assert_eq!(delete(&format!("/api/items/{}", rows[1].id)).await, 204);
    assert_eq!(delete(&format!("/api/items/{}", rows[0].id)).await, 204);
    assert_eq!(outline_of(&page.id).await, Vec::<String>::new());
}

// ---- Pages ----

// 新しいdocページは空の項目1つで始まり、同じタイトルで再び見つかる。
#[tokio::test]
async fn a_new_doc_page_starts_with_one_empty_item_and_is_found_again_by_its_title() {
    let title = format!(
        "pages-new-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );

    let (_, first) = post("/api/pages/doc", json!({ "title": title, "dirPath": "" })).await;
    let (_, again) = post("/api/pages/doc", json!({ "title": title, "dirPath": "" })).await;

    assert_eq!(first["created"], json!(true));
    assert_eq!(field_of_each(&first, "items", "body"), vec![json!("")]);
    assert_eq!(first["page"]["type"], json!("doc"));
    assert_eq!(again["created"], json!(false));
    assert_eq!(first["page"]["id"], again["page"]["id"]);
}

// ページの取得は、ページとその項目を順番どおりに返す。
#[tokio::test]
async fn get_page_returns_the_page_with_its_items_in_order() {
    let page = new_doc_page("pages-get").await;
    fill(&page.id, &["one", "  two", "three"]).await;

    let (status, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(status, 200);

    assert_eq!(resp["page"]["name"], json!(page.title));
    assert_eq!(
        field_of_each(&resp, "items", "body"),
        vec![json!("one"), json!("two"), json!("three")]
    );
    assert_eq!(
        field_of_each(&resp, "items", "level"),
        vec![json!(1), json!(2), json!(1)]
    );
}

// ページの改名で、タイトルとパスが変わる。
#[tokio::test]
async fn renaming_a_page_changes_its_title_and_path() {
    let page = new_doc_page("pages-rename").await;
    let old = date_the_page_in_the_past(&page.id).await;

    let (status, resp) = put(
        &format!("/api/pages/{}", page.id),
        json!({ "title": format!("{}-renamed", page.title), "dirPath": "notes/" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(
        resp["page"]["name"],
        json!(format!("{}-renamed", page.title))
    );
    assert_eq!(resp["page"]["dirPath"], json!("notes/"));
    assert_ne!(resp["page"]["modifiedDate"], json!(old));
    let (_, again) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(
        again["page"]["name"],
        json!(format!("{}-renamed", page.title))
    );
}

/// Gives the page a modified date in the past, so a later change shows as a new date.
async fn date_the_page_in_the_past(page_id: &str) -> i64 {
    let (status, resp) = put(
        &format!("/api/pages/{page_id}/properties/modified"),
        json!({ "valueType": "datetime", "value": "2021-02-03T04:05:06Z" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    resp["page"]["modifiedDate"].as_i64().unwrap()
}

// ページの下にページを作ると、そのページはディレクトリのindexページになり、変更日が新しくなる。
#[tokio::test]
async fn a_page_becomes_the_index_page_of_its_directory_when_a_page_is_made_under_it() {
    let root = new_doc_page("pages-promote").await;
    let old = date_the_page_in_the_past(&root.id).await;

    let (status, resp) = post(
        "/api/pages/doc",
        json!({ "title": "child", "dirPath": format!("{}/", root.title) }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (_, resp) = get(&format!("/api/pages/{}", root.id)).await;
    assert_eq!(resp["page"]["name"], json!("index"));
    assert_eq!(resp["page"]["dirPath"], json!(format!("{}/", root.title)));
    assert_ne!(resp["page"]["modifiedDate"], json!(old));
}

// 子ページを持つindexページは、普通のページに改名できず、削除もできない。
#[tokio::test]
async fn an_index_page_with_children_is_neither_renamed_to_a_plain_page_nor_deleted() {
    let root = new_doc_page("pages-index-guard").await;
    let (status, resp) = post(
        "/api/pages/doc",
        json!({ "title": "child", "dirPath": format!("{}/", root.title) }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (status, resp) = put(
        &format!("/api/pages/{}", root.id),
        json!({ "title": "plain" }),
    )
    .await;
    assert_eq!(status, 400, "{resp}");
    assert_eq!(delete(&format!("/api/pages/{}", root.id)).await, 400);
}

// indexページを動かすと、その下のページも一緒に動く。
#[tokio::test]
async fn moving_an_index_page_moves_the_pages_under_it() {
    // Making a page under it turns this page into the index page of its directory
    let root = new_doc_page("pages-move-dir").await;
    let dir = format!("{}/", root.title);
    let (status, resp) = post(
        "/api/pages/doc",
        json!({ "title": "deep", "dirPath": format!("{dir}sub/") }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let deep_id = resp["page"]["id"].as_str().unwrap().to_string();

    let moved_dir = format!("{}-moved/", root.title);
    let (status, resp) = put(
        &format!("/api/pages/{}", root.id),
        json!({ "title": "index", "dirPath": moved_dir }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (_, deep) = get(&format!("/api/pages/{deep_id}")).await;
    assert_eq!(deep["page"]["dirPath"], json!(format!("{moved_dir}sub/")));
}

// 別のページと同じ名前への改名は、上書きを指定しない限り拒否される。
#[tokio::test]
async fn renaming_to_the_name_of_another_page_is_refused_unless_it_is_overwritten() {
    let page = new_doc_page("pages-overwrite").await;
    let other = new_doc_page("pages-overwritten").await;
    fill(&page.id, &["mine"]).await;
    fill(&other.id, &["theirs"]).await;

    let (status, resp) = put(
        &format!("/api/pages/{}", page.id),
        json!({ "title": other.title }),
    )
    .await;
    assert_eq!(status, 409, "{resp}");
    assert_eq!(get(&format!("/api/pages/{}", other.id)).await.0, 200);

    let (status, resp) = put(
        &format!("/api/pages/{}", page.id),
        json!({ "title": other.title.to_uppercase(), "overwrite": true }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["name"], json!(other.title.to_uppercase()));
    assert_eq!(get(&format!("/api/pages/{}", other.id)).await.0, 404);
    assert!(outline_of(&page.id)
        .await
        .iter()
        .any(|line| line.ends_with(":mine")));
}

// 最近のページでは、最後に触ったページが先頭に来る。
#[tokio::test]
async fn the_most_recently_touched_page_comes_first_in_recent_pages() {
    let older = new_doc_page("pages-recent-older").await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let newer = new_doc_page("pages-recent-newer").await;

    // Other tests create pages at the same time, so only the relative order is checked
    let (status, resp) = get("/api/pages/recent?maxResults=2000&type=doc").await;
    assert_eq!(status, 200);
    let ids: Vec<Value> = field_of_each(&resp, "pages", "id");

    let pos = |id: &str| ids.iter().position(|v| v == &json!(id));
    assert!(pos(&newer.id).unwrap() < pos(&older.id).unwrap(), "{ids:?}");
}

// 削除したページは見つからなくなる。
#[tokio::test]
async fn a_deleted_page_is_gone() {
    let page = new_doc_page("pages-delete").await;

    assert_eq!(delete(&format!("/api/pages/{}", page.id)).await, 204);

    assert_eq!(get(&format!("/api/pages/{}", page.id)).await.0, 404);
}

// ページへのウィキリンクは、そのページのバックリンクに現れる。
#[tokio::test]
async fn a_wiki_link_to_a_page_shows_up_in_its_backlinks() {
    let target = new_doc_page("pages-target").await;
    let source = new_doc_page("pages-source").await;
    fill(&source.id, &[&format!("see [[{}]]", target.title)]).await;

    let (status, resp) = get(&format!("/api/pages/{}/backlinks", target.id)).await;
    assert_eq!(status, 200);

    assert_eq!(
        resp["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["sourcePage"]["id"].clone())
            .collect::<Vec<_>>(),
        vec![json!(source.id)]
    );
    assert_eq!(
        resp["entries"][0]["sourceItem"]["body"],
        json!(format!("see [[{}]]", target.title))
    );
}

// タグページのバックリンクには、そのタグを使うページと項目が並ぶ。
#[tokio::test]
async fn a_tag_page_lists_the_pages_and_items_that_use_its_tag() {
    let _tags_dir = TAGS_DIR.lock().await;
    let tag = "backlink-tag";
    let (_, config) = get("/api/config").await;
    let tag_path = format!("{}{tag}", config["tagsDir"].as_str().unwrap());
    let by_page_tag = new_doc_page("backlinks-by-page-tag").await;
    set_page_tags(&by_page_tag.id, tag).await;
    let target = doc_page_named(tag).await.unwrap();
    let by_items = new_doc_page("backlinks-by-items").await;
    let rows = fill(
        &by_items.id,
        &[
            &format!("a hashtag #{}", tag.to_uppercase()),
            "no link here",
            &format!("a wikilink [[{tag_path}#x|label]]"),
            &format!("a longer path [[{tag_path}/more]]"),
        ],
    )
    .await;

    let (status, resp) = get(&format!("/api/pages/{target}/backlinks")).await;
    assert_eq!(status, 200, "{resp}");
    let entries: Vec<(Value, Value)> = resp["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["sourcePage"]["id"].clone(), e["sourceItem"]["id"].clone()))
        .collect();
    // The newest source page first; its items in page order
    assert_eq!(
        entries,
        vec![
            (json!(by_items.id), json!(rows[0].id)),
            (json!(by_items.id), json!(rows[2].id)),
            (json!(by_page_tag.id), Value::Null),
        ]
    );
}

// ---- Journals (1999 so that nothing else is near) ----

async fn journal(name: &str) -> Value {
    let (status, resp) = post_empty(&format!("/api/journals/{name}")).await;
    assert_eq!(status, 200, "{resp}");
    resp
}

// 日のジャーナルは空の項目1つで作られ、同じ日付で再び見つかる。
#[tokio::test]
async fn a_day_is_created_with_one_empty_item_and_found_again_by_its_date() {
    let first = journal("1999-06-15").await;
    let again = journal("1999-06-15").await;

    assert_eq!(first["created"], json!(true));
    assert_eq!(first["page"]["type"], json!("journal"));
    assert_eq!(first["page"]["name"], json!("1999-06-15"));
    assert_eq!(field_of_each(&first, "items", "body"), vec![json!("")]);
    assert_eq!(again["created"], json!(false));
    assert_eq!(first["page"]["id"], again["page"]["id"]);
}

// ある日付より前の、直近の日のジャーナルが見つかる。
#[tokio::test]
async fn the_most_recent_day_before_a_date_is_found() {
    journal("1999-03-05").await;
    journal("1999-03-10").await;

    let (_, before) = get("/api/journals/before?date=1999-03-10").await;
    let (_, nothing) = get("/api/journals/before?date=1999-01-01").await;

    assert_eq!(before["found"], json!(true));
    assert_eq!(before["page"]["name"], json!("1999-03-05"));
    assert_eq!(before["items"].as_array().unwrap().len(), 1);
    assert_eq!(nothing["found"], json!(false));
}

// 週のジャーナルは、1日に1つのリンクを持って作られる。
#[tokio::test]
async fn a_week_is_created_with_one_link_per_day() {
    let week = journal("1999-W10").await;

    assert_eq!(week["created"], json!(true));
    let bodies = field_of_each(&week, "items", "body");
    assert_eq!(bodies.len(), 7);
    assert_eq!(bodies[0], json!("[[1999-03-08]]")); // ISO week 10 of 1999 starts on Monday the 8th
    assert_eq!(bodies[6], json!("[[1999-03-14]]"));
}

// 米国式の週番号で閏年が土曜に始まると週54が出るので、週54のジャーナルも作られる。
#[tokio::test]
async fn a_week_54_is_created_as_the_us_numbering_reaches_it_in_a_leap_year_starting_on_saturday() {
    let week = journal("2028-W54").await;

    assert_eq!(week["created"], json!(true));
    assert_eq!(week["page"]["name"], json!("2028-W54"));
    assert_eq!(field_of_each(&week, "items", "body").len(), 7);
}

// 月のジャーナルは、その月にかかる週に1つずつのリンクを持って作られる。
#[tokio::test]
async fn a_month_is_created_with_one_link_per_week_that_touches_it() {
    let month = journal("1999-03").await;

    assert_eq!(month["created"], json!(true));
    let bodies = field_of_each(&month, "items", "body");
    // March 1999 runs from ISO week 9 (Monday the 1st) to week 13 (Wednesday the 31st)
    assert_eq!(bodies.len(), 5);
    assert_eq!(bodies[0], json!("[[1999-W09]]"));
    assert_eq!(bodies[4], json!("[[1999-W13]]"));
}

// 年のジャーナルは、1か月に1つのリンクを持って作られる。
#[tokio::test]
async fn a_year_is_created_with_one_link_per_month() {
    let year = journal("1999").await;

    assert_eq!(year["created"], json!(true));
    let bodies = field_of_each(&year, "items", "body");
    assert_eq!(bodies.len(), 12);
    assert_eq!(bodies[0], json!("[[1999-01]]"));
    assert_eq!(bodies[11], json!("[[1999-12]]"));
}

// ある週より前の、直近の週のジャーナルが見つかる。
#[tokio::test]
async fn the_most_recent_week_before_a_week_is_found() {
    journal("1999-W20").await;
    journal("1999-W22").await;

    let (_, before) = get("/api/journals/weekly/before?week=1999-W22").await;

    assert_eq!(before["found"], json!(true));
    assert_eq!(before["page"]["name"], json!("1999-W20"));
}

// 項目は別の日のジャーナルへ移動する。
#[tokio::test]
async fn an_item_moves_to_another_day() {
    let from = journal("1999-09-01").await["page"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let to = journal("1999-09-02").await["page"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let moved = fill(&from, &["carry over"]).await.remove(0);

    move_items(&[&moved.id], &to, "last", None).await;

    assert_eq!(outline_of(&from).await, Vec::<String>::new());
    assert_eq!(outline_of(&to).await, vs(&["1:", "1:carry over"]));
}

// ---- Search ----

async fn search(path: &str, query: &str) -> Value {
    let (status, resp) = client()
        .get(url(path))
        .query(&[("q", query), ("maxResults", "10")])
        .send()
        .await
        .map(|r| (r.status().as_u16(), r))
        .unwrap()
        .1
        .json::<Value>()
        .await
        .map(|v| (200u16, v))
        .unwrap();
    assert_eq!(status, 200);
    resp
}

fn nanos() -> i64 {
    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
}

// ページ名は前方一致で見つかる。
#[tokio::test]
async fn page_names_are_found_by_prefix() {
    let page = new_doc_page("Searchable").await;

    let resp = search("/api/search/pages", &page.title[..12]).await;
    let titles = field_of_each(&resp, "results", "pageTitle");

    assert!(titles.contains(&json!(page.title)), "titles: {titles:?}");
}

// 項目の本文は、抜粋付きで見つかる。
#[tokio::test]
async fn item_text_is_found_with_a_snippet() {
    let page = new_doc_page("search-text").await;
    let word = format!("zebra{}", nanos());
    let item = fill(&page.id, &[&format!("the {word} crossing"), "nothing here"])
        .await
        .remove(0);

    let resp = search("/api/search/combined", &word).await;

    assert_eq!(
        field_of_each(&resp, "results", "itemId"),
        vec![json!(item.id)]
    );
    assert_eq!(resp["results"][0]["pageId"], json!(page.id));
    // The analyzer cuts letters from digits, so the match is marked word by word
    let snippet = resp["results"][0]["snippet"].as_str().unwrap_or("");
    assert!(snippet.contains("<mark>zebra"), "{snippet}");
}

// 2語の検索では、両方の語を含む項目だけが見つかる。
#[tokio::test]
async fn two_words_must_both_be_in_the_item() {
    let page = new_doc_page("search-and").await;
    let a = format!("apple{}", nanos());
    let b = format!("banana{}", nanos());
    let rows = fill(&page.id, &[&format!("{a} and {b}"), &format!("{a} alone")]).await;

    let resp = search("/api/search/combined", &format!("{a} {b}")).await;

    assert_eq!(
        field_of_each(&resp, "results", "itemId"),
        vec![json!(rows[0].id)]
    );
}

// 本文のハッシュタグはタグとして見つかる。
#[tokio::test]
async fn a_hashtag_in_the_body_is_found_as_a_tag() {
    let page = new_doc_page("search-tag").await;
    let tag = format!("tag{}", nanos());
    let item = fill(&page.id, &[&format!("remember #{tag}")])
        .await
        .remove(0);

    assert_eq!(items(&page.id).await[0].properties["tags"], json!([tag]));
    let resp = search("/api/search/tags", &tag).await;
    assert_eq!(
        field_of_each(&resp, "results", "itemId"),
        vec![json!(item.id)]
    );
}

// タグは大文字と小文字を区別せずに見つかる。
#[tokio::test]
async fn a_tag_is_found_in_any_case_of_its_letters() {
    let page = new_doc_page("search-tag-case").await;
    let tag = format!("CaseTag{}", nanos());
    let item = fill(&page.id, &[&format!("remember #{tag}")])
        .await
        .remove(0);

    let resp = search("/api/search/tags", &tag.to_lowercase()).await;
    assert_eq!(
        field_of_each(&resp, "results", "itemId"),
        vec![json!(item.id)]
    );
    let resp = search("/api/search/combined", &format!("#{}", tag.to_uppercase())).await;
    assert_eq!(
        field_of_each(&resp, "results", "itemId"),
        vec![json!(item.id)]
    );
}

// 削除した項目は見つからなくなる。
#[tokio::test]
async fn a_deleted_item_is_no_longer_found() {
    let page = new_doc_page("search-deleted").await;
    let word = format!("ghost{}", nanos());
    let item = fill(&page.id, &[&word, "stays"]).await.remove(0);

    assert_eq!(delete(&format!("/api/items/{}", item.id)).await, 204);

    let resp = search("/api/search/combined", &word).await;
    assert_eq!(
        field_of_each(&resp, "results", "itemId"),
        Vec::<Value>::new()
    );
}

// ---- Markdown conversions ----

async fn parse(markdown: &str) -> Value {
    let (status, resp) = post("/api/items/parse", json!({ "markdown": markdown })).await;
    assert_eq!(status, 200, "{resp}");
    resp
}

// 平らなMarkdownは、ブロックの境目で項目に分かれる。
#[tokio::test]
async fn flat_markdown_is_split_into_items_at_block_boundaries() {
    let resp = parse("# Night\n\n- [ ] Hanon\n\n- [ ] Upload\n").await;

    assert_eq!(
        field_of_each(&resp, "items", "body"),
        vec![json!("# Night"), json!("- [ ] Hanon\n\n- [ ] Upload")]
    );
    assert_eq!(
        field_of_each(&resp, "items", "level"),
        vec![json!(1), json!(1)]
    );
}

// メタデータの行が各項目のレベルを決める。
#[tokio::test]
async fn metadata_lines_carry_the_level_of_each_item() {
    let resp = parse("# Night\n\n<!-- @@ -->\n- [ ] Hanon\n\n<!-- @@ -->\n- [ ] Upload\n").await;

    assert_eq!(
        field_of_each(&resp, "items", "body"),
        vec![
            json!("# Night"),
            json!("- [ ] Hanon"),
            json!("- [ ] Upload")
        ]
    );
    assert_eq!(
        field_of_each(&resp, "items", "level"),
        vec![json!(1), json!(2), json!(2)]
    );
}

// エクスポートした項目は、インポートと解析を経て元のまま戻る。
#[tokio::test]
async fn exported_items_come_back_through_import_and_parse_unchanged() {
    let page = new_doc_page("markdown-roundtrip").await;
    let rows = fill(
        &page.id,
        &[
            "parent",
            "  child one",
            "    grandchild",
            "  child two",
            "sibling",
        ],
    )
    .await;
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();

    let (status, exported) = post("/api/items/export", json!({ "itemIds": ids })).await;
    assert_eq!(status, 200, "{exported}");
    let tree = exported["treeMarkdown"].as_str().unwrap();
    assert!(tree.contains("- parent"), "{tree}");
    assert!(tree.contains("    - grandchild"), "{tree}");
    assert!(exported["ast"].as_str().unwrap().contains("\"type\""));
    assert!(
        exported["flatMarkdown"]
            .as_str()
            .unwrap()
            .contains("<!-- @"),
        "flat markdown keeps the item metadata"
    );

    let (_, imported) = post("/api/items/import", json!({ "treeMarkdown": tree })).await;
    let parsed = parse(imported["flatMarkdown"].as_str().unwrap()).await;

    assert_eq!(
        field_of_each(&parsed, "items", "body"),
        rows.iter().map(|r| json!(r.body)).collect::<Vec<_>>()
    );
    assert_eq!(
        field_of_each(&parsed, "items", "level"),
        rows.iter().map(|r| json!(r.level)).collect::<Vec<_>>()
    );
}

// ページ全体のエクスポートは、その項目のエクスポートと同じになる。
#[tokio::test]
async fn a_whole_page_exports_like_its_items() {
    let page = new_doc_page("markdown-page").await;
    let rows = fill(&page.id, &["one", "  two"]).await;
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();

    let by_page: Value = client()
        .post(url("/api/items/export-page"))
        .query(&[("pageId", page.id.as_str())])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let (_, by_items) = post("/api/items/export", json!({ "itemIds": ids })).await;

    assert_eq!(by_items["treeMarkdown"], by_page["treeMarkdown"]);
}

// ---- Folder sync ----

fn file_of(page: &PageRef) -> std::path::PathBuf {
    server()
        .markdown_dir
        .join("docs")
        .join(format!("{}.md", page.title))
}

fn read(file: &std::path::Path) -> String {
    std::fs::read_to_string(file).unwrap_or_default()
}

/// Writes the file of the page until the condition holds. The test writes this one page, not
/// the export queue. A run of the queue does nothing while another test runs it, and that run
/// may have written the page before the last change. The file then passed the condition with
/// an older page, and the scan after the edit found the page newer than the file.
async fn wait_until(page: &PageRef, what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        let state = server().state.clone();
        let id = hmtid::Hmtid::parse(&page.id).unwrap();
        tokio::task::spawn_blocking(move || state.export.export_page(&id))
            .await
            .unwrap()
            .unwrap();
        if condition() {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting until {what}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// Held by the tests that edit files and scan the folder. A scan imports every changed file,
/// so it would take the change another test is about to look for in its own scan.
static FOLDER_SCAN: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn scan() -> Value {
    let (status, resp) = post_empty("/api/sync/scan").await;
    assert_eq!(status, 200, "{resp}");
    resp
}

// 同期の状態は、フォルダの場所と、それがあることを返す。
#[tokio::test]
async fn the_status_names_the_folder_and_says_it_is_there() {
    let (status, resp) = get("/api/sync/status").await;
    assert_eq!(status, 200);

    assert_eq!(resp["importEnabled"], json!(true));
    assert_eq!(
        resp["markdownRoot"],
        json!(server().markdown_dir.to_string_lossy())
    );
    assert_eq!(resp["rootAvailable"], json!(true));
}

// ページはMarkdownファイルに書き出される。
#[tokio::test]
async fn a_page_is_written_to_its_markdown_file() {
    let page = new_doc_page("sync-export").await;
    fill(&page.id, &["first line", "  second line"]).await;

    wait_until(&page, "the page is exported", || {
        read(&file_of(&page)).contains("second line")
    })
    .await;

    let markdown = read(&file_of(&page));
    assert!(
        markdown.starts_with(&format!("---\nid: {}\n", page.id)),
        "{markdown}"
    );
    assert!(markdown.contains("first line"), "{markdown}");
}

// ファイルへの変更はページに取り込まれる。
#[tokio::test]
async fn a_change_made_to_the_file_is_merged_into_the_page() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("sync-import").await;
    fill(&page.id, &["before the edit"]).await;
    wait_until(&page, "the page is exported", || {
        read(&file_of(&page)).contains("before the edit")
    })
    .await;

    std::fs::write(
        file_of(&page),
        read(&file_of(&page)).replace("before the edit", "edited on disk, a bit longer"),
    )
    .unwrap();
    let result = scan().await;

    assert!(result["imported"].as_i64().unwrap() >= 1, "{result}");
    assert_eq!(
        outline_of(&page.id).await,
        vs(&["1:edited on disk, a bit longer"])
    );
}

// フォルダに置いた新しいファイルはページになる。
#[tokio::test]
async fn a_new_file_in_the_folder_becomes_a_page() {
    let _folder = FOLDER_SCAN.lock().await;
    let name = format!("sync-new-{}", nanos());
    let docs = server().markdown_dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join(format!("{name}.md")),
        "hello from disk\n\nsecond block\n",
    )
    .unwrap();

    let result = scan().await;

    assert!(result["imported"].as_i64().unwrap() >= 1, "{result}");
    let resp = search("/api/search/pages", &name).await;
    let ids = field_of_each(&resp, "results", "pageId");
    assert_eq!(
        ids.len(),
        1,
        "the new page is found by its file name: {resp}"
    );
    assert_eq!(
        outline_of(ids[0].as_str().unwrap()).await,
        vs(&["1:hello from disk", "1:second block"])
    );
}

// 旧い形のインデックスのファイルは、付け直して書き戻される。
#[tokio::test]
async fn a_file_with_indexes_of_the_old_scheme_is_reindexed_and_written_back() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("sync-old-idx").await;
    fill(&page.id, &["one", "two"]).await;
    wait_until(&page, "the page is exported", || {
        read(&file_of(&page)).contains("two")
    })
    .await;

    // As the server of old wrote it, with a line someone added by hand
    let old = read(&file_of(&page))
        .replace("idx=\"a0\"", "idx=\"z0\"")
        .replace("idx=\"a1\"", "idx=\"z1\"")
        + "\nthree\n";
    assert!(old.contains("idx=\"z1\""), "{old}");
    std::fs::write(file_of(&page), old).unwrap();
    let result = scan().await;

    assert!(result["imported"].as_i64().unwrap() >= 1, "{result}");
    assert_eq!(
        outline_of(&page.id).await,
        vs(&["1:one", "1:two", "1:three"])
    );
    assert_eq!(idxs_of(&page.id).await, vs(&["a0", "a1", "a2"]));
    wait_until(&page, "the file is written back", || {
        let file = read(&file_of(&page));
        file.contains("three") && !file.contains("idx=\"z")
    })
    .await;
}

// 長くなりすぎたインデックスのファイルは、付け直して書き戻される。
#[tokio::test]
async fn a_file_with_an_index_grown_too_long_is_reindexed_and_written_back() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("sync-long-idx").await;
    fill(&page.id, &["one", "two"]).await;
    wait_until(&page, "the page is exported", || {
        read(&file_of(&page)).contains("two")
    })
    .await;

    let before = idxs_of(&page.id).await;
    let long = format!("{}{}", before[1], "V".repeat(20));
    let grown = read(&file_of(&page)).replace(
        &format!("idx=\"{}\"", before[1]),
        &format!("idx=\"{long}\""),
    );
    assert!(grown.contains(&long), "{grown}");
    std::fs::write(file_of(&page), grown).unwrap();
    let result = scan().await;

    assert!(result["imported"].as_i64().unwrap() >= 1, "{result}");
    assert_eq!(outline_of(&page.id).await, vs(&["1:one", "1:two"]));
    let idxs = idxs_of(&page.id).await;
    assert_eq!(idxs[0], before[0]);
    assert!(idxs[1].len() <= 16, "{idxs:?}");
    wait_until(&page, "the file is written back", || {
        let file = read(&file_of(&page));
        file.contains(&format!("idx=\"{}\"", idxs[1])) && !file.contains(&long)
    })
    .await;
}

// 変わっていないファイルには手を付けない。
#[tokio::test]
async fn an_unchanged_file_is_left_alone() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("sync-unchanged").await;
    fill(&page.id, &["stable"]).await;
    wait_until(&page, "the page is exported", || {
        read(&file_of(&page)).contains("stable")
    })
    .await;
    scan().await;

    // Other tests may be exporting their own pages at the same time; this page itself must stay as it is
    scan().await;

    assert_eq!(outline_of(&page.id).await, vs(&["1:stable"]));
}

// ---- Health and config ----

// ヘルスチェックはokを返し、設定は読み書きできる。
#[tokio::test]
async fn health_is_ok_and_the_config_is_read_and_updated() {
    let (status, resp) = get("/api/health").await;
    assert_eq!(status, 200);
    assert_eq!(resp["status"], json!("ok"));

    let _tags_dir = TAGS_DIR.lock().await;
    let (status, resp) = get("/api/config").await;
    assert_eq!(status, 200);
    assert_eq!(resp["weekNumbering"], json!("iso"));
    assert_eq!(resp["logicalOutdent"], json!(false));
    assert_eq!(resp["tagsDir"], json!("tags/"));

    // The tags directory is stored in the dirPath form, whatever the client typed
    let (status, resp) = put("/api/config", json!({ "tagsDir": "/meta/tags" })).await;
    assert_eq!(status, 200);
    assert_eq!(resp["tagsDir"], json!("meta/tags/"));
    // The root is a value of its own, not the default
    let (_, resp) = put("/api/config", json!({ "tagsDir": "" })).await;
    assert_eq!(resp["tagsDir"], json!(""));
    let (_, resp) = get("/api/config").await;
    assert_eq!(resp["tagsDir"], json!(""));
    put("/api/config", json!({ "tagsDir": "tags" })).await;

    let (status, resp) = get("/api/config/property-definitions").await;
    assert_eq!(status, 200);
    assert_eq!(resp["propertyDefinitions"]["plan"], json!("date"));
}

// ホームリンクはリンク先の形で保存される。
#[tokio::test]
async fn the_home_link_is_kept_as_the_target_of_a_link() {
    let (status, resp) = put("/api/config", json!({ "homeLink": " [[a/Foo]] " })).await;
    assert_eq!(status, 200);
    assert_eq!(resp["homeLink"], json!("a/Foo"));
    let (_, resp) = get("/api/config").await;
    assert_eq!(resp["homeLink"], json!("a/Foo"));
    // Other settings leave it as it is
    let (_, resp) = put("/api/config", json!({ "logicalOutdent": false })).await;
    assert_eq!(resp["homeLink"], json!("a/Foo"));
    let (_, resp) = put("/api/config", json!({ "homeLink": "" })).await;
    assert_eq!(resp["homeLink"], json!(""));
}

// ---- Properties ----

fn definition<'a>(resp: &'a Value, key: &str) -> Option<&'a Value> {
    resp["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["key"] == key)
}

// プロパティ定義にはシステムのキーが並び、ユーザーのキーを追加できる。
#[tokio::test]
async fn property_definitions_list_the_system_keys_and_accept_user_keys() {
    let (status, resp) = get("/api/config/property-definitions").await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["propertyDefinitions"]["zoom"], json!("number"));
    assert_eq!(definition(&resp, "tags").unwrap()["system"], json!(true));
    assert_eq!(definition(&resp, "tags").unwrap()["page"], json!(true));
    assert_eq!(definition(&resp, "zoom").unwrap()["page"], json!(false));
    assert_eq!(
        definition(&resp, "modified").unwrap()["system"],
        json!(true)
    );
    assert_eq!(definition(&resp, "due").unwrap()["system"], json!(true));
    assert_eq!(
        definition(&resp, "cancelled").unwrap()["page"],
        json!(false)
    );
    assert_eq!(definition(&resp, "created").unwrap()["page"], json!(true));
    assert_eq!(
        definition(&resp, "published").unwrap()["system"],
        json!(true)
    );
    assert_eq!(definition(&resp, "published").unwrap()["page"], json!(true));
    assert_eq!(resp["propertyDefinitions"]["published"], json!("datetime"));
    assert_eq!(
        definition(&resp, "atlas_bounds").unwrap()["system"],
        json!(true)
    );
    assert_eq!(
        definition(&resp, "atlas_bounds").unwrap()["page"],
        json!(false)
    );
    assert_eq!(resp["propertyDefinitions"]["atlas_bounds"], json!("bounds"));

    let (status, resp) = post(
        "/api/config/property-definitions",
        json!({ "key": "status", "valueType": "text" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let added = definition(&resp, "status").unwrap();
    assert_eq!(added["valueType"], json!("text"));
    assert_eq!(added["system"], json!(false));
    assert_eq!(added["page"], json!(true));

    for body in [
        json!({ "key": "zoom", "valueType": "text" }),
        json!({ "key": "status", "valueType": "text" }),
        json!({ "key": "has space", "valueType": "text" }),
        json!({ "key": "", "valueType": "text" }),
        json!({ "key": "kind", "valueType": "tags" }),
    ] {
        let (status, resp) = post("/api/config/property-definitions", body.clone()).await;
        assert_eq!(status, 400, "{body} -> {resp}");
    }
}

// 項目からプロパティを削除できる。
#[tokio::test]
async fn a_property_can_be_removed_from_an_item() {
    let page = new_doc_page("items-property-delete").await;
    let item = fill(&page.id, &["task-like"]).await.remove(0);
    put(
        &format!("/api/items/{}/properties/plan", item.id),
        json!({ "valueType": "date", "value": "2026-03-01" }),
    )
    .await;

    let (status, resp) = send(
        reqwest::Method::DELETE,
        &format!("/api/items/{}/properties/plan", item.id),
        None,
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    assert!(resp["item"]["properties"].get("plan").is_none());
    assert!(items(&page.id).await[0].properties.get("plan").is_none());
}

// 項目のプロパティの変更だけでも、ページのファイルが書き直される。
#[tokio::test]
async fn an_item_property_change_alone_rewrites_the_page_file() {
    let page = new_doc_page("items-property-export").await;
    let item = fill(&page.id, &["planned"]).await.remove(0);
    wait_until(&page, "the page file exists", || {
        read(&file_of(&page)).contains("planned")
    })
    .await;

    put(
        &format!("/api/items/{}/properties/plan", item.id),
        json!({ "valueType": "date", "value": "2026-03-01" }),
    )
    .await;

    wait_until(&page, "the plan is in the file", || {
        read(&file_of(&page)).contains("plan=\"2026-03-01\"")
    })
    .await;
}

// ページのプロパティは設定と削除ができ、フロントマターに書き出される。
#[tokio::test]
async fn page_properties_are_set_removed_and_written_to_the_front_matter() {
    let page = new_doc_page("page-properties").await;
    post(
        "/api/config/property-definitions",
        json!({ "key": "owner", "valueType": "text" }),
    )
    .await;

    let (status, resp) = put(
        &format!("/api/pages/{}/properties/owner", page.id),
        json!({ "valueType": "text", "value": "me: now" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["properties"]["owner"], json!("me: now"));

    let (status, resp) = put(
        &format!("/api/pages/{}/properties/tags", page.id),
        json!({ "valueType": "tags", "value": "a, b" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["properties"]["tags"], json!(["a", "b"]));

    // An item-only system key stays off the page
    let (status, _) = put(
        &format!("/api/pages/{}/properties/zoom", page.id),
        json!({ "valueType": "number", "value": "2" }),
    )
    .await;
    assert_eq!(status, 400);

    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(resp["page"]["properties"]["owner"], json!("me: now"));
    assert_eq!(resp["page"]["properties"]["tags"], json!(["a", "b"]));

    wait_until(&page, "the front matter has the properties", || {
        let text = read(&file_of(&page));
        text.contains("owner: \"me: now\"\n") && text.contains("tags:\n  - a\n  - b\n---\n")
    })
    .await;

    let (status, resp) = send(
        reqwest::Method::DELETE,
        &format!("/api/pages/{}/properties/owner", page.id),
        None,
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert!(resp["page"]["properties"].get("owner").is_none());
    assert_eq!(resp["page"]["properties"]["tags"], json!(["a", "b"]));

    wait_until(&page, "the owner left the front matter", || {
        !read(&file_of(&page)).contains("owner:")
    })
    .await;
}

// ページのユーザー定義のlist型のキーは、そのキーのリストとして保存され、タグには触らない。
#[tokio::test]
async fn a_list_of_a_page_is_kept_under_its_own_key_apart_from_the_tags() {
    let _tags_dir = TAGS_DIR.lock().await;
    let page = new_doc_page("page-list").await;
    set_page_tags(&page.id, "page-list-tag").await;

    // The second value replaces the first, as a value of a scalar does
    for value in ["x, y", "y,z"] {
        let (status, resp) = put(
            &format!("/api/pages/{}/properties/page_list", page.id),
            json!({ "valueType": "list", "value": value }),
        )
        .await;
        assert_eq!(status, 200, "{resp}");
    }

    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(resp["page"]["properties"]["page_list"], json!(["y", "z"]));
    assert_eq!(resp["page"]["properties"]["tags"], json!(["page-list-tag"]));
    assert!(doc_page_named("y").await.is_none());
}

/// Held by a test that changes the tag directory, and by the tests of tag pages. A tag page
/// made while the directory is changed would be looked for in the wrong folder later.
static TAGS_DIR: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The id of a doc page by its name.
async fn doc_page_named(name: &str) -> Option<String> {
    let (_, resp) = get("/api/pages/recent?maxResults=2000&type=doc").await;
    resp["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == json!(name))
        .map(|p| p["id"].as_str().unwrap().to_string())
}

async fn set_page_tags(page_id: &str, tags: &str) {
    let (status, resp) = put(
        &format!("/api/pages/{page_id}/properties/tags"),
        json!({ "valueType": "tags", "value": tags }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
}

// 新しいページタグにはタグページが作られ、最後の使用がなくなると消える。
#[tokio::test]
async fn a_new_page_tag_gets_its_tag_page_and_the_last_use_takes_it_away() {
    let _tags_dir = TAGS_DIR.lock().await;
    let tag = "page-tag-pages-new";
    let first = new_doc_page("page-tag-pages-first").await;
    let second = new_doc_page("page-tag-pages-second").await;

    set_page_tags(&first.id, tag).await;
    set_page_tags(&second.id, tag).await;
    assert!(doc_page_named(tag).await.is_some());

    // The second page still uses the tag
    set_page_tags(&first.id, "page-tag-pages-other").await;
    assert!(doc_page_named(tag).await.is_some());

    let (status, resp) = send(
        reqwest::Method::DELETE,
        &format!("/api/pages/{}/properties/tags", second.id),
        None,
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert!(doc_page_named(tag).await.is_none());
}

/// The ids of the doc pages with this name in any case of its letters.
async fn doc_pages_named_ignoring_case(name: &str) -> Vec<String> {
    let (_, resp) = get("/api/pages/recent?maxResults=2000&type=doc").await;
    resp["pages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["name"].as_str().unwrap().eq_ignore_ascii_case(name))
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect()
}

// 大文字と小文字だけが違うタグは、1つのタグページを共有する。
#[tokio::test]
async fn tags_that_differ_only_in_case_share_one_tag_page() {
    let _tags_dir = TAGS_DIR.lock().await;
    let first = new_doc_page("case-tag-pages-first").await;
    let second = new_doc_page("case-tag-pages-second").await;

    set_page_tags(&first.id, "Case-Tag-Pages").await;
    set_page_tags(&second.id, "case-tag-pages").await;
    assert_eq!(
        doc_pages_named_ignoring_case("case-tag-pages").await.len(),
        1
    );

    // The other case still uses the page
    set_page_tags(&first.id, "case-tag-pages-other").await;
    assert_eq!(
        doc_pages_named_ignoring_case("case-tag-pages").await.len(),
        1
    );

    set_page_tags(&second.id, "case-tag-pages-other").await;
    assert!(doc_pages_named_ignoring_case("case-tag-pages")
        .await
        .is_empty());
}

// 使われなくなっても、内容のあるタグページは残る。
#[tokio::test]
async fn an_unused_tag_page_with_content_stays() {
    let _tags_dir = TAGS_DIR.lock().await;
    let tag = "page-tag-pages-written";
    let page = new_doc_page("page-tag-pages-written").await;
    set_page_tags(&page.id, tag).await;
    let tag_page = doc_page_named(tag).await.unwrap();
    let first = items(&tag_page).await.into_iter().next().unwrap();
    update_item(&first.id, json!({ "body": "a note" })).await;

    set_page_tags(&page.id, "page-tag-pages-written-other").await;
    assert_eq!(doc_page_named(tag).await, Some(tag_page));
}

// 本文のハッシュタグも、ページタグと同じようにタグページを保つ。
#[tokio::test]
async fn a_hashtag_in_a_body_keeps_its_tag_page_as_a_page_tag_does() {
    let _tags_dir = TAGS_DIR.lock().await;
    let tag = "body-tag-pages-new";
    let page = new_doc_page("body-tag-pages").await;
    let item = items(&page.id).await.into_iter().next().unwrap();

    update_item(&item.id, json!({ "body": format!("see #{tag}") })).await;
    assert!(doc_page_named(tag).await.is_some());

    update_item(&item.id, json!({ "body": "no tag now" })).await;
    assert!(doc_page_named(tag).await.is_none());

    let added = create_item(
        &page.id,
        &format!("after:{}", item.id),
        1,
        &format!("#{tag}"),
    )
    .await;
    assert!(doc_page_named(tag).await.is_some());
    assert_eq!(delete(&format!("/api/items/{}", added.id)).await, 204);
    assert!(doc_page_named(tag).await.is_none());
}

// タグページの改名で、ページタグと本文のタグも改名される。
#[tokio::test]
async fn renaming_a_tag_page_renames_its_tag_in_page_tags_and_bodies() {
    let _tags_dir = TAGS_DIR.lock().await;
    let old = format!("renamed-tag-old-{}", nanos());
    let new = format!("renamed-tag-new-{}", nanos());
    let tagged = new_doc_page("renamed-tag-page").await;
    set_page_tags(&tagged.id, &format!("{old},other")).await;
    let page = new_doc_page("renamed-tag-body").await;
    let item = items(&page.id).await.into_iter().next().unwrap();
    update_item(
        &item.id,
        json!({ "body": format!("see #{old} and `#{old}`") }),
    )
    .await;
    let tag_page = doc_page_named(&old).await.unwrap();

    let (status, resp) = put(&format!("/api/pages/{tag_page}"), json!({ "title": new })).await;
    assert_eq!(status, 200, "{resp}");

    let (_, resp) = get(&format!("/api/pages/{}", tagged.id)).await;
    assert_eq!(resp["page"]["properties"]["tags"], json!([new, "other"]));
    let item = items(&page.id).await.into_iter().next().unwrap();
    assert_eq!(item.body, format!("see #{new} and `#{old}`"));
    assert_eq!(item.properties["tags"], json!([new]));
    assert_eq!(doc_page_named(&new).await, Some(tag_page.clone()));
    assert!(doc_page_named(&old).await.is_none());
}

// タグページをタグフォルダの中の別のフォルダへ動かすと、タグもそのフォルダの名前で付け直される。
#[tokio::test]
async fn moving_a_tag_page_into_a_folder_renames_its_tag_after_the_folder() {
    let _tags_dir = TAGS_DIR.lock().await;
    let old = format!("moved-tag-{}", nanos());
    let new = format!("sub/{old}");
    let tagged = new_doc_page("moved-tag-page").await;
    set_page_tags(&tagged.id, &old).await;
    let tag_page = doc_page_named(&old).await.unwrap();
    let (_, config) = get("/api/config").await;
    let tags_dir = config["tagsDir"].as_str().unwrap();

    let (status, resp) = put(
        &format!("/api/pages/{tag_page}"),
        json!({ "dirPath": format!("{tags_dir}sub/") }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (_, resp) = get(&format!("/api/pages/{}", tagged.id)).await;
    assert_eq!(resp["page"]["properties"]["tags"], json!([new]));
}

// タグページをタグが変わらないように更新しても、タグの付いたページは触られない。
#[tokio::test]
async fn updating_a_tag_page_without_changing_its_tag_leaves_the_tagged_pages_alone() {
    let _tags_dir = TAGS_DIR.lock().await;
    let tag = format!("same-tag-{}", nanos());
    let tagged = new_doc_page("same-tag-page").await;
    set_page_tags(&tagged.id, &tag).await;
    let tag_page = doc_page_named(&tag).await.unwrap();
    let (_, before) = get(&format!("/api/pages/{}", tagged.id)).await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let (status, resp) = put(&format!("/api/pages/{tag_page}"), json!({ "title": tag })).await;
    assert_eq!(status, 200, "{resp}");

    let (_, after) = get(&format!("/api/pages/{}", tagged.id)).await;
    assert_eq!(
        after["page"]["modifiedDate"],
        before["page"]["modifiedDate"]
    );
}

// タグページには、タグでないタイトルを付けられない。
#[tokio::test]
async fn a_tag_page_cannot_take_a_title_that_is_not_a_tag() {
    let _tags_dir = TAGS_DIR.lock().await;
    let tag = format!("untitled-tag-{}", nanos());
    let page = new_doc_page("untitled-tag").await;
    set_page_tags(&page.id, &tag).await;
    let tag_page = doc_page_named(&tag).await.unwrap();

    let (status, resp) = put(
        &format!("/api/pages/{tag_page}"),
        json!({ "title": "two words" }),
    )
    .await;
    assert_eq!(status, 400, "{resp}");
    assert_eq!(doc_page_named(&tag).await, Some(tag_page));
}

// ページを削除すると、そのタグのタグページも消える。
#[tokio::test]
async fn a_deleted_page_takes_the_tag_pages_of_its_tags_with_it() {
    let _tags_dir = TAGS_DIR.lock().await;
    let page_tag = "deleted-page-tag-pages-page";
    let body_tag = "deleted-page-tag-pages-body";
    let page = new_doc_page("deleted-page-tag-pages").await;
    set_page_tags(&page.id, page_tag).await;
    let item = items(&page.id).await.into_iter().next().unwrap();
    update_item(&item.id, json!({ "body": format!("#{body_tag}") })).await;
    assert!(doc_page_named(page_tag).await.is_some());
    assert!(doc_page_named(body_tag).await.is_some());

    assert_eq!(delete(&format!("/api/pages/{}", page.id)).await, 204);
    assert!(doc_page_named(page_tag).await.is_none());
    assert!(doc_page_named(body_tag).await.is_none());
}

/// Edits a page file as an outside editor would. The `modified` line goes, so the file's
/// own time makes it newer than the page.
fn edit_file(page: &PageRef, edit: impl Fn(&str) -> String) {
    let text: String = read(&file_of(page))
        .lines()
        .filter(|l| !l.starts_with("modified: "))
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(file_of(page), edit(&text)).unwrap();
}

// 外部で編集したファイルのタグにも、タグページが保たれる。
#[tokio::test]
async fn the_tags_of_a_file_edited_outside_keep_their_tag_pages() {
    let _tags_dir = TAGS_DIR.lock().await;
    let _folder = FOLDER_SCAN.lock().await;
    let page_tag = "file-tag-pages-page";
    let body_tag = "file-tag-pages-body";
    let page = new_doc_page("file-tag-pages").await;
    fill(&page.id, &["before the edit"]).await;
    wait_until(&page, "the page is exported", || {
        read(&file_of(&page)).contains("before the edit")
    })
    .await;

    edit_file(&page, |text| {
        text.replacen(
            "---\n\n",
            &format!("tags:\n  - \"#{page_tag}\"\n---\n\n"),
            1,
        )
        .replace("before the edit", &format!("see #{body_tag}"))
    });
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(resp["page"]["properties"]["tags"], json!([page_tag]));
    assert!(doc_page_named(page_tag).await.is_some());
    assert!(doc_page_named(body_tag).await.is_some());

    edit_file(&page, |text| {
        text.lines()
            .filter(|l| !l.contains(page_tag) && *l != "tags:")
            .map(|l| {
                if l.contains(body_tag) {
                    "no tag now\n".to_string()
                } else {
                    format!("{l}\n")
                }
            })
            .collect()
    });
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert!(resp["page"]["properties"].get("tags").is_none(), "{resp}");
    assert!(doc_page_named(page_tag).await.is_none());
    assert!(doc_page_named(body_tag).await.is_none());
}

// ファイルで直した項目のlist型の値は、項目に取り込まれる。
#[tokio::test]
async fn a_list_of_an_item_edited_in_the_file_is_merged_into_the_item() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("file-item-list").await;
    let item = fill(&page.id, &["listed"]).await.remove(0);
    put(
        &format!("/api/items/{}/properties/item_list", item.id),
        json!({ "valueType": "list", "value": "a, b" }),
    )
    .await;
    wait_until(&page, "the list is in the file", || {
        read(&file_of(&page)).contains("item_list=\"a,b\"")
    })
    .await;

    edit_file(&page, |text| {
        text.replace("item_list=\"a,b\"", "item_list=\"b,c\"")
    });
    scan().await;

    assert_eq!(
        items(&page.id).await[0].properties["item_list"],
        json!(["b", "c"])
    );
}

// ファイルから消した項目のlist型のキーは、項目からも消える。
#[tokio::test]
async fn a_list_of_an_item_removed_from_the_file_goes_from_the_item() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("file-item-list-gone").await;
    let item = fill(&page.id, &["listed"]).await.remove(0);
    put(
        &format!("/api/items/{}/properties/item_list", item.id),
        json!({ "valueType": "list", "value": "a, b" }),
    )
    .await;
    wait_until(&page, "the list is in the file", || {
        read(&file_of(&page)).contains("item_list=\"a,b\"")
    })
    .await;

    edit_file(&page, |text| text.replace(" item_list=\"a,b\"", ""));
    scan().await;

    let properties = &items(&page.id).await[0].properties;
    assert!(properties.get("item_list").is_none(), "{properties:?}");
}

// 古いtagキーで書かれたファイルのタグは失われない。
#[tokio::test]
async fn a_file_written_with_the_old_tag_key_keeps_its_tags() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("file-old-tag-key").await;
    fill(&page.id, &["before the edit"]).await;
    wait_until(&page, "the page is exported", || {
        read(&file_of(&page)).contains("before the edit")
    })
    .await;

    edit_file(&page, |text| {
        text.replacen("---\n\n", "tag:\n  - old-key\n---\n\n", 1)
            .replace("before the edit", "after the edit")
    });
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(resp["page"]["properties"]["tags"], json!(["old-key"]));
    wait_until(&page, "the file has the new key", || {
        read(&file_of(&page)).contains("tags:\n  - old-key\n")
    })
    .await;
}

// ページの日付はプロパティとして設定でき、削除はできない。
#[tokio::test]
async fn the_dates_of_a_page_are_set_as_properties_and_cannot_be_removed() {
    let page = new_doc_page("page-dates").await;

    let (status, resp) = put(
        &format!("/api/pages/{}/properties/created", page.id),
        json!({ "valueType": "datetime", "value": "2020-01-02T03:04:05Z" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["createdDate"], json!(1_577_934_245_000i64));

    let (status, resp) = put(
        &format!("/api/pages/{}/properties/modified", page.id),
        json!({ "valueType": "datetime", "value": "2021-02-03T04:05:06Z" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["modifiedDate"], json!(1_612_325_106_000i64));

    let (status, _) = put(
        &format!("/api/pages/{}/properties/created", page.id),
        json!({ "valueType": "datetime", "value": "yesterday" }),
    )
    .await;
    assert_eq!(status, 400);
    let (status, _) = send(
        reqwest::Method::DELETE,
        &format!("/api/pages/{}/properties/created", page.id),
        None,
    )
    .await;
    assert_eq!(status, 400);

    wait_until(&page, "the front matter has the new created date", || {
        read(&file_of(&page)).contains("created: 2020-01-02T03:04:05Z\n")
    })
    .await;
}

// indexページの子ページは、スカラーのプロパティを持って返る。
#[tokio::test]
async fn the_children_of_an_index_page_carry_their_scalar_properties() {
    // A unique directory so other tests' pages stay out of the listing
    let dir = format!("{}/", new_doc_page("children-props").await.title);
    let (status, resp) = post(
        "/api/pages/doc",
        json!({ "title": "a-page", "dirPath": dir }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let page_id = resp["page"]["id"].as_str().unwrap().to_string();
    let parent_id = resp["page"]["parentId"].as_str().unwrap().to_string();
    let (status, resp) = put(
        &format!("/api/pages/{page_id}/properties/published"),
        json!({ "valueType": "datetime", "value": "2026-10-20T09:00:00Z" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (status, resp) = get(&format!("/api/pages/{parent_id}/children")).await;
    assert_eq!(status, 200, "{resp}");
    let child = resp["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == page_id)
        .unwrap();
    assert_eq!(
        child["properties"]["published"],
        json!("2026-10-20T09:00:00Z")
    );

    let (status, resp) = get("/api/pages/recent?maxResults=50").await;
    assert_eq!(status, 200, "{resp}");
    let recent = resp["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == page_id)
        .unwrap();
    assert_eq!(
        recent["properties"]["published"],
        json!("2026-10-20T09:00:00Z")
    );
}

// indexページの子ページは、タグを持って返る。他のリストのプロパティは返らない。アトラスがカードのアイコンに使うのはタグだけである。
#[tokio::test]
async fn the_children_of_an_index_page_carry_their_tags_but_no_other_list() {
    let _tags_dir = TAGS_DIR.lock().await;
    let dir = format!("{}/", new_doc_page("children-tags").await.title);
    let (status, resp) = post(
        "/api/pages/doc",
        json!({ "title": "a-page", "dirPath": dir }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let page_id = resp["page"]["id"].as_str().unwrap().to_string();
    let parent_id = resp["page"]["parentId"].as_str().unwrap().to_string();
    set_page_tags(&page_id, "children-tag-b, children-tag-a").await;
    let (status, resp) = put(
        &format!("/api/pages/{page_id}/properties/children_list"),
        json!({ "valueType": "list", "value": "x" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (_, resp) = get(&format!("/api/pages/{parent_id}/children")).await;
    let child = resp["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == page_id)
        .unwrap();
    assert_eq!(
        child["properties"]["tags"],
        json!(["children-tag-b", "children-tag-a"])
    );
    assert!(
        child["properties"].get("children_list").is_none(),
        "{}",
        child["properties"]
    );
}

// プロパティ定義を削除すると、ページと項目からその値が消える。
#[tokio::test]
async fn deleting_a_property_definition_removes_its_values_from_pages_and_items() {
    let page = new_doc_page("definition-delete").await;
    let item = fill(&page.id, &["rated"]).await.remove(0);
    let (status, resp) = put(
        &format!("/api/items/{}/properties/rating", item.id),
        json!({ "valueType": "number", "value": "5" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/rating", page.id),
        json!({ "valueType": "number", "value": "3" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["properties"]["rating"], json!("3"));
    let modified_before = resp["page"]["modifiedDate"].as_i64().unwrap();

    let (status, resp) = send(
        reqwest::Method::DELETE,
        "/api/config/property-definitions/rating",
        None,
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert!(definition(&resp, "rating").is_none(), "{resp}");

    assert!(items(&page.id).await[0].properties.get("rating").is_none());
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert!(resp["page"]["properties"].get("rating").is_none(), "{resp}");
    assert!(resp["page"]["modifiedDate"].as_i64().unwrap() >= modified_before);

    // System keys and unknown keys are refused
    let (status, _) = send(
        reqwest::Method::DELETE,
        "/api/config/property-definitions/tags",
        None,
    )
    .await;
    assert_eq!(status, 400);
    let (status, _) = send(
        reqwest::Method::DELETE,
        "/api/config/property-definitions/rating",
        None,
    )
    .await;
    assert_eq!(status, 400);
}

// ---- the atlas page of a directory ----

async fn atlas_of(dir: &str) -> String {
    let (status, resp) = post(
        "/api/pages/doc",
        json!({ "title": "index_atlas", "dirPath": dir }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    resp["page"]["id"].as_str().unwrap().to_string()
}

// アトラスページは、名前で検索しても見つからない。
#[tokio::test]
async fn an_atlas_page_is_not_found_by_its_name() {
    let dir = format!("{}/", new_doc_page("atlas-search").await.title);
    let atlas_id = atlas_of(&dir).await;

    for query in [format!("{dir}index_atlas"), "index_atlas".to_string()] {
        let resp = search("/api/search/pages", &query).await;
        let ids = field_of_each(&resp, "results", "pageId");
        assert!(!ids.contains(&json!(atlas_id)), "{query}: {ids:?}");
    }
}

// アトラスページは改名も移動もできない。
#[tokio::test]
async fn the_atlas_page_cannot_be_renamed_or_moved() {
    let dir = format!("{}/", new_doc_page("atlas-rename").await.title);
    let atlas_id = atlas_of(&dir).await;

    let (status, resp) = put(&format!("/api/pages/{atlas_id}"), json!({ "title": "map" })).await;
    assert_eq!(status, 400, "{resp}");
    let (status, resp) = put(
        &format!("/api/pages/{atlas_id}"),
        json!({ "dirPath": "elsewhere/" }),
    )
    .await;
    assert_eq!(status, 400, "{resp}");
}

// 他のページはアトラスの名前を取れない。
#[tokio::test]
async fn no_other_page_can_take_the_atlas_name() {
    let page = new_doc_page("atlas-reserved").await;

    let (status, resp) = put(
        &format!("/api/pages/{}", page.id),
        json!({ "title": "index_atlas" }),
    )
    .await;
    assert_eq!(status, 400, "{resp}");
}

// boundsのプロパティは4つの数を持つ。
#[tokio::test]
async fn a_bounds_property_holds_four_numbers() {
    let (status, resp) = post(
        "/api/config/property-definitions",
        json!({ "key": "area", "valueType": "bounds" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(
        definition(&resp, "area").unwrap()["valueType"],
        json!("bounds")
    );

    let page = new_doc_page("bounds").await;
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/area", page.id),
        json!({ "valueType": "bounds", "value": " 10, 20.5,160 ,-56 " }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["properties"]["area"], json!("10,20.5,160,-56"));
}

async fn doc_in(dir: &str, title: &str) -> String {
    let (status, resp) = post("/api/pages/doc", json!({ "title": title, "dirPath": dir })).await;
    assert_eq!(status, 200, "{resp}");
    resp["page"]["id"].as_str().unwrap().to_string()
}

/// A directory with the pages `b` and `a` and the folder `z`, and its atlas page.
async fn atlas_dir(prefix: &str) -> (String, String) {
    let dir = format!("{}/", new_doc_page(prefix).await.title);
    doc_in(&dir, "b").await;
    doc_in(&dir, "a").await;
    doc_in(&format!("{dir}z/"), "c").await;
    (dir.clone(), atlas_of(&dir).await)
}

async fn atlas_post(atlas_id: &str, path: &str, body: Value) -> Vec<ItemRow> {
    let (status, resp) = post(&format!("/api/pages/{atlas_id}/atlas/{path}"), body).await;
    assert_eq!(status, 200, "{resp}");
    resp["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(ItemRow::of)
        .collect()
}

async fn atlas_put(atlas_id: &str, path: &str, body: Value) -> Vec<ItemRow> {
    let (status, resp) = put(&format!("/api/pages/{atlas_id}/atlas/{path}"), body).await;
    assert_eq!(status, 200, "{resp}");
    resp["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(ItemRow::of)
        .collect()
}

fn outline(rows: &[ItemRow]) -> Vec<String> {
    rows.iter().map(ItemRow::outline).collect()
}

fn card_of<'a>(rows: &'a [ItemRow], page_id: &str) -> &'a ItemRow {
    rows.iter()
        .find(|r| r.properties.get("atlas_page") == Some(&json!(page_id)))
        .unwrap_or_else(|| panic!("no card of {page_id} in {rows:?}"))
}

// アトラスには、ディレクトリのフォルダが先に、次にページが名前順に並ぶ。
#[tokio::test]
async fn an_atlas_lists_the_folders_and_then_the_pages_of_its_directory_by_name() {
    let (dir, atlas_id) = atlas_dir("atlas-list").await;

    assert_eq!(
        outline_of(&atlas_id).await,
        vs(&[
            &format!("1:[[{dir}z/index]]"),
            &format!("1:[[{dir}a]]"),
            &format!("1:[[{dir}b]]"),
        ])
    );
}

// アトラスは、ページの追加、改名、削除に追随する。
#[tokio::test]
async fn the_atlas_follows_a_new_a_renamed_and_a_deleted_page() {
    let (dir, atlas_id) = atlas_dir("atlas-follow").await;
    let a = doc_in(&dir, "a").await;
    let b = doc_in(&dir, "b").await;
    let card = card_of(&items(&atlas_id).await, &a).id.clone();
    let (status, resp) = put(
        &format!("/api/items/{card}/properties/atlas_bounds"),
        json!({ "valueType": "bounds", "value": "5,6,0,40" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (status, resp) = put(&format!("/api/pages/{a}"), json!({ "title": "y" })).await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(delete(&format!("/api/pages/{b}")).await, 204);
    doc_in(&dir, "x").await;

    let rows = items(&atlas_id).await;
    assert_eq!(
        outline(&rows),
        vs(&[
            &format!("1:[[{dir}z/index]]"),
            &format!("1:[[{dir}x]]"),
            &format!("1:[[{dir}y]]"),
        ])
    );
    let renamed = card_of(&rows, &a);
    assert_eq!(renamed.id, card);
    assert_eq!(renamed.properties["atlas_bounds"], json!("5,6,0,40"));
}

// カードを置いても、そのページは変わらない。
#[tokio::test]
async fn placing_a_card_leaves_its_page_as_it_was() {
    let (dir, atlas_id) = atlas_dir("atlas-place").await;
    let a = doc_in(&dir, "a").await;
    let (_, before) = get(&format!("/api/pages/{a}")).await;
    let card = card_of(&items(&atlas_id).await, &a).id.clone();

    let (status, resp) = put(
        &format!("/api/items/{card}/properties/atlas_bounds"),
        json!({ "valueType": "text", "value": " 24, 24, 0 ,40 " }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(
        card_of(&items(&atlas_id).await, &a).properties["atlas_bounds"],
        json!("24,24,0,40")
    );
    let (_, after) = get(&format!("/api/pages/{a}")).await;
    assert_eq!(
        after["page"]["modifiedDate"],
        before["page"]["modifiedDate"]
    );
    let (status, resp) = put(
        &format!("/api/pages/{a}/properties/atlas_bounds"),
        json!({ "valueType": "bounds", "value": "1,2,3,4" }),
    )
    .await;
    assert_eq!(status, 400, "{resp}");
    let (status, resp) = put(
        &format!("/api/items/{card}/properties/atlas_bounds"),
        json!({ "valueType": "text", "value": "not bounds" }),
    )
    .await;
    assert_eq!(status, 400, "{resp}");
}

// リンクカードは名前の順に置かれ、リンク先の変更と削除ができる。
#[tokio::test]
async fn a_link_card_takes_its_place_by_its_name_and_can_be_relinked_and_removed() {
    let (dir, atlas_id) = atlas_dir("atlas-link").await;

    let rows = atlas_post(
        &atlas_id,
        "links",
        json!({ "target": "[[elsewhere/aa]]", "bounds": "1,2,0,40" }),
    )
    .await;
    assert_eq!(
        outline(&rows),
        vs(&[
            &format!("1:[[{dir}z/index]]"),
            &format!("1:[[{dir}a]]"),
            "1:[[elsewhere/aa]]",
            &format!("1:[[{dir}b]]"),
        ])
    );
    let link = rows[2].clone();
    assert_eq!(link.properties["atlas_bounds"], json!("1,2,0,40"));

    let rows = atlas_put(
        &atlas_id,
        &format!("links/{}", link.id),
        json!({ "target": "far/zz" }),
    )
    .await;
    assert_eq!(rows.last().unwrap().outline(), "1:[[far/zz]]");
    assert_eq!(rows.last().unwrap().id, link.id);

    let (status, resp) = send(
        reqwest::Method::DELETE,
        &format!("/api/pages/{atlas_id}/atlas/items/{}", link.id),
        None,
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(outline_of(&atlas_id).await.len(), 3);
}

// カードに落としたカードはグループになり、グループは残り1枚で終わる。
#[tokio::test]
async fn cards_dropped_on_a_card_make_a_group_that_ends_with_its_second_last_card() {
    let (dir, atlas_id) = atlas_dir("atlas-group").await;
    let a = doc_in(&dir, "a").await;
    let b = doc_in(&dir, "b").await;
    let link = atlas_post(
        &atlas_id,
        "links",
        json!({ "target": "elsewhere/l", "bounds": "0,0,0,40" }),
    )
    .await
    .into_iter()
    .find(|r| r.body == "[[elsewhere/l]]")
    .unwrap();

    let rows = atlas_post(
        &atlas_id,
        "group",
        json!({ "cards": [b, link.id], "target": a }),
    )
    .await;
    assert_eq!(
        outline(&rows),
        vs(&[
            "1:",
            &format!("2:[[{dir}a]]"),
            &format!("2:[[{dir}b]]"),
            "2:[[elsewhere/l]]",
            &format!("1:[[{dir}z/index]]"),
        ])
    );
    let group = rows[0].id.clone();
    assert_eq!(rows[0].properties["atlas_type"], json!("group"));

    let rows = atlas_put(
        &atlas_id,
        &format!("groups/{group}"),
        json!({ "side": "left", "title": "Tools" }),
    )
    .await;
    assert_eq!(rows[0].outline(), "1:Tools");
    assert_eq!(rows[0].properties["atlas_side"], json!("left"));

    atlas_post(&atlas_id, "ungroup", json!({ "cards": [link.id] })).await;
    let rows = atlas_post(&atlas_id, "ungroup", json!({ "cards": [b] })).await;
    assert!(rows.iter().all(|r| r.level == 1), "{rows:?}");
    assert!(rows.iter().all(|r| r.id != group), "{rows:?}");
}

// コールアウトは、親の下にまとめて末尾に置かれる。
#[tokio::test]
async fn the_callouts_go_last_under_their_parent() {
    let (_, atlas_id) = atlas_dir("atlas-callout").await;

    let rows = atlas_post(
        &atlas_id,
        "callouts",
        json!({ "bounds": "10,20,0,60", "side": "left", "text": "Hello" }),
    )
    .await;
    let n = rows.len();
    assert_eq!(outline(&rows[n - 2..]), vs(&["1:callout", "2:Hello"]));
    let callout = rows[n - 1].clone();
    assert_eq!(callout.properties["atlas_type"], json!("callout"));
    assert_eq!(callout.properties["atlas_side"], json!("left"));
    assert_eq!(callout.properties["atlas_bounds"], json!("10,20,0,60"));

    // A new text is a change of the item: it is parsed again and dated anew
    let (_, before) = get(&format!("/api/items/{}", callout.id)).await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let rows = atlas_put(
        &atlas_id,
        &format!("callouts/{}", callout.id),
        json!({ "side": "bottom", "text": "## Bye" }),
    )
    .await;
    let renamed = rows.last().unwrap();
    assert_eq!(renamed.outline(), "2:## Bye");
    assert!(
        renamed.types.contains(&"heading".to_string()),
        "{renamed:?}"
    );
    let (_, after) = get(&format!("/api/items/{}", callout.id)).await;
    assert!(after["item"]["ast"].as_str().unwrap().contains("Bye"));
    assert!(
        after["item"]["modifiedDate"].as_i64() > before["item"]["modifiedDate"].as_i64(),
        "{before} {after}"
    );

    let rows = atlas_put(
        &atlas_id,
        &format!("callouts/{}", callout.id),
        json!({ "side": "bottom", "text": " " }),
    )
    .await;
    assert_eq!(rows.len(), n - 2, "{rows:?}");
}

// アトラスページに書いた項目は、アトラスが書き直されると消える。
#[tokio::test]
async fn an_item_written_on_an_atlas_page_goes_when_the_atlas_is_written() {
    let (dir, atlas_id) = atlas_dir("atlas-own").await;
    fill(&atlas_id, &["a note"]).await;

    doc_in(&dir, "d").await;

    let lines = outline_of(&atlas_id).await;
    assert!(!lines.iter().any(|l| l.contains("a note")), "{lines:?}");
}

// 値は、送られた型ではなく定義の型で検査される。
#[tokio::test]
async fn a_value_is_checked_by_the_type_of_its_definition_not_the_type_sent() {
    let page = new_doc_page("defined-type").await;
    let item = fill(&page.id, &["zoomed"]).await.remove(0);
    let (status, resp) = put(
        &format!("/api/items/{}/properties/zoom", item.id),
        json!({ "valueType": "text", "value": "large" }),
    )
    .await;
    assert_eq!(status, 400, "{resp}");
}

// ---- Extension functions ----

// 拡張機能のファイルは、フォルダを除いて名前順に一覧される。
#[tokio::test]
async fn the_files_of_a_function_are_listed_by_name_without_its_folders() {
    let dir = server()
        .root
        .join("config")
        .join("functions")
        .join("fn-list");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    for name in ["b.md", "a.md", "index.html"] {
        std::fs::write(dir.join(name), "x").unwrap();
    }

    let (status, resp) = get("/api/functions/fn-list").await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(strings(&resp), vs(&["a.md", "b.md", "index.html"]));
}

// ない拡張機能にはファイル一覧がない。
#[tokio::test]
async fn a_missing_function_has_no_file_list() {
    let (status, _) = get("/api/functions/fn-missing").await;
    assert_eq!(status, 404);
}
