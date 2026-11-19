//! The API tests of the Java server, run against the Rust server.

use crate::common::*;
use serde_json::{json, Value};

// ---- Items ----

// 型での一覧の`withoutProperty`のクエリは、そのプロパティを持つ項目を除く条件として読まれる。除く規則はjournal-coreのテストが確かめる。
#[tokio::test]
async fn the_without_property_query_of_a_list_by_type_is_read() {
    let page = new_doc_page("items-task-list-query").await;
    let done = fill(&page.id, &["done task"]).await.remove(0);
    // A cycle makes a task of an item, starts it, and ends it, in this order
    for _ in 0..3 {
        let cycle = format!("/api/items/{}/cycle-task", done.id);
        assert_eq!(post_empty(&cycle).await.0, 200);
    }

    let (status, all) = get("/api/items?types=task").await;
    assert_eq!(status, 200);
    let (status, open) = get("/api/items?types=task&withoutProperty=end").await;
    assert_eq!(status, 200);

    assert!(field_of_each(&all, "items", "id").contains(&json!(done.id)));
    assert!(!field_of_each(&open, "items", "id").contains(&json!(done.id)));
}

// 型での一覧は、fieldsで頼まれたフィールドだけを各項目に持つ。表示だけのクライアントは、本文のMarkdownを受け取らずに済む。
#[tokio::test]
async fn items_listed_by_type_have_only_the_fields_asked_for() {
    let page = new_doc_page("items-task-list-fields").await;
    let item = fill(&page.id, &["narrow task"]).await.remove(0);
    let cycle = format!("/api/items/{}/cycle-task", item.id);
    let (_, task) = post_empty(&cycle).await;

    let (status, resp) = get("/api/items?types=task&fields=id,ast").await;
    assert_eq!(status, 200);

    let listed = resp["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|listed| listed["id"] == json!(item.id));
    assert_eq!(
        listed,
        Some(&json!({ "id": item.id, "ast": task["item"]["ast"] }))
    );
}

// 1件の取得は、fieldsで頼まれたフィールドだけを項目に持つ。itemの包みは変わらない。
#[tokio::test]
async fn one_item_is_read_with_only_the_fields_asked_for() {
    let page = new_doc_page("items-get-fields").await;
    let item = fill(&page.id, &["top", "  narrow"]).await.remove(1);

    let (status, resp) = get(&format!("/api/items/{}?fields=body,level", item.id)).await;

    assert_eq!(status, 200);
    assert_eq!(resp, json!({ "item": { "body": "narrow", "level": 2 } }));
}

// 型を指定しない一覧は、空である。
#[tokio::test]
async fn items_listed_without_a_type_are_none() {
    let page = new_doc_page("items-no-type-list").await;
    fill(&page.id, &["an item"]).await;

    let (status, resp) = get("/api/items").await;

    assert_eq!(status, 200);
    assert_eq!(resp["items"], json!([]));
}

// ---- Items: the clients told of a change ----

const CLIENT_A: &str = "0a0a0a0a-0000-4000-8000-00000000000a";

/// Creates an item as the client that the `X-Client-Id` header names, or as one that names none.
async fn create_item_as(client_id: Option<&str>, page_id: &str) {
    let mut request = client().post(url("/api/items")).json(&json!({
        "pageId": page_id, "position": "last", "level": 1, "body": "x", "id": ""
    }));
    if let Some(client_id) = client_id {
        request = request.header("X-Client-Id", client_id);
    }
    assert_eq!(request.send().await.unwrap().status(), 200);
}

// APIでの変更は、リクエストのX-Client-Idヘッダーが名乗るクライアントの変更として知らされる。
#[tokio::test]
async fn the_clients_hear_which_client_changed_the_page() {
    let page = new_doc_page("items-event-client").await;
    let mut events = client().get(url("/api/events")).send().await.unwrap();

    create_item_as(Some(CLIENT_A), &page.id).await;

    let event = event_with(&mut events, &format!("\"pageId\":\"{}\"", page.id)).await;
    assert_eq!(event["source"], "app");
    assert_eq!(event["clientId"], CLIENT_A);
}

// ---- Pages ----

// ページの取得の`fields`のクエリは読まれ、各項目はそのフィールドだけを、答えのJSONの名前で持つ。絞る規則はjournal-coreのテストが確かめる。
#[tokio::test]
async fn the_fields_query_of_a_page_is_read() {
    let page = new_doc_page("pages-get-fields-query").await;
    let rows = fill(&page.id, &["one"]).await;

    let (status, resp) = get(&format!("/api/pages/{}?fields=id,parentItemId", page.id)).await;
    assert_eq!(status, 200);

    assert_eq!(
        resp["items"],
        json!([{ "id": rows[0].id, "parentItemId": null }])
    );
}

// 別のページと同じ名前への改名は409で断られる。上書きの規則はjournal-coreのテストが確かめる。
#[tokio::test]
async fn a_rename_to_the_name_of_another_page_is_answered_with_409() {
    let page = new_doc_page("pages-overwrite-status").await;
    let other = new_doc_page("pages-overwritten-status").await;

    let (status, resp) = put(
        &format!("/api/pages/{}", page.id),
        json!({ "title": other.title }),
    )
    .await;

    assert_eq!(status, 409, "{resp}");
}

// ページの削除は204で答え、消したページの取得は404で答える。
#[tokio::test]
async fn a_deleted_page_is_answered_with_204_and_then_404() {
    let page = new_doc_page("pages-delete-status").await;

    assert_eq!(delete(&format!("/api/pages/{}", page.id)).await, 204);

    assert_eq!(get(&format!("/api/pages/{}", page.id)).await.0, 404);
}

// ---- Markdown conversions ----

// `/api/items/export`の答えは、木とFlat Markdownだけを持ち、ASTを持たない。アプリは写しを自分で解析する。木の形はjournal-coreのテストが確かめる。
#[tokio::test]
async fn the_export_answers_without_an_ast() {
    let page = new_doc_page("markdown-export-answer").await;
    let rows = fill(&page.id, &["parent", "  child"]).await;
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();

    let (status, exported) = post("/api/items/export", json!({ "itemIds": ids })).await;
    assert_eq!(status, 200, "{exported}");

    let mut keys: Vec<&String> = exported.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["flatMarkdown", "treeMarkdown"]);
}

// ---- Folder sync ----

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

// フォルダから消えたファイルは、削除を確かめるページに、そのページのidとともに並ぶ。
#[tokio::test]
async fn a_file_gone_from_the_folder_is_listed_on_the_page_that_asks_about_it() {
    let page = new_doc_page("sync-pending").await;
    let state = &server().state;
    state.export.process_whole_queue();
    let file = server()
        .markdown_dir
        .join(format!("docs/{}.md", page.title));
    std::fs::remove_file(&file).unwrap();
    post_empty("/api/sync/scan").await;

    let html = client()
        .get(url("/api/sync/pending-deletes"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert!(html.contains(&page.id), "{html}");
}

// ---- Health and config ----

// ヘルスチェックはokを返す。設定の読み書きはjournal-coreのテストが確かめる。
#[tokio::test]
async fn health_is_ok() {
    let (status, resp) = get("/api/health").await;
    assert_eq!(status, 200);
    assert_eq!(resp["status"], json!("ok"));
}

// ---- Assets ----

const MB: usize = 1024 * 1024;

/// Sends a file through the upload of the frontends. The server may close the connection
/// while a file it refuses is on the way, so the request itself can fail.
async fn upload_asset(file_name: &str, bytes: Vec<u8>) -> reqwest::Result<reqwest::Response> {
    let part = reqwest::multipart::Part::bytes(bytes).file_name(file_name.to_string());
    let form = reqwest::multipart::Form::new().part("file", part);
    client()
        .post(url("/api/assets/upload"))
        .multipart(form)
        .send()
        .await
}

fn saved_asset(file_name: &str) -> std::path::PathBuf {
    server().markdown_dir.join("assets").join(file_name)
}

// 送ったファイルは、HMTIDの名前で、中身のまま保存される。答えには、保存した名前と、元の名前がそのまま入る。名前の拡張子の規則は`asset_name`の単体テストが確かめる。
#[tokio::test]
async fn an_upload_is_answered_with_its_saved_and_its_original_name() {
    let response = upload_asset("構成図 v2.PNG", b"drawing".to_vec())
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
    let answer: Value = response.json().await.unwrap();
    let file_name = answer["fileName"].as_str().unwrap();
    let id = file_name.split_once('.').map_or(file_name, |(id, _)| id);
    assert!(hmtid::Hmtid::parse(id).is_ok(), "{file_name}");
    assert_eq!(answer["originalName"], "構成図 v2.PNG");
    assert_eq!(std::fs::read(saved_asset(file_name)).unwrap(), b"drawing");
}

// 同じ名前のファイルを続けて送ると、別々の名前で保存され、どちらも残る。
#[tokio::test]
async fn two_uploads_with_one_name_are_saved_as_two_files() {
    let mut saved = Vec::new();
    for bytes in [b"first", b"again"] {
        let response = upload_asset("image.png", bytes.to_vec()).await.unwrap();
        let answer: Value = response.json().await.unwrap();
        saved.push(answer["fileName"].as_str().unwrap().to_string());
    }

    assert_ne!(saved[0], saved[1]);
    assert_eq!(std::fs::read(saved_asset(&saved[0])).unwrap(), b"first");
    assert_eq!(std::fs::read(saved_asset(&saved[1])).unwrap(), b"again");
}

// 上限の50MBに近い画像も、中身のまま保存される。
#[tokio::test]
async fn an_image_just_under_the_limit_is_saved_as_it_is() {
    let bytes: Vec<u8> = (0..49 * MB).map(|i| (i % 251) as u8).collect();

    let response = upload_asset("just-under-the-limit.png", bytes.clone())
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
    let answer: Value = response.json().await.unwrap();
    let saved = saved_asset(answer["fileName"].as_str().unwrap());
    assert!(std::fs::read(&saved).unwrap() == bytes);
    std::fs::remove_file(saved).unwrap();
}

// 上限の50MBを超える画像は断られ、保存されない。
#[tokio::test]
async fn an_image_over_the_limit_is_refused_and_not_saved() {
    let size = 51 * MB;

    let result = upload_asset("over-the-limit.png", vec![0; size]).await;

    assert!(result.map_or(true, |response| !response.status().is_success()));
    // The name of a saved file is not known beforehand, so it is looked for by its size.
    // The folder is missing until the first upload is saved, and the other tests add and
    // remove files in it meanwhile: what cannot be read is not the file looked for
    let saved = std::fs::read_dir(server().markdown_dir.join("assets"))
        .into_iter()
        .flatten()
        .flatten()
        .any(|file| file.metadata().is_ok_and(|data| data.len() == size as u64));
    assert!(!saved);
}
