//! The MCP tools, called over Streamable HTTP at `/mcp` the way an MCP client does. Pages and
//! items are set up over the REST API where they are not what the test is about.
//!
//! These tests are a binary of their own, apart from the REST tests in `it`. One test here
//! changes the config, such as the week numbering. The REST tests read the same config, so the
//! two must not share a server.

#[path = "../common/mod.rs"]
mod common;
mod rest_only;
mod table;

use common::*;
use rmcp::model::CallToolRequestParam;
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransport;
use rmcp::{RoleClient, ServiceExt};
use serde_json::{json, Value};

type Mcp = RunningService<RoleClient, ()>;

async fn connect() -> Mcp {
    let transport = StreamableHttpClientTransport::from_uri(url("/mcp"));
    ().serve(transport).await.expect("mcp client")
}

/// Calls a tool and parses the JSON it answers with.
async fn call(mcp: &Mcp, name: &'static str, args: Value) -> Value {
    try_call(mcp, name, args)
        .await
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The error message of a tool call that is refused.
async fn call_err(mcp: &Mcp, name: &'static str, args: Value) -> String {
    match try_call(mcp, name, args).await {
        Ok(v) => panic!("{name} answered {v} instead of an error"),
        Err(e) => e,
    }
}

async fn try_call(mcp: &Mcp, name: &'static str, args: Value) -> Result<Value, String> {
    let params = CallToolRequestParam {
        name: name.into(),
        arguments: args.as_object().cloned(),
    };
    let result = mcp.call_tool(params).await.map_err(|e| e.to_string())?;
    let text = result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.clone())
        .unwrap_or_default();
    Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
}

// ---- The server ----

// サーバーは、接続したクライアントに、ツールを持つことと、何のサーバーかの説明を伝える。
#[tokio::test]
async fn the_server_tells_a_client_it_has_tools_and_what_it_is_for() {
    let mcp = connect().await;

    let info = mcp.peer_info().expect("server info");

    assert!(info.capabilities.tools.is_some(), "{info:?}");
    let instructions = info.instructions.as_deref().unwrap_or_default();
    assert!(instructions.starts_with("PetaJournal"), "{instructions}");
}

// 項目を変えるツールの引数は、項目のIDと、RESTのルートが本文で受ける欄である。
#[tokio::test]
async fn a_tool_that_changes_an_item_takes_its_id_and_the_fields_of_the_rest_body() {
    let mcp = connect().await;

    let tools = mcp.list_all_tools().await.expect("tools");
    let update = tools.iter().find(|t| t.name == "update_item").unwrap();
    let mut arguments: Vec<&String> = update.input_schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    arguments.sort_unstable();

    assert_eq!(
        arguments,
        ["body", "collapsed", "dateFormat", "itemId", "level"]
    );
    assert_eq!(update.input_schema["required"], json!(["itemId"]));
}

/// The answer of a REST route as a tool gives it: without the AST of its items.
fn without_asts(mut answer: Value) -> Value {
    for item in answer["items"].as_array_mut().into_iter().flatten() {
        item.as_object_mut().unwrap().remove("ast");
    }
    answer
}

// ツールは、項目のASTを除いて、同じ操作のRESTのルートと同じJSONで答える。日付はミリ秒の数で、プロパティも入る。
#[tokio::test]
async fn a_tool_answers_with_the_json_of_its_rest_route() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-same-json").await;
    fill(&page.id, &["a #mcp"]).await;

    let by_tool = call(&mcp, "get_page_by_id", json!({ "pageId": page.id })).await;
    let by_rest: Value = client()
        .get(url(&format!("/api/pages/{}", page.id)))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(by_rest["items"][0]["ast"].is_string(), "{by_rest}");
    assert_eq!(by_tool, without_asts(by_rest));
    assert!(by_tool["page"]["modifiedDate"].is_number(), "{by_tool}");
    assert_eq!(by_tool["items"][0]["properties"]["tags"], json!(["mcp"]));
}

// dateFormatにisoを渡すと、ツールは日付をUTCのISO 8601の文字列で答える。RESTの同じクエリと同じ答えになる。
#[tokio::test]
async fn a_tool_writes_its_dates_as_iso_text_when_asked_as_its_rest_route_does() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-iso-dates").await;

    let by_tool = call(
        &mcp,
        "get_page_by_id",
        json!({ "pageId": page.id, "dateFormat": "iso" }),
    )
    .await;
    let (_, by_rest) = get(&format!("/api/pages/{}?dateFormat=iso", page.id)).await;

    let written = by_tool["page"]["modifiedDate"].as_str().unwrap_or_default();
    assert!(
        chrono::DateTime::parse_from_rfc3339(written).is_ok() && written.ends_with('Z'),
        "{by_tool}"
    );
    assert!(by_tool["items"][0]["createdDate"].is_string(), "{by_tool}");
    assert_eq!(by_tool, without_asts(by_rest));
}

// 知らないdateFormatは、何もせずに拒否される。
#[tokio::test]
async fn a_date_format_that_is_not_known_is_refused_before_anything_changes() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-bad-date-format").await;

    let error = call_err(
        &mcp,
        "create_item",
        json!({ "pageId": page.id, "position": "last", "level": 1, "body": "x", "dateFormat": "rfc" }),
    )
    .await;

    assert!(
        error.contains("dateFormat") || error.contains("rfc"),
        "{error}"
    );
    assert_eq!(outline_of(&page.id).await, vec!["1:"]);
}

// 不正なIDは、invalid paramsとして拒否される。
#[tokio::test]
async fn a_bad_id_is_refused_as_invalid_params() {
    let mcp = connect().await;

    let error = call_err(&mcp, "get_item", json!({ "itemId": "nope", "fields": "" })).await;

    assert!(error.contains("Invalid HMTID"), "{error}");
}

// ---- Items ----

// 項目は作成、取得、更新、削除できる。
#[tokio::test]
async fn an_item_is_created_read_updated_and_deleted() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-item").await;
    let a = fill(&page.id, &["a"]).await.remove(0);

    let created = call(
        &mcp,
        "create_item",
        json!({ "pageId": page.id, "position": format!("after:{}", a.id), "level": 2, "body": "b #mcp" }),
    )
    .await;
    let id = created["item"]["id"].as_str().unwrap().to_string();
    assert_eq!(created["item"]["level"], json!(2));
    assert_eq!(outline_of(&page.id).await, vec!["1:a", "2:b #mcp"]);

    // Only the fields asked for, else the item with its properties
    let fields = call(
        &mcp,
        "get_item",
        json!({ "itemId": id, "fields": "body,level" }),
    )
    .await;
    assert_eq!(fields, json!({ "item": { "body": "b #mcp", "level": 2 } }));
    let whole = call(&mcp, "get_item", json!({ "itemId": id, "fields": "" })).await;
    assert_eq!(whole["item"]["properties"]["tags"], json!(["mcp"]));
    // An AI reads the body: no tool gives the AST, also when it is asked for by name
    assert!(whole["item"].get("ast").is_none(), "{whole}");
    let asked = call(
        &mcp,
        "get_item",
        json!({ "itemId": id, "fields": "body,ast" }),
    )
    .await;
    assert_eq!(asked, json!({ "item": { "body": "b #mcp" } }));

    // A field that is left out stays as it is
    let updated = call(
        &mcp,
        "update_item",
        json!({ "itemId": id, "body": "c", "collapsed": true }),
    )
    .await;
    assert_eq!(updated["item"]["body"], json!("c"));
    assert_eq!(updated["item"]["level"], json!(2));
    assert_eq!(updated["item"]["collapsed"], json!(true));

    let outdented = call(&mcp, "update_item", json!({ "itemId": id, "level": 1 })).await;
    assert_eq!(outdented["item"]["level"], json!(1));

    let deleted = call(&mcp, "delete_item", json!({ "itemId": id })).await;
    assert_eq!(deleted, json!({ "deleted": true }));
    let error = call_err(&mcp, "get_item", json!({ "itemId": id, "fields": "" })).await;
    assert!(error.contains("Item not found"), "{error}");
}

// 項目は、レベルをずらして、またはそのままで、ページの間を移動する。
#[tokio::test]
async fn items_move_between_pages_with_their_levels_shifted_or_kept() {
    let mcp = connect().await;
    let source = new_doc_page("mcp-move-from").await;
    let target = new_doc_page("mcp-move-to").await;
    let rows = fill(&source.id, &["a", "  b"]).await;
    let x = fill(&target.id, &["x"]).await.remove(0);
    let ab = json!([rows[0].id, rows[1].id]);

    let moved = call(
        &mcp,
        "move_items",
        json!({ "itemIds": ab, "targetPageId": target.id, "position": "last", "levelDelta": 1 }),
    )
    .await;
    assert_eq!(
        field_of_each(&moved, "items", "level"),
        vec![json!(2), json!(3)]
    );
    assert_eq!(outline_of(&target.id).await, vec!["1:x", "2:a", "3:b"]);
    // The page keeps one empty item (ADR 021)
    assert_eq!(outline_of(&source.id).await, vec!["1:"]);

    call(
        &mcp,
        "move_items",
        json!({ "itemIds": ab, "targetPageId": source.id, "position": "first", "levelDelta": -1 }),
    )
    .await;
    assert_eq!(outline_of(&source.id).await, vec!["1:a", "2:b", "1:"]);

    // No levelDelta keeps the levels
    call(
        &mcp,
        "move_items",
        json!({ "itemIds": [x.id], "targetPageId": source.id, "position": "last" }),
    )
    .await;
    assert_eq!(
        outline_of(&source.id).await,
        vec!["1:a", "2:b", "1:", "1:x"]
    );
    assert_eq!(outline_of(&target.id).await, vec!["1:"]);
}

// タスクは状態を進められ、型で一覧される。
#[tokio::test]
async fn a_task_is_cycled_and_listed_by_its_type() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-task").await;
    let item = fill(&page.id, &["todo"]).await.remove(0);

    let task = call(&mcp, "cycle_task", json!({ "itemId": item.id })).await;
    let begun = call(&mcp, "cycle_task", json!({ "itemId": item.id })).await;
    let tasks = call(&mcp, "get_items_by_type", json!({ "types": "task" })).await;

    assert_eq!(task["item"]["types"], json!(["task"]));
    assert!(task["item"]["properties"]["begin"].is_null());
    assert!(begun["item"]["properties"]["begin"].is_string());
    assert!(
        field_of_each(&tasks, "items", "id").contains(&json!(item.id)),
        "{tasks}"
    );
}

// トランスクルージョンは、解除されるまで参照元を映す。
#[tokio::test]
async fn a_transclusion_mirrors_the_source_until_it_is_removed() {
    let mcp = connect().await;
    let source_page = new_doc_page("mcp-transclusion-source").await;
    let page = new_doc_page("mcp-transclusion").await;
    let source = fill(&source_page.id, &["the source"]).await.remove(0);
    let item = fill(&page.id, &["placeholder"]).await.remove(0);

    let set = call(
        &mcp,
        "set_transclusion",
        json!({ "itemId": item.id, "sourceItemId": source.id }),
    )
    .await;
    let removed = call(&mcp, "remove_transclusion", json!({ "itemId": item.id })).await;

    assert_eq!(set["item"]["body"], json!("the source"));
    assert_eq!(set["item"]["transcludeItemId"], json!(source.id));
    assert_eq!(removed["item"]["body"], json!(""));
    assert!(removed["item"]["transcludeItemId"].is_null());
}

// 項目にプロパティが設定される。
#[tokio::test]
async fn a_property_is_set_on_an_item() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-property").await;
    let item = fill(&page.id, &["planned"]).await.remove(0);

    let updated = call(
        &mcp,
        "upsert_property",
        json!({ "itemId": item.id, "key": "plan", "valueType": "date", "value": "2026-03-01" }),
    )
    .await;

    assert_eq!(updated["item"]["properties"]["plan"], json!("2026-03-01"));
    assert_eq!(
        items(&page.id).await[0].properties["plan"],
        json!("2026-03-01")
    );
}

// 項目は木のMarkdownとしてエクスポートされ、平らなMarkdownとしてインポートされる。
#[tokio::test]
async fn items_are_exported_as_tree_markdown_and_imported_back_as_flat_markdown() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-export").await;
    let rows = fill(&page.id, &["a", "  b"]).await;

    let by_items = call(
        &mcp,
        "export_tree_markdown",
        json!({ "itemIds": [rows[0].id, rows[1].id] }),
    )
    .await;
    let subtree = call(
        &mcp,
        "export_subtree_markdown",
        json!({ "itemId": rows[0].id }),
    )
    .await;
    let by_page = call(&mcp, "export_page_markdown", json!({ "pageId": page.id })).await;
    let imported = call(
        &mcp,
        "import_tree_markdown",
        json!({ "treeMarkdown": by_items["treeMarkdown"] }),
    )
    .await;

    assert_eq!(by_items["treeMarkdown"], json!("- a\n  - b"));
    assert_eq!(subtree["treeMarkdown"], by_items["treeMarkdown"]);
    assert_eq!(by_page["treeMarkdown"], by_items["treeMarkdown"]);
    // An AI reads the Markdown: the export carries no AST
    assert!(by_items.get("ast").is_none(), "{by_items}");
    // The levels become the item markers; the items have no ids yet
    assert_eq!(imported["flatMarkdown"], json!("a\n\n<!-- @@ -->\nb"));
}

// ---- Telling the clients ----

/// The event stream, opened before the change the test is about.
async fn listen() -> reqwest::Response {
    client().get(url("/api/events")).send().await.unwrap()
}

/// Waits for the event that tells of a change of the page.
async fn told_of(events: &mut reqwest::Response, page_id: &str) -> Value {
    event_with(events, &format!("\"pageId\":\"{page_id}\"")).await
}

// 項目を変えるツールのどれも、その項目のページの変更を知らせる。
#[tokio::test]
async fn the_clients_hear_of_each_change_of_an_item_over_mcp() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-tell-item").await;
    let other = new_doc_page("mcp-tell-item-other").await;
    let rows = fill(&page.id, &["a", "source"]).await;
    let (item, source) = (&rows[0].id, &rows[1].id);
    let changes = [
        (
            "create_item",
            json!({ "pageId": page.id, "position": "last", "level": 1, "body": "c" }),
        ),
        ("update_item", json!({ "itemId": item, "body": "b" })),
        ("cycle_task", json!({ "itemId": item })),
        (
            "upsert_property",
            json!({ "itemId": item, "key": "plan", "valueType": "date", "value": "2026-03-01" }),
        ),
        (
            "set_transclusion",
            json!({ "itemId": item, "sourceItemId": source }),
        ),
        ("remove_transclusion", json!({ "itemId": item })),
        (
            "move_items",
            json!({ "itemIds": [item], "targetPageId": other.id, "position": "last" }),
        ),
        ("delete_item", json!({ "itemId": source })),
    ];

    for (tool, args) in changes {
        let mut events = listen().await;

        call(&mcp, tool, args).await;

        assert_eq!(
            told_of(&mut events, &page.id).await["source"],
            "mcp",
            "{tool}"
        );
    }
}

// ---- Pages ----

// docページは作成、再取得、改名、削除できる。
#[tokio::test]
async fn a_doc_page_is_created_found_again_renamed_and_deleted() {
    let mcp = connect().await;
    let title = format!("mcp-doc-{}-{}", std::process::id(), nanos());

    let created = call(
        &mcp,
        "get_or_create_doc",
        json!({ "title": title, "dirPath": "" }),
    )
    .await;
    let page_id = created["page"]["id"].as_str().unwrap().to_string();
    let again = call(
        &mcp,
        "get_or_create_doc",
        json!({ "title": title, "dirPath": "" }),
    )
    .await;
    let got = call(&mcp, "get_page_by_id", json!({ "pageId": page_id })).await;
    let renamed = call(
        &mcp,
        "update_page",
        json!({ "pageId": page_id, "title": format!("{title}-renamed"), "overwrite": false }),
    )
    .await;
    let deleted = call(&mcp, "delete_page", json!({ "pageId": page_id })).await;
    let error = call_err(&mcp, "get_page_by_id", json!({ "pageId": page_id })).await;

    assert_eq!(created["created"], json!(true));
    assert_eq!(field_of_each(&created, "items", "body"), vec![json!("")]);
    assert_eq!(again["created"], json!(false));
    assert_eq!(again["page"]["id"], json!(page_id));
    assert_eq!(got["page"]["name"], json!(title));
    assert_eq!(got["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(renamed["page"]["name"], json!(format!("{title}-renamed")));
    assert_eq!(deleted, json!({ "deleted": true }));
    assert!(error.contains("Page not found"), "{error}");
}

// ページを改名しても消しても、そのページの変更がクライアントに知らされる。
#[tokio::test]
async fn the_clients_hear_of_a_page_renamed_or_deleted_over_mcp() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-tell-page").await;
    let changes = [
        (
            "update_page",
            json!({ "pageId": page.id, "title": format!("{}-renamed", page.title) }),
        ),
        ("delete_page", json!({ "pageId": page.id })),
    ];

    for (tool, args) in changes {
        let mut events = listen().await;

        call(&mcp, tool, args).await;

        assert_eq!(
            told_of(&mut events, &page.id).await["source"],
            "mcp",
            "{tool}"
        );
    }
}

// 最近のページでは、最後に触ったページが先頭に来る。
#[tokio::test]
async fn the_most_recently_touched_page_comes_first_among_recent_pages() {
    let mcp = connect().await;
    // The default of maxResults is 10, so the test makes enough pages on its own
    for i in 0..10 {
        new_doc_page(&format!("mcp-recent-filler-{i}")).await;
    }
    let older = new_doc_page("mcp-recent-older").await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let newer = new_doc_page("mcp-recent-newer").await;

    let docs = call(
        &mcp,
        "get_recent_pages",
        json!({ "maxResults": 2000, "type": "doc" }),
    )
    .await;
    // An empty type takes every type, and maxResults 0 the default of 10
    let any_type = call(
        &mcp,
        "get_recent_pages",
        json!({ "maxResults": 0, "type": "" }),
    )
    .await;

    // Other tests create pages at the same time, so only the relative order is checked
    let ids = field_of_each(&docs, "pages", "id");
    assert!(ids.len() >= 12, "more than the default: {}", ids.len());
    let pos = |id: &str| ids.iter().position(|v| v == &json!(id));
    assert!(pos(&newer.id).unwrap() < pos(&older.id).unwrap(), "{ids:?}");
    assert!(field_of_each(&docs, "pages", "type")
        .iter()
        .all(|t| t == &json!("doc")));
    assert_eq!(any_type["pages"].as_array().map(Vec::len), Some(10));
}

// ページへのウィキリンクは、そのページのバックリンクに現れる。
#[tokio::test]
async fn a_wiki_link_to_a_page_shows_up_in_its_backlinks() {
    let mcp = connect().await;
    let target = new_doc_page("mcp-backlink-target").await;
    let source = new_doc_page("mcp-backlink-source").await;
    let link = fill(&source.id, &[&format!("see [[{}]]", target.title)])
        .await
        .remove(0);

    let backlinks = call(&mcp, "get_backlinks", json!({ "pageId": target.id })).await;

    assert_eq!(backlinks["entries"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        backlinks["entries"][0]["sourcePage"]["id"],
        json!(source.id)
    );
    assert_eq!(backlinks["entries"][0]["sourceItem"]["id"], json!(link.id));
}

// ---- Search ----

fn nanos() -> i64 {
    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
}

// ページ名と項目の本文を検索できる。
#[tokio::test]
async fn page_names_and_item_text_are_searched() {
    let mcp = connect().await;
    let page = new_doc_page("McpSearchable").await;
    let word = format!("yak{}", nanos());
    let item = fill(&page.id, &[&format!("a {word} grazing")])
        .await
        .remove(0);

    let by_name = call(
        &mcp,
        "search_page_names",
        // maxResults 0 takes the default
        json!({ "query": &page.title[..12], "maxResults": 0 }),
    )
    .await;
    let by_text = call(
        &mcp,
        "search_combined",
        json!({ "query": word, "maxResults": 5 }),
    )
    .await;
    let by_text_default = call(
        &mcp,
        "search_combined",
        json!({ "query": word, "maxResults": 0 }),
    )
    .await;

    assert!(
        field_of_each(&by_name, "results", "pageTitle").contains(&json!(page.title)),
        "{by_name}"
    );
    assert_eq!(
        field_of_each(&by_text, "results", "itemId"),
        vec![json!(item.id)]
    );
    assert_eq!(by_text["results"][0]["pageId"], json!(page.id));
    // The score moves as other tests index items, so only the hits are compared
    assert_eq!(
        field_of_each(&by_text_default, "results", "itemId"),
        vec![json!(item.id)]
    );
}

// 検索の結果は、maxResultsの数までである。0のときは、既定の数になる。
#[tokio::test]
async fn a_search_gives_at_most_max_results_and_the_default_number_for_zero() {
    let mcp = connect().await;
    let page = new_doc_page("McpSearchLimit").await;
    let word = format!("ibex{}", nanos());
    fill(
        &page.id,
        &[&format!("one {word}"), &format!("another {word}")],
    )
    .await;

    let one = call(
        &mcp,
        "search_combined",
        json!({ "query": word, "maxResults": 1 }),
    )
    .await;
    let by_default = call(
        &mcp,
        "search_combined",
        json!({ "query": word, "maxResults": 0 }),
    )
    .await;

    assert_eq!(one["results"].as_array().unwrap().len(), 1, "{one}");
    assert_eq!(
        by_default["results"].as_array().unwrap().len(),
        2,
        "{by_default}"
    );
}

// ---- Config ----

// 設定は読み書きできる。
#[tokio::test]
async fn the_config_is_read_and_updated() {
    let mcp = connect().await;

    let config = call(&mcp, "get_config", json!({})).await;
    let logical_outdent = config["logicalOutdent"].as_bool().unwrap();
    let updated = call(
        &mcp,
        "update_config",
        json!({
            "weekNumbering": "us",
            "markdownDir": config["markdownDir"],
            "logicalOutdent": !logical_outdent,
            "tagsDir": "labels/",
            "homeLink": "today",
        }),
    )
    .await;
    let restored = call(
        &mcp,
        "update_config",
        json!({
            "weekNumbering": config["weekNumbering"],
            "logicalOutdent": logical_outdent,
            "tagsDir": config["tagsDir"],
            "homeLink": config["homeLink"],
        }),
    )
    .await;

    assert_eq!(config["weekNumbering"], json!("iso"));
    assert_eq!(config["homeLink"], json!(""));
    assert!(config["markdownDir"]
        .as_str()
        .unwrap_or("")
        .ends_with("markdown"));
    assert_eq!(
        updated,
        json!({
            "weekNumbering": "us",
            "markdownDir": config["markdownDir"],
            "logicalOutdent": !logical_outdent,
            "tagsDir": "labels/",
            "homeLink": "today",
            "wikilinkHref": "/pages/{}",
            "hashtagHref": "/pages/labels/{}",
            "pagesBase": "/pages/",
            "wikilinkFragmentPattern": config["wikilinkFragmentPattern"],
        })
    );
    assert_eq!(restored, config);
}

// ---- Journals (1998 so that nothing else is near) ----

// ジャーナルは名前で作られ、ある日付より前の直近の日が見つかる。
#[tokio::test]
async fn a_journal_is_created_by_name_and_the_most_recent_day_before_a_date_is_found() {
    let mcp = connect().await;

    let first = call(
        &mcp,
        "get_or_create_journal",
        json!({ "fileName": "1998-06-15" }),
    )
    .await;
    let again = call(
        &mcp,
        "get_or_create_journal",
        json!({ "fileName": "1998-06-15" }),
    )
    .await;
    call(
        &mcp,
        "get_or_create_journal",
        json!({ "fileName": "1998-06-20" }),
    )
    .await;
    let before = call(
        &mcp,
        "get_most_recent_journal_before",
        json!({ "beforeDate": "1998-06-20" }),
    )
    .await;
    let nothing = call(
        &mcp,
        "get_most_recent_journal_before",
        json!({ "beforeDate": "1998-01-01" }),
    )
    .await;
    let not_a_journal = call_err(
        &mcp,
        "get_or_create_journal",
        json!({ "fileName": "notes" }),
    )
    .await;
    let not_a_date = call_err(
        &mcp,
        "get_most_recent_journal_before",
        json!({ "beforeDate": "June" }),
    )
    .await;

    assert_eq!(first["created"], json!(true));
    assert_eq!(first["page"]["type"], json!("journal"));
    assert_eq!(field_of_each(&first, "items", "body"), vec![json!("")]);
    assert_eq!(again["created"], json!(false));
    assert_eq!(again["page"]["id"], first["page"]["id"]);
    assert_eq!(before["found"], json!(true));
    assert_eq!(before["page"]["name"], json!("1998-06-15"));
    assert_eq!(nothing["found"], json!(false));
    assert!(nothing["page"].is_null(), "{nothing}");
    assert!(
        not_a_journal.contains("Not a journal filename"),
        "{not_a_journal}"
    );
    assert!(not_a_date.contains("Invalid date"), "{not_a_date}");
}
