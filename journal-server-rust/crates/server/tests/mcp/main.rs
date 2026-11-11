//! The MCP tools, called over Streamable HTTP at `/mcp` the way an MCP client does. Pages and
//! items are set up over the REST API where they are not what the test is about.
//!
//! These tests are a binary of their own, apart from the REST tests in `it`. One test here
//! changes the config, such as the week numbering. The REST tests read the same config, so the
//! two must not share a server.

#[path = "../common/mod.rs"]
mod common;

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

fn csv(ids: &[&str]) -> String {
    ids.join(",")
}

// ---- The server ----

// Javaサーバーと同じツールが一覧される。
#[tokio::test]
async fn the_tools_of_the_java_server_are_listed() {
    let mcp = connect().await;

    let tools = mcp.list_all_tools().await.expect("tools");
    let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    names.sort_unstable();

    assert_eq!(
        names,
        vec![
            "create_item",
            "cycle_task",
            "delete_item",
            "delete_page",
            "export_page_markdown",
            "export_subtree_markdown",
            "export_tree_markdown",
            "get_backlinks",
            "get_config",
            "get_item",
            "get_items_by_type",
            "get_most_recent_journal_before",
            "get_or_create_doc",
            "get_or_create_journal",
            "get_page_by_id",
            "get_recent_pages",
            "import_tree_markdown",
            "move_items",
            "remove_transclusion",
            "search_combined",
            "search_page_names",
            "set_transclusion",
            "update_config",
            "update_item",
            "update_page",
            "upsert_property",
        ]
    );
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
    assert_eq!(fields, json!({ "body": "b #mcp", "level": 2 }));
    let whole = call(&mcp, "get_item", json!({ "itemId": id, "fields": "" })).await;
    assert_eq!(whole["item"]["properties"]["tags"], json!(["mcp"]));

    // A level of -1 keeps the level; collapsed comes as a word
    let updated = call(
        &mcp,
        "update_item",
        json!({ "itemId": id, "body": "c", "collapsed": "true" }),
    )
    .await;
    assert_eq!(updated["item"]["body"], json!("c"));
    assert_eq!(updated["item"]["level"], json!(2));
    assert_eq!(updated["item"]["collapsed"], json!(true));

    let deleted = call(&mcp, "delete_item", json!({ "itemId": id })).await;
    assert_eq!(deleted, json!({ "deleted": true }));
    let error = call_err(&mcp, "get_item", json!({ "itemId": id, "fields": "" })).await;
    assert!(error.contains("Item not found"), "{error}");
}

// ページの最後の項目を消すと、同じ操作で空の項目が1つ作られる。
#[tokio::test]
async fn deleting_the_last_item_of_a_page_leaves_one_empty_item() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-delete-last").await;
    let only = fill(&page.id, &["only"]).await.remove(0);

    let deleted = call(&mcp, "delete_item", json!({ "itemId": only.id })).await;

    assert_eq!(deleted, json!({ "deleted": true }));
    assert_eq!(outline_of(&page.id).await, vec!["1:"]);
}

// 項目は、レベルをずらして、またはそのままで、ページの間を移動する。
#[tokio::test]
async fn items_move_between_pages_with_their_levels_shifted_or_kept() {
    let mcp = connect().await;
    let source = new_doc_page("mcp-move-from").await;
    let target = new_doc_page("mcp-move-to").await;
    let rows = fill(&source.id, &["a", "  b"]).await;
    let x = fill(&target.id, &["x"]).await.remove(0);
    let ab = csv(&[&rows[0].id, &rows[1].id]);

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
    assert!(outline_of(&source.id).await.is_empty());

    call(
        &mcp,
        "move_items",
        json!({ "itemIds": ab, "targetPageId": source.id, "position": "first", "levelDelta": -1 }),
    )
    .await;
    assert_eq!(outline_of(&source.id).await, vec!["1:a", "2:b"]);

    // A levelDelta of 0 keeps the levels
    call(
        &mcp,
        "move_items",
        json!({ "itemIds": x.id, "targetPageId": source.id, "position": "last", "levelDelta": 0 }),
    )
    .await;
    assert_eq!(outline_of(&source.id).await, vec!["1:a", "2:b", "1:x"]);
    assert!(outline_of(&target.id).await.is_empty());
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
        json!({ "itemIds": csv(&[&rows[0].id, &rows[1].id]) }),
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
    assert!(by_items["ast"].is_string());
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

// MCPで項目を作ると、そのページの変更が、MCPからの変更としてクライアントに知らされる。
#[tokio::test]
async fn the_clients_hear_of_an_item_created_over_mcp() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-tell-create").await;
    let mut events = listen().await;

    call(
        &mcp,
        "create_item",
        json!({ "pageId": page.id, "position": "last", "level": 1, "body": "a" }),
    )
    .await;

    assert_eq!(told_of(&mut events, &page.id).await["source"], "mcp");
}

// 項目を変えるツールのどれも、その項目のページの変更を知らせる。
#[tokio::test]
async fn the_clients_hear_of_each_change_of_an_item_over_mcp() {
    let mcp = connect().await;
    let page = new_doc_page("mcp-tell-item").await;
    let rows = fill(&page.id, &["a", "source"]).await;
    let (item, source) = (&rows[0].id, &rows[1].id);
    let changes = [
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

/// An item, its page, and another page whose only item transcludes the item.
async fn transcluded(mcp: &Mcp, name: &str) -> (ItemRow, PageRef, PageRef) {
    let source_page = new_doc_page(&format!("{name}-source")).await;
    let source = fill(&source_page.id, &["the source"]).await.remove(0);
    let page = new_doc_page(name).await;
    let host = fill(&page.id, &["placeholder"]).await.remove(0);
    call(
        mcp,
        "set_transclusion",
        json!({ "itemId": host.id, "sourceItemId": source.id }),
    )
    .await;
    (source, source_page, page)
}

// トランスクルードされている項目を変えると、その写しを見せているページの変更も知らされる。
#[tokio::test]
async fn the_clients_hear_of_the_pages_that_transclude_an_item_updated_over_mcp() {
    let mcp = connect().await;
    let (source, _, page) = transcluded(&mcp, "mcp-tell-transcluder").await;
    let mut events = listen().await;

    call(
        &mcp,
        "update_item",
        json!({ "itemId": source.id, "body": "changed" }),
    )
    .await;

    assert_eq!(told_of(&mut events, &page.id).await["source"], "mcp");
}

// 項目を消すと、その項目があったページと、その写しを見せていたページの変更が知らされる。
#[tokio::test]
async fn the_clients_hear_of_the_pages_that_showed_an_item_deleted_over_mcp() {
    let mcp = connect().await;
    let (source, source_page, page) = transcluded(&mcp, "mcp-tell-delete").await;
    let mut of_its_page = listen().await;
    let mut of_the_transcluder = listen().await;

    call(&mcp, "delete_item", json!({ "itemId": source.id })).await;

    let its_page = told_of(&mut of_its_page, &source_page.id).await;
    let transcluder = told_of(&mut of_the_transcluder, &page.id).await;
    assert_eq!(its_page["source"], "mcp");
    assert_eq!(transcluder["source"], "mcp");
}

// 項目をほかのページへ移すと、出たページと入ったページの両方の変更が知らされる。
#[tokio::test]
async fn the_clients_hear_of_both_pages_of_a_move_over_mcp() {
    let mcp = connect().await;
    let from = new_doc_page("mcp-tell-move-from").await;
    let to = new_doc_page("mcp-tell-move-to").await;
    let item = fill(&from.id, &["a"]).await.remove(0);
    let mut of_from = listen().await;
    let mut of_to = listen().await;

    call(
        &mcp,
        "move_items",
        json!({ "itemIds": item.id, "targetPageId": to.id, "position": "last" }),
    )
    .await;

    assert_eq!(told_of(&mut of_from, &from.id).await["source"], "mcp");
    assert_eq!(told_of(&mut of_to, &to.id).await["source"], "mcp");
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
    assert_eq!(nothing, json!({ "found": false }));
    assert!(
        not_a_journal.contains("Not a journal filename"),
        "{not_a_journal}"
    );
    assert!(not_a_date.contains("Invalid date"), "{not_a_date}");
}
