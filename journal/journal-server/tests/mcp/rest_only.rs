//! The tools of the operations that only REST had before ADR 060.

use crate::common::*;
use crate::{call, call_err, connect};
use serde_json::{json, Value};

fn ids(answer: &Value, list: &str) -> Vec<String> {
    strings(&Value::Array(field_of_each(answer, list, "id")))
}

// エディターの操作（貼り付け、分割、結合、字下げ、バッチ、まとめての削除、スタッシュ）が、ツールでもできる。
#[tokio::test]
async fn the_operations_of_an_editor_are_done_by_tools() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-editor").await;
    let rows = fill(&page.id, &["a", "b", "c", "d", "e"]).await;
    let id = |i: usize| rows[i].id.clone();

    let pasted = call(
        &mcp,
        "paste_text",
        json!({ "itemId": id(0), "body": "a", "cursor": 1, "text": "x" }),
    )
    .await;
    let split = call(
        &mcp,
        "split_item",
        json!({ "itemId": id(0), "body": "ax", "cursor": 1 }),
    )
    .await;
    let made = split["created"]["id"].as_str().unwrap().to_string();
    let merged = call(
        &mcp,
        "merge_items",
        json!({ "itemId": made, "intoId": id(0) }),
    )
    .await;
    let shifted = call(
        &mcp,
        "shift_item_levels",
        json!({ "itemIds": [id(1)], "delta": 1 }),
    )
    .await;
    let batch = call(
        &mcp,
        "apply_batch",
        json!({ "ops": [{ "op": "update", "id": id(2), "body": "c2" }] }),
    )
    .await;
    let deleted = call(&mcp, "delete_items", json!({ "itemIds": [id(3)] })).await;
    let stashed = call(&mcp, "stash_item", json!({ "itemId": id(4) })).await;

    assert_eq!(field_of_each(&pasted, "items", "body"), vec![json!("ax")]);
    assert_eq!(split["item"]["body"], json!("a"));
    assert_eq!(split["created"]["body"], json!("x"));
    assert_eq!(merged["item"]["body"], json!("ax"));
    assert_eq!(field_of_each(&shifted, "items", "level"), vec![json!(2)]);
    assert_eq!(field_of_each(&batch, "items", "body"), vec![json!("c2")]);
    assert_eq!(deleted["created"], json!([]));
    assert_eq!(ids(&stashed, "items"), vec![id(4)]);
    assert_eq!(stashed["heading"]["body"], json!("# Stash"));
    assert_eq!(
        outline_of(&page.id).await,
        vec!["1:ax", "2:b", "1:c2", "1:# Stash", "2:e"]
    );
}

// 項目のプロパティの削除、トランスクルードの問い合わせ、Flat Markdownの解析が、ツールでもできる。
#[tokio::test]
async fn a_property_is_deleted_transclusions_are_asked_and_flat_markdown_is_parsed_by_tools() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-more-items").await;
    let rows = fill(&page.id, &["a", "b"]).await;
    let (a, b) = (rows[0].id.clone(), rows[1].id.clone());
    call(
        &mcp,
        "upsert_property",
        json!({ "itemId": a, "key": "tags", "valueType": "tags", "value": "x" }),
    )
    .await;
    call(
        &mcp,
        "set_transclusion",
        json!({ "itemId": b, "sourceItemId": a }),
    )
    .await;

    let without = call(
        &mcp,
        "delete_property",
        json!({ "itemId": a, "key": "tags" }),
    )
    .await;
    let shown = call(&mcp, "get_transcluded_items", json!({ "itemIds": [a] })).await;
    let parsed = call(
        &mcp,
        "parse_flat_markdown",
        json!({ "markdown": "one\n\n<!-- @@ -->\n\ntwo" }),
    )
    .await;

    assert!(
        without["item"]["properties"].get("tags").is_none(),
        "{without}"
    );
    assert_eq!(shown["itemIds"], json!([a]));
    assert_eq!(
        field_of_each(&parsed, "items", "body"),
        vec![json!("one"), json!("two")]
    );
}

// ページのプロパティ、子ページの一覧、フォルダーのページを下ろす操作が、ツールでもできる。
#[tokio::test]
async fn the_properties_and_the_children_of_a_page_are_handled_by_tools() {
    let mcp = connect().await;
    let dir = unique_name("mcp-pages");
    let child = call(
        &mcp,
        "get_or_create_doc",
        json!({ "title": "child", "dirPath": format!("{dir}/") }),
    )
    .await;
    let child_id = child["page"]["id"].as_str().unwrap().to_string();
    let index = child["page"]["parentId"].as_str().unwrap().to_string();

    let children = call(&mcp, "get_child_pages", json!({ "pageId": index })).await;
    let tagged = call(
        &mcp,
        "upsert_page_property",
        json!({ "pageId": child_id, "key": "tags", "valueType": "tags", "value": "x" }),
    )
    .await;
    let untagged = call(
        &mcp,
        "delete_page_property",
        json!({ "pageId": child_id, "key": "tags" }),
    )
    .await;
    call(&mcp, "delete_page", json!({ "pageId": child_id })).await;
    let demoted = call(
        &mcp,
        "demote_page",
        json!({ "pageId": index, "overwrite": false }),
    )
    .await;

    assert_eq!(ids(&children, "pages"), vec![child_id]);
    assert_eq!(tagged["page"]["properties"]["tags"], json!(["x"]));
    assert!(
        untagged["page"]["properties"].get("tags").is_none(),
        "{untagged}"
    );
    assert_eq!(demoted["page"]["name"], json!(dir));
}

// 週のジャーナルの検索、ジャーナルへの移動、タグでの検索が、ツールでもできる。
#[tokio::test]
async fn items_go_to_a_journal_and_a_weekly_journal_and_a_tag_are_found_by_tools() {
    let mcp = connect().await;
    call(
        &mcp,
        "get_or_create_journal",
        json!({ "fileName": "2096-W10" }),
    )
    .await;
    let page = new_doc_page("mcp-to-journal").await;
    let rows = fill(&page.id, &["go #mcpfound", "stay"]).await;

    let moved = call(
        &mcp,
        "move_items_to_journal",
        json!({ "fileName": "2096-03-05", "itemIds": [rows[0].id], "position": "last" }),
    )
    .await;
    let week = call(
        &mcp,
        "get_most_recent_weekly_journal_before",
        json!({ "beforeWeek": "2096-W12" }),
    )
    .await;
    let tagged = call(
        &mcp,
        "search_by_tag",
        json!({ "tag": "mcpfound", "maxResults": 5 }),
    )
    .await;

    assert_eq!(ids(&moved, "items"), vec![rows[0].id.clone()]);
    assert_eq!(outline_of(&page.id).await, vec!["1:stay"]);
    assert_eq!(week["found"], json!(true));
    assert_eq!(week["page"]["name"], json!("2096-W10"));
    assert_eq!(tagged["results"].as_array().unwrap().len(), 1, "{tagged}");
}

// プロパティの定義が、ツールで読まれ、足され、消される。
#[tokio::test]
async fn the_definitions_of_the_properties_are_read_added_and_deleted_by_tools() {
    let mcp = connect().await;
    let key = format!("{}_n", unique_name("mcp").replace('-', "_"));
    let has = |answer: &Value| field_of_each(answer, "definitions", "key").contains(&json!(key));

    let before = call(&mcp, "get_property_definitions", json!({})).await;
    let added = call(&mcp, "add_property_definition", json!({ "key": key })).await;
    let deleted = call(&mcp, "delete_property_definition", json!({ "key": key })).await;

    assert!(!has(&before), "{before}");
    assert!(has(&added), "{added}");
    assert!(!has(&deleted), "{deleted}");
}

// フォルダーとの同期の状態と走査は、RESTのルートと同じJSONで答える。
#[tokio::test]
async fn the_state_of_the_folder_and_a_scan_answer_as_their_rest_routes_do() {
    let mcp = connect().await;

    let status = call(&mcp, "get_sync_status", json!({})).await;
    let scan = call(&mcp, "scan_folder", json!({})).await;
    let (_, by_rest) = get("/api/sync/status").await;

    assert_eq!(status["markdownRoot"], by_rest["markdownRoot"]);
    assert_eq!(status["rootAvailable"], by_rest["rootAvailable"]);
    assert!(scan["imported"].is_number(), "{scan}");
}

// ページと型での一覧は、fieldsで頼まれたフィールドだけを各項目に持つ。型での一覧は、プロパティのある項目を除ける。
#[tokio::test]
async fn a_page_and_a_list_by_type_give_only_the_fields_asked_for() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-fields").await;
    let rows = fill(&page.id, &["plain", "tagged"]).await;
    for row in &rows {
        call(&mcp, "cycle_task", json!({ "itemId": row.id })).await;
    }
    call(
        &mcp,
        "upsert_property",
        json!({ "itemId": rows[1].id, "key": "tags", "valueType": "tags", "value": "x" }),
    )
    .await;

    let narrow = call(
        &mcp,
        "get_page_by_id",
        json!({ "pageId": page.id, "fields": "body" }),
    )
    .await;
    let open = call(
        &mcp,
        "get_items_by_type",
        json!({ "types": "task", "withoutProperty": "tags", "fields": "id" }),
    )
    .await;

    assert_eq!(
        narrow["items"],
        json!([{ "body": "plain" }, { "body": "tagged" }])
    );
    assert_eq!(narrow["page"]["id"], json!(page.id));
    let open = open["items"].as_array().unwrap();
    assert!(open.contains(&json!({ "id": rows[0].id })), "{open:?}");
    assert!(!open.contains(&json!({ "id": rows[1].id })), "{open:?}");
}

// PCのファイルは、パスを渡すと、新しい名前でアセットに写される。答えは、RESTのアップロードと同じ形である。
#[tokio::test]
async fn a_file_of_the_pc_is_copied_into_the_assets_by_its_path() {
    let mcp = connect().await;
    let name = format!("{}.PNG", unique_name("mcp-shot"));
    let file = std::env::temp_dir().join(&name);
    std::fs::write(&file, b"drawing").unwrap();

    let answer = call(&mcp, "upload_asset", json!({ "filePath": file })).await;

    let saved = answer["fileName"].as_str().unwrap();
    assert_eq!(answer["originalName"], json!(name));
    assert!(
        saved.ends_with(".png") && !saved.contains("mcp-shot"),
        "{saved}"
    );
    let saved = server().markdown_dir.join("assets").join(saved);
    assert_eq!(std::fs::read(saved).unwrap(), b"drawing");
    assert_eq!(std::fs::read(&file).unwrap(), b"drawing");
    std::fs::remove_file(file).unwrap();
}

// ファイルのない絶対パスと、絶対でないパスは、断られる。
#[tokio::test]
async fn a_path_that_names_no_file_and_a_path_that_is_not_absolute_are_refused() {
    let mcp = connect().await;
    let missing = std::env::temp_dir().join(unique_name("mcp-no-such-file"));
    let folder = std::env::temp_dir();

    let no_file = call_err(&mcp, "upload_asset", json!({ "filePath": missing })).await;
    let a_folder = call_err(&mcp, "upload_asset", json!({ "filePath": folder })).await;
    let relative = call_err(&mcp, "upload_asset", json!({ "filePath": "shot.png" })).await;

    assert!(no_file.contains("No file at"), "{no_file}");
    assert!(a_folder.contains("No file at"), "{a_folder}");
    assert!(relative.contains("absolute"), "{relative}");
}

// ちょうど上限の50MBのファイルは写され、1バイトでも超えるファイルは断られる。
#[tokio::test]
async fn a_file_of_the_limit_is_copied_and_a_file_over_it_is_refused() {
    let mcp = connect().await;
    let limit = 50 * 1024 * 1024;
    let file = std::env::temp_dir().join(format!("{}.bin", unique_name("mcp-large")));

    std::fs::write(&file, vec![1; limit]).unwrap();
    let copied = call(&mcp, "upload_asset", json!({ "filePath": file })).await;
    std::fs::write(&file, vec![1; limit + 1]).unwrap();
    let refusal = call_err(&mcp, "upload_asset", json!({ "filePath": file })).await;
    std::fs::remove_file(file).unwrap();

    let saved = server()
        .markdown_dir
        .join("assets")
        .join(copied["fileName"].as_str().unwrap());
    assert_eq!(std::fs::metadata(&saved).unwrap().len(), limit as u64);
    std::fs::remove_file(saved).unwrap();
    assert!(refusal.contains("50MB"), "{refusal}");
}
