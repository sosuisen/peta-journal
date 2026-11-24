//! Every route of the REST API answers a request in order (ADR 058). The rules behind the
//! routes are tested in journal-core, through the entrance the routes call; this file only
//! sees that each route reaches it and answers in the shape a client reads.
//!
//! Left out: `/api/events` (a stream, see `the_clients_hear_which_client_changed_the_page`),
//! the routes that open a window of the OS (`/api/config/browse-folder`,
//! `/api/pages/{id}/open-folder`), the uploads and the files of
//! `/assets` (the `Assets` tests), the page of the pending deletions (a form of HTML, see
//! `client_id.rs`). `/api/confirm/{id}` answers a question the test asks through the state of
//! the server, as no extension is installed here.

use reqwest::Method;
use serde_json::{json, Value};

use crate::common::*;

/// Sends a request that is in order and gives the answer, which must hold the keys.
async fn answers(method: Method, path: &str, body: Option<Value>, keys: &[&str]) -> Value {
    let (status, resp) = send(method.clone(), path, body).await;
    assert_eq!(status, 200, "{method} {path}: {resp}");
    for key in keys {
        assert!(
            resp.get(key).is_some(),
            "{method} {path} has no {key}: {resp}"
        );
    }
    resp
}

async fn get_answers(path: &str, keys: &[&str]) -> Value {
    answers(Method::GET, path, None, keys).await
}

async fn post_answers(path: &str, body: Value, keys: &[&str]) -> Value {
    answers(Method::POST, path, Some(body), keys).await
}

async fn put_answers(path: &str, body: Value, keys: &[&str]) -> Value {
    answers(Method::PUT, path, Some(body), keys).await
}

async fn delete_answers(path: &str, keys: &[&str]) -> Value {
    answers(Method::DELETE, path, None, keys).await
}

fn id_of(v: &Value) -> String {
    v["id"].as_str().unwrap().to_string()
}

// クエリのdateFormat=isoを受けたルートは、日付をUTCのISO 8601の文字列で返す。クエリがなければ、ミリ秒の数で返す。
#[tokio::test]
async fn a_route_writes_its_dates_as_iso_text_only_when_the_query_asks() {
    let page = new_doc_page("routes-iso-dates").await;
    let path = format!("/api/pages/{}", page.id);

    let (_, by_default) = get(&path).await;
    let (_, in_iso) = get(&format!("{path}?dateFormat=iso")).await;
    let (_, in_millis) = get(&format!("{path}?dateFormat=millis")).await;

    let millis = by_default["page"]["modifiedDate"].as_i64().unwrap();
    let written = in_iso["page"]["modifiedDate"].as_str().unwrap();
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(written)
            .unwrap()
            .timestamp_millis(),
        millis
    );
    assert!(in_iso["items"][0]["createdDate"].is_string(), "{in_iso}");
    assert_eq!(in_iso["items"][0]["level"], by_default["items"][0]["level"]);
    assert_eq!(in_millis, by_default);
}

// 変更のルートも、同じクエリで日付の形式を受ける。
#[tokio::test]
async fn a_route_that_changes_something_takes_the_date_format_too() {
    let page = new_doc_page("routes-iso-dates-change").await;

    let (status, created) = post(
        "/api/items?dateFormat=iso",
        json!({ "pageId": page.id, "position": "last", "level": 1, "body": "x" }),
    )
    .await;

    assert_eq!(status, 200, "{created}");
    assert!(created["item"]["createdDate"].is_string(), "{created}");
}

// 知らないdateFormatは、何もせずに400で拒否される。
#[tokio::test]
async fn a_date_format_that_is_not_known_is_refused_before_anything_changes() {
    let page = new_doc_page("routes-bad-date-format").await;

    let (status, refusal) = post(
        "/api/items?dateFormat=rfc",
        json!({ "pageId": page.id, "position": "last", "level": 1, "body": "x" }),
    )
    .await;

    assert_eq!(status, 400, "{refusal}");
    assert!(refusal["error"].is_string(), "{refusal}");
    assert_eq!(outline_of(&page.id).await, vec!["1:"]);
}

// JSONでない答え（イベントの流れ）は、dateFormatがあっても、そのまま流れる。
#[tokio::test]
async fn an_answer_that_is_not_json_passes_as_it_is_with_a_date_format() {
    let events = client()
        .get(url("/api/events?dateFormat=iso"))
        .send()
        .await
        .unwrap();

    assert_eq!(events.status(), 200);
    assert!(events.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("text/event-stream"));
}

// 項目のルートは、正しい要求に200で答え、クライアントが読む形の答えを返す。
#[tokio::test]
async fn every_route_of_the_items_answers_a_request_in_order() {
    let page = new_doc_page("routes-items").await;
    let other = new_doc_page("routes-items-other").await;
    let rows = fill(&page.id, &["a", "b", "c", "d", "e", "f", "g"]).await;
    let [a, b, c, d, e, f, g]: [String; 7] = rows
        .into_iter()
        .map(|r| r.id)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let made = post_answers(
        "/api/items",
        json!({ "pageId": page.id, "position": "last", "level": 1, "body": "made", "id": "" }),
        &["item"],
    )
    .await;
    let made = id_of(&made["item"]);
    get_answers("/api/items?types=task", &["items"]).await;
    get_answers(&format!("/api/items/{a}"), &["item"]).await;
    put_answers(
        &format!("/api/items/{a}"),
        json!({ "body": "a2" }),
        &["item"],
    )
    .await;
    assert_eq!(delete(&format!("/api/items/{made}")).await, 204);

    post_answers(
        "/api/items/move",
        json!({ "itemIds": [g], "targetPageId": other.id, "position": "last" }),
        &["items"],
    )
    .await;
    post_answers(
        "/api/items/merge",
        json!({ "itemId": f, "intoId": e }),
        &["item"],
    )
    .await;
    post_answers("/api/items/delete", json!({ "itemIds": [e] }), &["created"]).await;
    post_answers(
        "/api/items/batch",
        json!({ "ops": [{ "op": "update", "id": d, "body": "d2" }] }),
        &["items"],
    )
    .await;
    post_answers(
        "/api/items/shift-level",
        json!({ "itemIds": [c], "delta": 1 }),
        &["items"],
    )
    .await;

    post_answers(
        &format!("/api/items/{d}/transclude"),
        json!({ "sourceItemId": a }),
        &["item"],
    )
    .await;
    post_answers(
        "/api/items/transcluded",
        json!({ "itemIds": [a] }),
        &["itemIds"],
    )
    .await;
    delete_answers(&format!("/api/items/{d}/transclude"), &["item"]).await;

    post_answers(
        &format!("/api/items/{b}/paste"),
        json!({ "body": "b", "cursor": 1, "text": "x", "mode": "auto" }),
        &["items"],
    )
    .await;
    post_answers(
        &format!("/api/items/{b}/split"),
        json!({ "body": "bx", "cursor": 1 }),
        &["created"],
    )
    .await;
    post_answers(&format!("/api/items/{c}/stash"), json!({}), &["items"]).await;
    answers(
        Method::POST,
        &format!("/api/items/{a}/cycle-task"),
        None,
        &["item"],
    )
    .await;
    put_answers(
        &format!("/api/items/{a}/properties/tags"),
        json!({ "valueType": "tags", "value": "routes" }),
        &["item"],
    )
    .await;
    delete_answers(&format!("/api/items/{a}/properties/tags"), &["item"]).await;
}

// Markdownとの変換のルートは、正しい要求に200で答える。
#[tokio::test]
async fn every_route_of_the_markdown_conversions_answers_a_request_in_order() {
    let page = new_doc_page("routes-markdown").await;
    let rows = fill(&page.id, &["a", "  b"]).await;

    post_answers(
        "/api/items/export",
        json!({ "itemIds": [rows[0].id, rows[1].id] }),
        &["treeMarkdown"],
    )
    .await;
    let (status, _) = query_post("/api/items/export-page", "pageId", &page.id).await;
    assert_eq!(status, 200);
    let (status, _) = query_post("/api/items/export-subtree", "itemId", &rows[0].id).await;
    assert_eq!(status, 200);
    post_answers(
        "/api/items/import",
        json!({ "treeMarkdown": "- a\n    - b\n" }),
        &["flatMarkdown"],
    )
    .await;
    post_answers("/api/items/parse", json!({ "markdown": "x" }), &["items"]).await;
}

/// A POST whose input goes in the query, as the export of a page and of a subtree take it.
async fn query_post(path: &str, name: &str, value: &str) -> (u16, Value) {
    let response = client()
        .post(url(path))
        .query(&[(name, value)])
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    (status, response.json().await.unwrap_or(Value::Null))
}

// ページのルートは、正しい要求に200で答え、クライアントが読む形の答えを返す。
#[tokio::test]
async fn every_route_of_the_pages_answers_a_request_in_order() {
    let title = unique_name("routes-pages");
    let made = post_answers(
        "/api/pages/doc",
        json!({ "title": title, "dirPath": "" }),
        &["page", "items"],
    )
    .await;
    let page = id_of(&made["page"]);
    let child = post_answers(
        "/api/pages/doc",
        json!({ "title": "child", "dirPath": format!("{title}/") }),
        &["page"],
    )
    .await;
    let child = id_of(&child["page"]);

    get_answers("/api/pages/recent?maxResults=5&type=doc", &["pages"]).await;
    get_answers(
        &format!("/api/pages/find?title={title}&dirPath="),
        &["page"],
    )
    .await;
    get_answers(&format!("/api/pages/{page}"), &["page", "items"]).await;
    get_answers(&format!("/api/pages/{page}/backlinks"), &["entries"]).await;
    let index = get_answers(&format!("/api/pages/{child}"), &["page"]).await;
    let index = index["page"]["parentId"].as_str().unwrap().to_string();
    get_answers(&format!("/api/pages/{index}/children"), &["pages"]).await;
    put_answers(
        &format!("/api/pages/{child}/properties/tags"),
        json!({ "valueType": "tags", "value": "routes" }),
        &["page"],
    )
    .await;
    delete_answers(&format!("/api/pages/{child}/properties/tags"), &["page"]).await;
    put_answers(
        &format!("/api/pages/{child}"),
        json!({ "title": "child2" }),
        &["page"],
    )
    .await;
    assert_eq!(delete(&format!("/api/pages/{child}")).await, 204);
    post_answers(
        &format!("/api/pages/{index}/demote"),
        json!({ "overwrite": false }),
        &["page"],
    )
    .await;
}

// アトラスのルートは、正しい要求に200で答え、アトラスの項目を返す。
#[tokio::test]
async fn every_route_of_the_atlas_answers_a_request_in_order() {
    let dir = format!("{}/", unique_name("routes-atlas"));
    let atlas = post_answers(
        "/api/pages/doc",
        json!({ "title": "index_atlas", "dirPath": dir }),
        &["page"],
    )
    .await;
    let atlas = id_of(&atlas["page"]);
    let at = |path: &str| format!("/api/pages/{atlas}/atlas/{path}");
    let link = |target: &str| json!({ "target": target, "bounds": "0,0,0,40" });

    let rows = post_answers(&at("links"), link("elsewhere/a"), &["items"]).await;
    let first = id_of(rows["items"].as_array().unwrap().last().unwrap());
    let rows = post_answers(&at("links"), link("elsewhere/b"), &["items"]).await;
    let second = id_of(rows["items"].as_array().unwrap().last().unwrap());
    put_answers(
        &at(&format!("links/{first}")),
        json!({ "target": "far/a" }),
        &["items"],
    )
    .await;
    let rows = post_answers(
        &at("group"),
        json!({ "cards": [second], "target": first }),
        &["items"],
    )
    .await;
    let group = rows["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] != json!(first) && r["id"] != json!(second))
        .map(id_of)
        .unwrap();
    put_answers(
        &at(&format!("groups/{group}")),
        json!({ "side": "left", "title": "T" }),
        &["items"],
    )
    .await;
    post_answers(&at("ungroup"), json!({ "cards": [second] }), &["items"]).await;
    let rows = post_answers(
        &at("callouts"),
        json!({ "bounds": "10,20,0,60", "side": "left", "text": "Hello" }),
        &["items"],
    )
    .await;
    let callout = rows["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["body"].as_str().unwrap_or_default().contains("Hello"))
        .map(id_of)
        .unwrap();
    put_answers(
        &at(&format!("callouts/{callout}")),
        json!({ "side": "bottom", "text": "Bye" }),
        &["items"],
    )
    .await;
    delete_answers(&at(&format!("items/{callout}")), &["items"]).await;
}

// ジャーナル、検索、設定、同期のルートは、正しい要求に200で答える。
#[tokio::test]
async fn every_route_of_the_journals_the_search_the_config_and_the_sync_answers_a_request_in_order()
{
    let day = answers(Method::POST, "/api/journals/1997-04-02", None, &["page"]).await;
    let day = id_of(&day["page"]);
    let rows = fill(&day, &["go"]).await;
    post_answers(
        "/api/journals/1997-04-03/move",
        json!({ "itemIds": [rows[0].id], "position": "last" }),
        &["items"],
    )
    .await;
    get_answers("/api/journals/before?date=1997-04-05", &["found"]).await;
    get_answers("/api/journals/weekly/before?week=1997-W20", &["found"]).await;

    get_answers("/api/search/pages?q=1997&maxResults=5", &["results"]).await;
    get_answers("/api/search/combined?q=go&maxResults=5", &["results"]).await;
    get_answers("/api/search/tags?q=routes&maxResults=5", &["results"]).await;

    get_answers("/api/health", &[]).await;
    get_answers("/api/config", &["markdownDir"]).await;
    put_answers("/api/config", json!({}), &["markdownDir"]).await;
    get_answers("/api/config/property-definitions", &["propertyDefinitions"]).await;
    let key = format!("{}_n", unique_name("routes").replace('-', "_"));
    post_answers(
        "/api/config/property-definitions",
        json!({ "key": key }),
        &["propertyDefinitions"],
    )
    .await;
    delete_answers(
        &format!("/api/config/property-definitions/{key}"),
        &["propertyDefinitions"],
    )
    .await;

    get_answers("/api/sync/status", &["markdownRoot"]).await;
    answers(Method::POST, "/api/sync/scan", None, &["imported"]).await;
}

// 待っていない問い合わせへの答えは、404になる。
#[tokio::test]
async fn an_answer_to_a_question_that_no_one_waits_for_is_not_found() {
    let (status, _) = post("/api/confirm/unknown", json!({ "ok": true })).await;

    assert_eq!(status, 404);
}

// 拡張の問い合わせが待っているあいだ、`POST /api/confirm/{id}`は200で答え、問い合わせはその答えを受け取る。
#[tokio::test]
async fn the_confirm_route_answers_a_question_that_waits() {
    let confirm = server().state.confirm.clone();
    let mut events = client().get(url("/api/events")).send().await.unwrap();
    let message = unique_name("routes-confirm");
    let asked = message.clone();
    let asking = std::thread::spawn(move || confirm.confirm(&asked));
    let question = event_with(&mut events, &message).await;
    let request_id = question["requestId"].as_str().unwrap();

    let (status, _) = post(&format!("/api/confirm/{request_id}"), json!({ "ok": true })).await;

    assert_eq!(status, 200);
    assert!(asking.join().unwrap());
}
