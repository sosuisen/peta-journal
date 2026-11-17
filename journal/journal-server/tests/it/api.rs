//! The API tests of the Java server, run against the Rust server.

use crate::common::*;
use serde_json::{json, Value};

fn vs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// ---- Items ----

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

// ファイルから取り込んだ変更は、ファイルからの変更としてクライアントに知らされる。
#[tokio::test]
async fn the_clients_hear_of_a_change_taken_in_from_the_file() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("sync-import-event").await;
    fill(&page.id, &["before the edit"]).await;
    wait_until(&page, "the page is exported", || {
        read(&file_of(&page)).contains("before the edit")
    })
    .await;
    let mut events = client().get(url("/api/events")).send().await.unwrap();

    std::fs::write(
        file_of(&page),
        read(&file_of(&page)).replace("before the edit", "edited on disk, a bit longer"),
    )
    .unwrap();
    scan().await;

    let event = event_with(&mut events, &format!("\"pageId\":\"{}\"", page.id)).await;
    assert_eq!(event["source"], "file");
}

/// Exports the page, and returns its file once it is there.
async fn exported_file_of(page: &PageRef) -> std::path::PathBuf {
    let file = file_of(page);
    wait_until(page, "the page has its file", || file.exists()).await;
    file
}

async fn rename(page: &PageRef) -> PageRef {
    let title = format!("{}-renamed", page.title);
    let (status, resp) = put(
        &format!("/api/pages/{}", page.id),
        json!({ "title": title, "dirPath": "" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    PageRef {
        id: page.id.clone(),
        title,
    }
}

// ページの名前を変えて書き出すと、古い名前のファイルは消える。
#[tokio::test]
async fn exporting_a_renamed_page_removes_the_file_of_its_old_name() {
    let page = new_doc_page("sync-export-rename").await;
    let old_file = exported_file_of(&page).await;

    let renamed = rename(&page).await;
    let new_file = exported_file_of(&renamed).await;

    assert!(new_file.exists());
    assert!(!old_file.exists());
}

// ページを削除すると、そのファイルも消える。
#[tokio::test]
async fn deleting_a_page_removes_its_file() {
    let page = new_doc_page("sync-export-delete").await;
    let file = exported_file_of(&page).await;

    assert_eq!(delete(&format!("/api/pages/{}", page.id)).await, 204);

    assert!(!file.exists());
}

// 名前を変えたページを書き出す前に削除しても、古い名前のファイルは消える。
#[tokio::test]
async fn deleting_a_renamed_page_before_its_export_removes_the_file_of_its_old_name() {
    let page = new_doc_page("sync-export-rename-delete").await;
    let old_file = exported_file_of(&page).await;

    rename(&page).await;
    assert_eq!(delete(&format!("/api/pages/{}", page.id)).await, 204);

    assert!(!old_file.exists());
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

/// Defines a key with the type its name gives, as the user does in Settings. The tests share
/// one server, so a key another test defined already is left as it is.
async fn define_key(key: &str) {
    let (status, resp) = post("/api/config/property-definitions", json!({ "key": key })).await;
    let defined = status == 200 || resp.to_string().contains("already defined");
    assert!(defined, "{key} -> {resp}");
}

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

// リストのキーは、名前が_listで終わるときだけ定義でき、名前が_listで終わるキーはリストでしか定義できない。
#[tokio::test]
async fn a_list_key_is_defined_only_with_a_name_that_ends_with_list() {
    let (status, resp) = post(
        "/api/config/property-definitions",
        json!({ "key": "defined_member_list", "valueType": "list" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(
        definition(&resp, "defined_member_list").unwrap()["valueType"],
        json!("list")
    );

    for body in [
        json!({ "key": "defined_members", "valueType": "list" }),
        json!({ "key": "defined_owner_list", "valueType": "text" }),
    ] {
        let (status, resp) = post("/api/config/property-definitions", body.clone()).await;
        assert_eq!(status, 400, "{body} -> {resp}");
    }
}

// キーは、名前の接尾辞から決まる型でだけ定義できる。
#[tokio::test]
async fn a_key_is_defined_only_with_the_type_its_suffix_gives() {
    for (key, value_type) in [
        ("defined_score_n", "number"),
        ("defined_due_d", "date"),
        ("defined_seen_dt", "datetime"),
        ("defined_read_cb", "checkbox"),
        ("defined_place_bounds", "bounds"),
    ] {
        let (status, resp) = post(
            "/api/config/property-definitions",
            json!({ "key": key, "valueType": value_type }),
        )
        .await;
        assert_eq!(status, 200, "{key} -> {resp}");
        assert_eq!(
            definition(&resp, key).unwrap()["valueType"],
            json!(value_type)
        );
    }

    for body in [
        json!({ "key": "defined_score", "valueType": "number" }),
        json!({ "key": "defined_when_d", "valueType": "text" }),
        json!({ "key": "defined_count_n", "valueType": "date" }),
    ] {
        let (status, resp) = post("/api/config/property-definitions", body.clone()).await;
        assert_eq!(status, 400, "{body} -> {resp}");
    }
}

// 名前の本体が予約の接尾辞で終わるキーは、定義できない。
#[tokio::test]
async fn a_key_whose_body_ends_with_a_reserved_suffix_is_not_defined() {
    for body in [
        json!({ "key": "defined_score_n_n", "valueType": "number" }),
        json!({ "key": "defined_score_n_list", "valueType": "list" }),
    ] {
        let (status, resp) = post("/api/config/property-definitions", body.clone()).await;
        assert_eq!(status, 400, "{body} -> {resp}");
    }
}

// 名前の本体がほかのユーザーのキーと同じなら、別の型でも定義できない。消したキーの本体なら、別の型で定義できる。
#[tokio::test]
async fn a_key_whose_body_another_key_has_is_not_defined_until_that_key_is_deleted() {
    let defs = "/api/config/property-definitions";
    let (status, resp) = post(defs, json!({ "key": "shared_body_n" })).await;
    assert_eq!(status, 200, "{resp}");

    let (status, resp) = post(defs, json!({ "key": "shared_body_d" })).await;
    assert_eq!(status, 400, "{resp}");

    let (status, resp) = send(
        reqwest::Method::DELETE,
        "/api/config/property-definitions/shared_body_n",
        None,
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let (status, resp) = post(defs, json!({ "key": "shared_body_d" })).await;
    assert_eq!(status, 200, "{resp}");
}

// 型を送らずに定義すると、キーは名前から決まる型になる。
#[tokio::test]
async fn a_key_defined_without_a_type_takes_the_type_its_name_gives() {
    let (status, resp) = post(
        "/api/config/property-definitions",
        json!({ "key": "untyped_score_n" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(
        definition(&resp, "untyped_score_n").unwrap()["valueType"],
        json!("number")
    );
}

// 項目からプロパティを削除できる。
#[tokio::test]
async fn a_property_can_be_removed_from_an_item() {
    let page = new_doc_page("items-property-delete").await;
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
    define_key("page_list").await;
    let _tags_dir = TAGS_DIR.lock().await;
    let page = new_doc_page("page-list").await;
    set_page_tags(&page.id, "page-list-tag").await;

    // The second value replaces the first, as a value of a scalar does. The values are names
    // no other test gives a page: the server is shared, and another test renames a page `y`.
    for value in ["list-x, list-y", "list-y,list-z"] {
        let (status, resp) = put(
            &format!("/api/pages/{}/properties/page_list", page.id),
            json!({ "valueType": "list", "value": value }),
        )
        .await;
        assert_eq!(status, 200, "{resp}");
    }

    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(
        resp["page"]["properties"]["page_list"],
        json!(["list-y", "list-z"])
    );
    assert_eq!(resp["page"]["properties"]["tags"], json!(["page-list-tag"]));
    assert!(doc_page_named("list-y").await.is_none());
}

// 定義のないキーには、ページの値を書けない。名前に合う型でも、合わない型でも同じである（ADR 047）。
#[tokio::test]
async fn a_key_with_no_definition_takes_no_value_of_a_page() {
    let page = new_doc_page("page-type-by-name").await;

    for (key, body) in [
        (
            "undefined_page_note",
            json!({ "valueType": "text", "value": "x" }),
        ),
        (
            "undefined_page_score_n",
            json!({ "valueType": "number", "value": "1" }),
        ),
        (
            "undefined_page_members",
            json!({ "valueType": "list", "value": "x" }),
        ),
        (
            "undefined_page_note_list",
            json!({ "valueType": "text", "value": "x" }),
        ),
    ] {
        let (status, resp) = put(
            &format!("/api/pages/{}/properties/{key}", page.id),
            body.clone(),
        )
        .await;
        assert_eq!(status, 400, "{key} {body} -> {resp}");
    }
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

// タグページをNFDのタイトルへ改名すると、本文とタグにはNFCの名前が入る。
#[tokio::test]
async fn renaming_a_tag_page_to_an_nfd_title_writes_the_tag_in_nfc() {
    let _tags_dir = TAGS_DIR.lock().await;
    let old = format!("nfd-tag-old-{}", nanos());
    let n = nanos();
    let page = new_doc_page("nfd-tag-body").await;
    let item = items(&page.id).await.into_iter().next().unwrap();
    update_item(&item.id, json!({ "body": format!("see #{old}") })).await;
    let tag_page = doc_page_named(&old).await.unwrap();

    let (status, resp) = put(
        &format!("/api/pages/{tag_page}"),
        json!({ "title": format!("\u{30ab}\u{3099}-{n}") }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let new = format!("\u{30ac}-{n}");
    let item = items(&page.id).await.into_iter().next().unwrap();
    assert_eq!(item.body, format!("see #{new}"));
    assert_eq!(item.properties["tags"], json!([new]));
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
    define_key("item_list").await;
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
    define_key("item_list").await;
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
    define_key("children_list").await;
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
    define_key("rating_n").await;
    let page = new_doc_page("definition-delete").await;
    let item = fill(&page.id, &["rated"]).await.remove(0);
    let (status, resp) = put(
        &format!("/api/items/{}/properties/rating_n", item.id),
        json!({ "valueType": "number", "value": "5" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/rating_n", page.id),
        json!({ "valueType": "number", "value": "3" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["properties"]["rating_n"], json!("3"));
    let modified_before = resp["page"]["modifiedDate"].as_i64().unwrap();

    let (status, resp) = send(
        reqwest::Method::DELETE,
        "/api/config/property-definitions/rating_n",
        None,
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert!(definition(&resp, "rating_n").is_none(), "{resp}");

    assert!(items(&page.id).await[0]
        .properties
        .get("rating_n")
        .is_none());
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert!(
        resp["page"]["properties"].get("rating_n").is_none(),
        "{resp}"
    );
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
        "/api/config/property-definitions/rating_n",
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
        json!({ "key": "area_bounds", "valueType": "bounds" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(
        definition(&resp, "area_bounds").unwrap()["valueType"],
        json!("bounds")
    );

    let page = new_doc_page("bounds").await;
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/area_bounds", page.id),
        json!({ "valueType": "bounds", "value": " 10, 20.5,160 ,-56 " }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(
        resp["page"]["properties"]["area_bounds"],
        json!("10,20.5,160,-56")
    );
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

// コールアウトの文が閉じていないブロックで終われば、コールアウトは足されない（ADR 050）。
#[tokio::test]
async fn a_callout_with_an_unclosed_block_is_refused() {
    let (_, atlas_id) = atlas_dir("atlas-callout-unclosed").await;
    let before = outline_of(&atlas_id).await;

    let (status, resp) = post(
        &format!("/api/pages/{atlas_id}/atlas/callouts"),
        json!({ "bounds": "10,20,0,60", "side": "left", "text": "```\nabc" }),
    )
    .await;

    assert_eq!(status, 400, "{resp}");
    assert_eq!(outline_of(&atlas_id).await, before);
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

// dateとdatetimeの値も、型に合うかが検査される。dateは年月日、datetimeはRFC 3339の日時である。
#[tokio::test]
async fn a_date_and_a_datetime_are_checked_by_their_types() {
    define_key("checked_due_d").await;
    define_key("checked_seen_dt").await;
    let page = new_doc_page("checked-dates").await;
    let item = fill(&page.id, &["dated"]).await.remove(0);
    let set = |key: &'static str, value: &'static str| {
        let id = item.id.clone();
        async move {
            put(
                &format!("/api/items/{id}/properties/{key}"),
                json!({ "value": value }),
            )
            .await
            .0
        }
    };

    assert_eq!(set("checked_due_d", "2026-03-01").await, 200);
    assert_eq!(set("checked_seen_dt", "2026-03-01T10:00:00Z").await, 200);
    for (key, value) in [
        ("checked_due_d", "soon"),
        ("checked_due_d", "2026-13-01"),
        ("checked_seen_dt", "2026-03-01"),
        ("checked_seen_dt", "2026-03-01 10:00"),
    ] {
        assert_eq!(set(key, value).await, 400, "{key} {value}");
    }
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

/// The HMTID and the extension in the name the server gave an upload.
fn id_and_extension(file_name: &str) -> (hmtid::Hmtid, Option<&str>) {
    let (id, extension) = match file_name.rsplit_once('.') {
        Some((id, extension)) => (id, Some(extension)),
        None => (file_name, None),
    };
    (hmtid::Hmtid::parse(id).expect(file_name), extension)
}

// 送ったファイルは、HMTIDと元の拡張子でできた名前で、中身のまま保存される。拡張子は小文字になる。
// 答えには、保存した名前と、元の名前がそのまま入る。
#[tokio::test]
async fn an_upload_is_saved_under_an_hmtid_with_the_extension_of_its_name() {
    let response = upload_asset("構成図 v2.PNG", b"drawing".to_vec())
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
    let answer: Value = response.json().await.unwrap();
    let file_name = answer["fileName"].as_str().unwrap();
    let (_, extension) = id_and_extension(file_name);
    assert_eq!(extension, Some("png"));
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

// 拡張子のない名前と、拡張子がASCIIの英数字でない名前のファイルは、HMTIDだけの名前で保存される。
#[tokio::test]
async fn an_upload_without_a_plain_extension_is_saved_under_the_hmtid_alone() {
    for name in ["README", "memo.下書き", "notes."] {
        let response = upload_asset(name, b"text".to_vec()).await.unwrap();
        let answer: Value = response.json().await.unwrap();
        let file_name = answer["fileName"].as_str().unwrap();

        let (_, extension) = id_and_extension(file_name);
        assert_eq!(extension, None, "{name} was saved as {file_name}");
        assert_eq!(std::fs::read(saved_asset(file_name)).unwrap(), b"text");
    }
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
