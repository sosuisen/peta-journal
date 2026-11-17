//! The API tests of the Java server, run against the Rust server.

use crate::common::*;
use serde_json::{json, Value};

fn vs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

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

// ---- Health and config ----

// ヘルスチェックはokを返す。設定の読み書きはjournal-coreのテストが確かめる。
#[tokio::test]
async fn health_is_ok() {
    let (status, resp) = get("/api/health").await;
    assert_eq!(status, 200);
    assert_eq!(resp["status"], json!("ok"));
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
