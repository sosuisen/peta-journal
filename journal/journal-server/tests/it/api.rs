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

/// Reads the event stream until an event has `needle` in it, and returns all that was read
async fn events_until(events: &mut reqwest::Response, needle: &str) -> String {
    let mut seen = String::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while !seen.contains(needle) {
        let chunk = tokio::time::timeout_at(deadline, events.chunk())
            .await
            .unwrap_or_else(|_| panic!("no event with {needle}"))
            .unwrap()
            .unwrap();
        seen.push_str(&String::from_utf8_lossy(&chunk));
    }
    seen
}

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

/// A source page with the given rows, and a page whose only item transcludes the first row
async fn transcluded(
    name: &str,
    source_rows: &[&str],
) -> (PageRef, Vec<ItemRow>, PageRef, ItemRow) {
    let source_page = new_doc_page(&format!("{name}-source")).await;
    let source = fill(&source_page.id, source_rows).await;
    let page = new_doc_page(name).await;
    let host = fill(&page.id, &["placeholder"]).await.remove(0);
    let (status, resp) = post(
        &format!("/api/items/{}/transclude", host.id),
        json!({ "sourceItemId": source[0].id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    (source_page, source, page, host)
}

/// How often each of the pages was reported as changed, from now until `act` is through
async fn times_told<F: std::future::Future<Output = ()>>(pages: &[&PageRef], act: F) -> Vec<usize> {
    let after = new_doc_page("items-told-after").await;
    let mut events = client().get(url("/api/events")).send().await.unwrap();
    act.await;
    // The events come in order: what is read up to this one holds all those of the act
    create_item(&after.id, "last", 1, "after the act").await;
    let seen = events_until(&mut events, &format!("\"pageId\":\"{}\"", after.id)).await;
    pages
        .iter()
        .map(|page| seen.matches(&format!("\"pageId\":\"{}\"", page.id)).count())
        .collect()
}

// ---- Items: batch ----

/// Applies operations as one batch, the way an undo sends what it replays
async fn batch(ops: Value) -> (u16, Value) {
    post("/api/items/batch", json!({ "ops": ops })).await
}

// 更新の操作は、本文、階層、折りたたみを書き、書いた項目を答える。
#[tokio::test]
async fn a_batch_update_writes_the_body_the_level_and_the_collapsed_state() {
    let page = new_doc_page("items-batch-update").await;
    let rows = fill(&page.id, &["top", "old"]).await;

    let (status, resp) = batch(json!([
        { "op": "update", "id": rows[1].id, "body": "new", "level": 2, "collapsed": true },
    ]))
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(field_of_each(&resp, "items", "body"), vec![json!("new")]);
    let stored = items(&page.id).await;
    assert_eq!(outline_of(&page.id).await, ["1:top", "2:new"]);
    assert!(stored[1].collapsed);
}

// バッチがページの項目をすべて消すと、同じ要求で空の項目が1つ作られ、答えの`created`に入る。
#[tokio::test]
async fn a_batch_that_empties_the_page_makes_one_empty_item_and_answers_it() {
    let page = new_doc_page("items-batch-empties").await;
    let rows = fill(&page.id, &["only"]).await;

    let (status, resp) = batch(json!([{ "op": "delete", "id": rows[0].id }])).await;
    assert_eq!(status, 200, "{resp}");

    let left = items(&page.id).await;
    assert_eq!(outline_of(&page.id).await, vs(&["1:"]));
    assert_eq!(
        field_of_each(&resp, "created", "id"),
        vec![json!(left[0].id)]
    );
    assert_eq!(resp["items"], json!([]));
}

// 項目が残るバッチは、空の項目を作らない。
#[tokio::test]
async fn a_batch_that_leaves_an_item_on_the_page_makes_no_item() {
    let page = new_doc_page("items-batch-leaves").await;
    let rows = fill(&page.id, &["a", "b"]).await;

    let (status, resp) = batch(json!([{ "op": "delete", "id": rows[0].id }])).await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(resp["created"], json!([]));
    assert_eq!(outline_of(&page.id).await, vs(&["1:b"]));
}

// 作成の操作は、名指しされたIDの項目を、指定の位置と階層に作る。削除した項目を、同じIDで作り直せる。
#[tokio::test]
async fn a_batch_create_makes_the_item_again_with_the_id_it_names() {
    let page = new_doc_page("items-batch-create").await;
    let rows = fill(&page.id, &["a", "  gone", "c"]).await;
    assert_eq!(delete(&format!("/api/items/{}", rows[1].id)).await, 204);

    let (status, resp) = batch(json!([{
        "op": "create", "id": rows[1].id, "pageId": page.id,
        "position": format!("after:{}", rows[0].id), "level": 2, "body": "back",
    }]))
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(field_of_each(&resp, "items", "id"), vec![json!(rows[1].id)]);
    let stored = items(&page.id).await;
    assert_eq!(outline_of(&page.id).await, ["1:a", "2:back", "1:c"]);
    assert_eq!(stored[1].id, rows[1].id);
}

// 作り直した項目は、もう「ここで消した項目」としては覚えられていない。Markdownの取り込みが、その項目を消されたものとして扱わないためである。
#[tokio::test]
async fn an_item_made_again_is_no_longer_remembered_as_deleted() {
    let page = new_doc_page("items-batch-undelete").await;
    let rows = fill(&page.id, &["a", "gone"]).await;
    assert_eq!(delete(&format!("/api/items/{}", rows[1].id)).await, 204);
    let deleted_dates = || {
        let conn = server().state.db.conn().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM deleted_items WHERE id = ?",
            [&rows[1].id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
    };
    assert_eq!(deleted_dates(), 1);

    let (status, resp) = batch(json!([{
        "op": "create", "id": rows[1].id, "pageId": page.id,
        "position": format!("after:{}", rows[0].id), "level": 1, "body": "back",
    }]))
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(deleted_dates(), 0);
}

// 削除の操作は、まとめて1つのグループとして消される。親が子より先に並んでいてもよい。消した項目は答えに入らない。
#[tokio::test]
async fn the_deletes_of_a_batch_go_as_one_group() {
    let page = new_doc_page("items-batch-delete").await;
    let rows = fill(&page.id, &["keep", "parent", "  child"]).await;

    let (status, resp) = batch(json!([
        { "op": "delete", "id": rows[1].id },
        { "op": "delete", "id": rows[2].id },
    ]))
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(resp["items"], json!([]));
    assert_eq!(outline_of(&page.id).await, ["1:keep"]);
}

// 更新の操作は、タスクの状態も、時刻と予定日ごと書ける。タスクの切り替えの undo が、本文と同じ操作で戻すためである。
#[tokio::test]
async fn a_batch_update_writes_the_task_state_with_its_times() {
    let page = new_doc_page("items-batch-task").await;
    let item = fill(&page.id, &["buy milk"]).await.remove(0);

    let (status, resp) = batch(json!([{
        "op": "update", "id": item.id, "body": "buy oat milk",
        "task": {
            "task": true,
            "begin": "2026-01-02T03:04:05",
            "end": "2026-01-02T04:05:06",
            "plan": "2026-01-02",
        },
    }]))
    .await;
    assert_eq!(status, 200, "{resp}");

    let answered = ItemRow::of(&resp["items"][0]);
    let stored = items(&page.id).await.remove(0);
    for row in [answered, stored] {
        assert_eq!(row.body, "buy oat milk");
        assert!(row.types.contains(&"task".to_string()));
        assert_eq!(row.properties["begin"], json!("2026-01-02T03:04:05"));
        assert_eq!(row.properties["end"], json!("2026-01-02T04:05:06"));
        assert_eq!(row.properties["plan"], json!("2026-01-02"));
    }
}

// 更新の操作は、指定されなかった時刻と予定日を消し、項目を通常の項目に戻せる。
#[tokio::test]
async fn a_batch_update_removes_what_the_task_state_does_not_give() {
    let page = new_doc_page("items-batch-task-unset").await;
    let item = fill(&page.id, &["buy milk"]).await.remove(0);
    let full = json!({ "task": true, "begin": "2026-01-02T03:04:05Z", "end": "2026-01-02T04:05:06Z", "plan": "2026-01-02" });
    let (status, resp) = batch(json!([{"op": "update", "id": item.id, "task": full}])).await;
    assert_eq!(status, 200, "{resp}");

    let (status, resp) =
        batch(json!([{"op": "update", "id": item.id, "task": {"task": false}}])).await;
    assert_eq!(status, 200, "{resp}");

    let row = items(&page.id).await.remove(0);
    assert!(!row.types.contains(&"task".to_string()));
    for key in ["begin", "end", "plan"] {
        assert!(!row.properties.contains_key(key), "{key} is still there");
    }
}

// 通常の項目に時刻を付ける更新は断られ、同じバッチのほかの操作も書かれない。cycle-taskも、そのような状態を作らない。
#[tokio::test]
async fn a_batch_that_gives_times_to_a_normal_item_is_refused_and_writes_nothing() {
    let page = new_doc_page("items-batch-task-refused").await;
    let rows = fill(&page.id, &["first", "second"]).await;

    for times in [json!({ "begin": "b" }), json!({ "end": "e" })] {
        let (status, _) = batch(json!([
            {"op": "update", "id": rows[0].id, "body": "changed"},
            {"op": "update", "id": rows[1].id, "task": times},
        ]))
        .await;
        assert_eq!(status, 400);
    }

    let stored = items(&page.id).await;
    assert_eq!(stored[0].body, "first");
    assert!(stored[1].properties.is_empty());
}

// 操作は並んだ順に行われる。先に作った項目を、後の操作が位置の基準にも更新の相手にもできる。
#[tokio::test]
async fn the_operations_of_a_batch_are_applied_in_their_order() {
    let page = new_doc_page("items-batch-order").await;
    let rows = fill(&page.id, &["a", "d"]).await;
    let ids = hmtid::Generator::new('_', false);
    let (b, c) = (ids.generate().to_string(), ids.generate().to_string());

    let (status, resp) = batch(json!([
        {"op": "create", "id": b, "pageId": page.id,
         "position": format!("after:{}", rows[0].id), "level": 1, "body": "b"},
        {"op": "create", "id": c, "pageId": page.id,
         "position": format!("after:{b}"), "level": 2, "body": "c"},
        {"op": "update", "id": b, "body": "B"},
    ]))
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(outline_of(&page.id).await, ["1:a", "1:B", "2:c", "1:d"]);
}

// 同じ項目を2回書くバッチは、その項目を最後の姿で1回だけ答える。
#[tokio::test]
async fn an_item_written_twice_in_a_batch_is_answered_once_as_it_is_at_the_end() {
    let page = new_doc_page("items-batch-twice").await;
    let item = fill(&page.id, &["old"]).await.remove(0);

    let (status, resp) = batch(json!([
        {"op": "update", "id": item.id, "body": "middle"},
        {"op": "update", "id": item.id, "body": "new"},
    ]))
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(field_of_each(&resp, "items", "body"), vec![json!("new")]);
}

// ない項目を名指しする操作があるバッチは断られ、ほかの操作も書かれない。
#[tokio::test]
async fn a_batch_that_names_an_item_that_is_gone_is_refused_and_writes_nothing() {
    let page = new_doc_page("items-batch-missing").await;
    let rows = fill(&page.id, &["stays", "gone"]).await;
    assert_eq!(delete(&format!("/api/items/{}", rows[1].id)).await, 204);

    let (status, _) = batch(json!([
        {"op": "update", "id": rows[0].id, "body": "changed"},
        {"op": "update", "id": rows[1].id, "body": "back"},
    ]))
    .await;
    assert_eq!(status, 400);
    let (status, _) = batch(json!([
        {"op": "update", "id": rows[0].id, "body": "changed"},
        {"op": "delete", "id": rows[1].id},
    ]))
    .await;
    assert_eq!(status, 404);

    assert_eq!(outline_of(&page.id).await, ["1:stays"]);
}

// 置けない階層への作成や更新があるバッチは断られ、ほかの操作も書かれない。
#[tokio::test]
async fn a_batch_with_a_level_that_does_not_fit_is_refused_and_writes_nothing() {
    let page = new_doc_page("items-batch-level").await;
    let rows = fill(&page.id, &["a", "b"]).await;
    let id = hmtid::Generator::new('_', false).generate().to_string();

    let (status, _) = batch(json!([
        {"op": "update", "id": rows[0].id, "body": "changed"},
        {"op": "create", "id": id, "pageId": page.id,
         "position": format!("after:{}", rows[0].id), "level": 3, "body": "too deep"},
    ]))
    .await;
    assert_eq!(status, 400);
    let (status, _) = batch(json!([
        {"op": "update", "id": rows[0].id, "body": "changed"},
        {"op": "update", "id": rows[1].id, "level": 0},
    ]))
    .await;
    assert_eq!(status, 400);

    assert_eq!(outline_of(&page.id).await, ["1:a", "1:b"]);
}

// 子孫を残す削除があるバッチは断られ、ほかの操作も書かれない。
#[tokio::test]
async fn a_batch_whose_deletes_leave_a_descendant_behind_is_refused_and_writes_nothing() {
    let page = new_doc_page("items-batch-orphan").await;
    let rows = fill(&page.id, &["parent", "  child", "other"]).await;

    let (status, _) = batch(json!([
        {"op": "update", "id": rows[2].id, "body": "changed"},
        {"op": "delete", "id": rows[0].id},
    ]))
    .await;
    assert_eq!(status, 400);

    assert_eq!(
        outline_of(&page.id).await,
        ["1:parent", "2:child", "1:other"]
    );
}

// バッチがいくつ項目を書いても、ページの変更は、ページごとに1回だけクライアントに知らされる。
#[tokio::test]
async fn the_clients_hear_of_each_page_of_a_batch_once() {
    let page = new_doc_page("items-batch-event").await;
    let rows = fill(&page.id, &["a", "b"]).await;
    let second_page = new_doc_page("items-batch-event-second").await;
    let second = fill(&second_page.id, &["c"]).await.remove(0);
    let other_page = new_doc_page("items-batch-event-after").await;
    let mut events = client().get(url("/api/events")).send().await.unwrap();

    let (status, resp) = batch(json!([
        {"op": "update", "id": rows[0].id, "body": "A"},
        {"op": "update", "id": second.id, "body": "C"},
        {"op": "delete", "id": rows[1].id},
    ]))
    .await;
    assert_eq!(status, 200, "{resp}");
    // The events come in order: what is read up to this one holds all those of the batch
    create_item(&other_page.id, "last", 1, "after the batch").await;

    let other = format!("\"pageId\":\"{}\"", other_page.id);
    let seen = events_until(&mut events, &other).await;
    for page_id in [&page.id, &second_page.id] {
        let told = format!("\"pageId\":\"{page_id}\"");
        assert_eq!(seen.matches(&told).count(), 1, "{seen}");
    }
}

// トランスクルードされている項目を更新すると、本文が変わらなくても、その写しを見せているページの変更が知らされる。
#[tokio::test]
async fn the_clients_hear_of_the_pages_that_transclude_an_item_a_batch_updates() {
    let (_, source, page, _) = transcluded("items-batch-transcluder", &["buy milk"]).await;
    let mut events = client().get(url("/api/events")).send().await.unwrap();

    let (status, resp) = batch(json!([
        {"op": "update", "id": source[0].id, "task": {"task": true}},
    ]))
    .await;
    assert_eq!(status, 200, "{resp}");

    event_with(&mut events, &format!("\"pageId\":\"{}\"", page.id)).await;
}

// 移動の操作は、項目のまとまりを、指定の位置へ動かし、動かした項目を答える。移動の undo が位置を戻すためである。
#[tokio::test]
async fn a_batch_move_puts_a_group_where_its_position_says() {
    let page = new_doc_page("items-batch-move").await;
    let rows = fill(&page.id, &["a", "  a1", "b", "  b1"]).await;

    let (status, resp) = batch(json!([{
        "op": "move", "ids": [rows[2].id, rows[3].id], "pageId": page.id, "position": "first",
    }]))
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(
        field_of_each(&resp, "items", "id"),
        vec![json!(rows[2].id), json!(rows[3].id)]
    );
    assert_eq!(outline_of(&page.id).await, ["1:b", "2:b1", "1:a", "2:a1"]);

    let (status, resp) = batch(json!([{
        "op": "move", "ids": [rows[2].id, rows[3].id], "pageId": page.id,
        "position": format!("after:{}", rows[1].id),
    }]))
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(outline_of(&page.id).await, ["1:a", "2:a1", "1:b", "2:b1"]);
}

// 階層の更新と移動を1つのバッチで行える。論理アウトデントの undo が、階層と位置を一緒に戻すためである。
#[tokio::test]
async fn a_batch_updates_a_level_and_then_moves_the_item() {
    let page = new_doc_page("items-batch-level-move").await;
    let rows = fill(&page.id, &["parent", "  first", "  last", "moved"]).await;

    let (status, resp) = batch(json!([
        {"op": "update", "id": rows[3].id, "level": 2},
        {"op": "move", "ids": [rows[3].id], "pageId": page.id,
         "position": format!("after:{}", rows[1].id)},
    ]))
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(
        outline_of(&page.id).await,
        ["1:parent", "2:first", "2:moved", "2:last"]
    );
    assert_eq!(field_of_each(&resp, "items", "id"), vec![json!(rows[3].id)]);
}

// 置けない位置への移動があるバッチは断られ、ほかの操作も書かれない。
#[tokio::test]
async fn a_batch_with_a_move_that_does_not_fit_is_refused_and_writes_nothing() {
    let page = new_doc_page("items-batch-move-refused").await;
    let rows = fill(&page.id, &["a", "  a1", "b"]).await;

    // A child cannot be the first item of a page
    let (status, _) = batch(json!([
        {"op": "update", "id": rows[2].id, "body": "changed"},
        {"op": "move", "ids": [rows[1].id], "pageId": page.id, "position": "first"},
    ]))
    .await;
    assert_eq!(status, 400);
    // An item cannot go after itself
    let (status, _) = batch(json!([
        {"op": "update", "id": rows[2].id, "body": "changed"},
        {"op": "move", "ids": [rows[2].id], "pageId": page.id,
         "position": format!("after:{}", rows[2].id)},
    ]))
    .await;
    assert_eq!(status, 400);

    assert_eq!(outline_of(&page.id).await, ["1:a", "2:a1", "1:b"]);
}

// ---- Items: unclosed blocks (ADR 050) ----

/// What the server answers when a body ends inside a block that is not closed.
const UNCLOSED: &str =
    "Markdownのブロック要素は、開始記号と閉じる記号が揃わない限り、保存できません。";

// 閉じていないブロックを持つ本文での更新は断られ、本文はそのまま残る。断るときの文は決まっている。
#[tokio::test]
async fn an_update_to_a_body_with_an_unclosed_block_is_refused() {
    let page = new_doc_page("unclosed-update").await;
    let rows = fill(&page.id, &["a"]).await;

    let (status, resp) = put(
        &format!("/api/items/{}", rows[0].id),
        json!({ "body": "```\nabc" }),
    )
    .await;

    assert_eq!(status, 400, "{resp}");
    assert_eq!(resp["error"], json!(UNCLOSED));
    assert_eq!(outline_of(&page.id).await, vs(&["1:a"]));
}

// 閉じていないブロックを持つ本文での作成は断られ、項目はできない。
#[tokio::test]
async fn an_item_with_an_unclosed_block_is_not_created() {
    let page = new_doc_page("unclosed-create").await;
    fill(&page.id, &["a"]).await;

    let (status, resp) = post(
        "/api/items",
        json!({ "pageId": page.id, "position": "last", "level": 1, "body": ">>>\nabc" }),
    )
    .await;

    assert_eq!(status, 400, "{resp}");
    assert_eq!(outline_of(&page.id).await, vs(&["1:a"]));
}

// バッチの本文の1つでも閉じていなければ、バッチ全体が断られる。閉じた本文の更新も書かれない。
#[tokio::test]
async fn a_batch_with_one_unclosed_body_is_refused_as_a_whole() {
    let page = new_doc_page("unclosed-batch").await;
    let rows = fill(&page.id, &["a", "b"]).await;

    let (status, resp) = batch(json!([
        { "op": "update", "id": rows[0].id, "body": "closed" },
        { "op": "update", "id": rows[1].id, "body": "<!--\nabc" },
    ]))
    .await;

    assert_eq!(status, 400, "{resp}");
    assert_eq!(outline_of(&page.id).await, vs(&["1:a", "1:b"]));
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
    assert_eq!(status, 200, "{resp}");

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

// グループ削除でページの項目がすべて消えると、同じ要求で空の項目が1つ作られ、答えに入る。
#[tokio::test]
async fn a_group_delete_that_empties_the_page_makes_one_empty_item_and_answers_it() {
    let page = new_doc_page("items-group-delete-all").await;
    let rows = fill(&page.id, &["a", "  a1"]).await;

    let (status, resp) = post(
        "/api/items/delete",
        json!({ "itemIds": [rows[0].id, rows[1].id] }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let left = items(&page.id).await;
    assert_eq!(outline_of(&page.id).await, vs(&["1:"]));
    assert_eq!(
        field_of_each(&resp, "created", "id"),
        vec![json!(left[0].id)]
    );
    assert_eq!(resp["created"][0]["pageId"], json!(page.id));
}

// ページに項目が残るグループ削除は、項目を作らない。
#[tokio::test]
async fn a_group_delete_that_leaves_an_item_on_the_page_makes_no_item() {
    let page = new_doc_page("items-group-delete-some").await;
    let rows = fill(&page.id, &["a", "b"]).await;

    let (status, resp) = post("/api/items/delete", json!({ "itemIds": [rows[0].id] })).await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(resp["created"], json!([]));
    assert_eq!(outline_of(&page.id).await, vs(&["1:b"]));
}

// トランスクルージョンをグループ削除すると、その項目だけが消え、元の項目とその子は元のページに残る。
#[tokio::test]
async fn a_group_delete_of_a_transclusion_deletes_only_that_item_and_keeps_its_source() {
    let source_page = new_doc_page("items-group-delete-source").await;
    let source = fill(&source_page.id, &["root", "  child"]).await;
    let page = new_doc_page("items-group-delete-transclusion").await;
    let rows = fill(&page.id, &["host", "  own child", "after"]).await;
    let (status, resp) = post(
        &format!("/api/items/{}/transclude", rows[0].id),
        json!({ "sourceItemId": source[0].id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (status, resp) = post(
        "/api/items/delete",
        json!({ "itemIds": [rows[0].id, rows[1].id] }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(outline_of(&page.id).await, vs(&["1:after"]));
    assert_eq!(
        outline_of(&source_page.id).await,
        vs(&["1:root", "2:child"])
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
    assert_eq!(outline_of(&page.id).await, ["1:parent"]);
}

// 単体の削除でページの最後の項目が消えると、同じ要求で空の項目が1つ作られる。
#[tokio::test]
async fn a_single_delete_of_the_last_item_leaves_one_empty_item_on_the_page() {
    let page = new_doc_page("items-single-delete-last").await;
    let rows = fill(&page.id, &["only"]).await;

    assert_eq!(delete(&format!("/api/items/{}", rows[0].id)).await, 204);

    let left = items(&page.id).await;
    assert_eq!(outline_of(&page.id).await, vs(&["1:"]));
    assert_ne!(left[0].id, rows[0].id);
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

// ページの取得は、fieldsで頼まれたフィールドだけを各項目に持つ。トランスクルードの木の項目も同じである。ページの情報はそのまま返る。
#[tokio::test]
async fn get_page_returns_its_items_with_only_the_fields_asked_for() {
    let source_page = new_doc_page("pages-get-fields-source").await;
    let source = fill(&source_page.id, &["source"]).await.remove(0);
    let page = new_doc_page("pages-get-fields").await;
    let rows = fill(&page.id, &["one", "  host"]).await;
    let (status, resp) = post(
        &format!("/api/items/{}/transclude", rows[1].id),
        json!({ "sourceItemId": source.id }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (status, resp) = get(&format!("/api/pages/{}?fields=id,level", page.id)).await;
    assert_eq!(status, 200);

    assert_eq!(resp["page"]["name"], json!(page.title));
    assert_eq!(
        resp["items"],
        json!([
            { "id": rows[0].id, "level": 1 },
            { "id": rows[1].id, "level": 2 },
        ])
    );
    assert_eq!(
        resp["transcludedTrees"],
        json!({ source.id.as_str(): [{ "id": source.id, "level": 1 }] })
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

// NFDのタイトルとフォルダで開いても、ページはNFCの名前になり、NFCで開いたときと同じページである。
#[tokio::test]
async fn a_doc_opened_with_an_nfd_title_and_folder_has_nfc_names_and_is_the_same_page() {
    let n = nanos();

    let (status, first) = post(
        "/api/pages/doc",
        json!({ "title": format!("\u{30ab}\u{3099}-{n}"), "dirPath": format!("\u{30c6}\u{3099}-{n}/") }),
    )
    .await;
    assert_eq!(status, 200, "{first}");
    let (_, again) = post(
        "/api/pages/doc",
        json!({ "title": format!("\u{30ac}-{n}"), "dirPath": format!("\u{30c7}-{n}/") }),
    )
    .await;

    assert_eq!(first["page"]["name"], json!(format!("\u{30ac}-{n}")));
    assert_eq!(first["page"]["dirPath"], json!(format!("\u{30c7}-{n}/")));
    assert_eq!(again["created"], json!(false));
    assert_eq!(first["page"]["id"], again["page"]["id"]);
}

// NFDのタイトルとフォルダへ改名すると、ページはNFCの名前になる。
#[tokio::test]
async fn renaming_a_page_to_an_nfd_title_and_folder_gives_nfc_names() {
    let page = new_doc_page("pages-rename-nfd").await;
    let n = nanos();

    let (status, resp) = put(
        &format!("/api/pages/{}", page.id),
        json!({ "title": format!("\u{30ab}\u{3099}-{n}"), "dirPath": format!("\u{30c6}\u{3099}-{n}/") }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(resp["page"]["name"], json!(format!("\u{30ac}-{n}")));
    assert_eq!(resp["page"]["dirPath"], json!(format!("\u{30c7}-{n}/")));
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

/// Demotes an index page the way the app does
async fn demote(page_id: &str, overwrite: bool) -> (u16, Value) {
    post(
        &format!("/api/pages/{page_id}/demote"),
        json!({ "overwrite": overwrite }),
    )
    .await
}

// 降格は、1つの要求で、アトラスページを消し、indexページをディレクトリの名前のページに変える。
#[tokio::test]
async fn demoting_an_index_page_deletes_its_atlas_and_renames_it_in_one_request() {
    let root = new_doc_page("pages-demote").await;
    let atlas_id = atlas_of(&format!("{}/", root.title)).await;
    let (_, before) = get(&format!("/api/pages/{}", root.id)).await;
    assert_eq!(before["page"]["name"], json!("index"));

    let (status, resp) = demote(&root.id, false).await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(resp["page"]["id"], json!(root.id));
    assert_eq!(resp["page"]["name"], json!(root.title));
    assert_eq!(resp["page"]["dirPath"], json!(""));
    let (status, _) = get(&format!("/api/pages/{atlas_id}")).await;
    assert_eq!(status, 404);
}

// 入れ子のディレクトリのindexページは、親のディレクトリの中の、ディレクトリの名前のページになる。
#[tokio::test]
async fn demoting_a_nested_index_page_puts_it_in_the_directory_above() {
    let root = new_doc_page("pages-demote-nested").await;
    let dir = format!("{}/", root.title);
    let (status, resp) = post("/api/pages/doc", json!({ "title": "sub", "dirPath": dir })).await;
    assert_eq!(status, 200, "{resp}");
    let sub_id = resp["page"]["id"].as_str().unwrap().to_string();
    atlas_of(&format!("{dir}sub/")).await;

    let (status, resp) = demote(&sub_id, false).await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(resp["page"]["name"], json!("sub"));
    assert_eq!(resp["page"]["dirPath"], json!(dir));
}

// アトラス以外の子ページがあるindexページの降格は断られ、アトラスも残る。
#[tokio::test]
async fn demoting_an_index_page_with_a_child_page_is_refused_and_keeps_the_atlas() {
    let root = new_doc_page("pages-demote-child").await;
    let dir = format!("{}/", root.title);
    let atlas_id = atlas_of(&dir).await;
    let (status, resp) = post(
        "/api/pages/doc",
        json!({ "title": "child", "dirPath": dir }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (status, resp) = demote(&root.id, false).await;
    assert_eq!(status, 400, "{resp}");

    let (status, _) = get(&format!("/api/pages/{atlas_id}")).await;
    assert_eq!(status, 200);
    let (_, page) = get(&format!("/api/pages/{}", root.id)).await;
    assert_eq!(page["page"]["name"], json!("index"));
}

// 降格先の名前を別のページが持っていると、降格は断られ、アトラスも残る。上書きを指定すると、そのページを消して降格する。
#[tokio::test]
async fn demoting_to_the_name_of_another_page_is_refused_unless_it_is_overwritten() {
    let root = new_doc_page("pages-demote-taken").await;
    let atlas_id = atlas_of(&format!("{}/", root.title)).await;
    let other = new_doc_page("pages-demote-taker").await;
    let (status, resp) = put(
        &format!("/api/pages/{}", other.id),
        json!({ "title": root.title }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    let (status, resp) = demote(&root.id, false).await;
    assert_eq!(status, 409, "{resp}");
    assert_eq!(get(&format!("/api/pages/{atlas_id}")).await.0, 200);
    assert_eq!(get(&format!("/api/pages/{}", other.id)).await.0, 200);

    let (status, resp) = demote(&root.id, true).await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["name"], json!(root.title));
    assert_eq!(get(&format!("/api/pages/{atlas_id}")).await.0, 404);
    assert_eq!(get(&format!("/api/pages/{}", other.id)).await.0, 404);
}

// indexページでないページは、ルートにあってもディレクトリの中にあっても降格できない。ルートのindexページも降格できない。
#[tokio::test]
async fn a_page_that_is_not_an_index_page_of_a_directory_is_not_demoted() {
    let page = new_doc_page("pages-demote-plain").await;

    let (status, resp) = demote(&page.id, false).await;
    assert_eq!(status, 400, "{resp}");

    let (_, root) = post("/api/pages/doc", json!({ "title": "index", "dirPath": "" })).await;
    let (status, resp) = demote(root["page"]["id"].as_str().unwrap(), false).await;
    assert_eq!(status, 400, "{resp}");

    // A page in a directory that is not its index page keeps its name
    let dir = format!("{}/", page.title);
    let (_, child) = post(
        "/api/pages/doc",
        json!({ "title": "child", "dirPath": dir }),
    )
    .await;
    let child_id = child["page"]["id"].as_str().unwrap();
    let (status, resp) = demote(child_id, false).await;
    assert_eq!(status, 400, "{resp}");
    let (_, kept) = get(&format!("/api/pages/{child_id}")).await;
    assert_eq!(kept["page"]["name"], json!("child"));
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

// ---- Journals: moving items to a day ----

/// The id of the page of a journal answer.
fn page_id_of(journal: &Value) -> String {
    journal["page"]["id"].as_str().unwrap().to_string()
}

/// Moves items to the daily journal of `date`, the way a move key at the edge of a day
/// does: with the body the editor of the first item has now, when it is not saved yet.
async fn move_to_journal(
    date: &str,
    item_ids: &[&str],
    position: &str,
    body: Option<&str>,
) -> (u16, Value) {
    let mut req = json!({ "itemIds": item_ids, "position": position });
    if let Some(body) = body {
        req["body"] = json!(body);
    }
    post(&format!("/api/journals/{date}/move"), req).await
}

// 項目は子ごと、その日のジャーナルの末尾へ動く。要求の本文は、先頭の項目に保存される。日をまたぐ移動が1つの要求で送るものである。
#[tokio::test]
async fn items_move_to_the_end_of_a_days_journal_with_the_body_the_request_carries() {
    let from = page_id_of(&journal("1999-07-01").await);
    let rows = fill(&from, &["go", "  go-child", "stay"]).await;
    let to = page_id_of(&journal("1999-07-02").await);
    fill(&to, &["there"]).await;

    let (status, resp) = move_to_journal(
        "1999-07-02",
        &[&rows[0].id, &rows[1].id],
        "last",
        Some("go typed"),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(resp["created"], json!(false));
    assert_eq!(resp["page"]["id"], json!(to));
    assert_eq!(
        field_of_each(&resp, "items", "body"),
        vec![json!("go typed"), json!("go-child")]
    );
    assert_eq!(
        outline_of(&to).await,
        vs(&["1:there", "1:go typed", "2:go-child"])
    );
    assert_eq!(outline_of(&from).await, vs(&["1:stay"]));
}

// その日のジャーナルがなければ、移動が作る。作られたページは、空の項目を持たずに動かした項目だけを持ち、`created`で答え、名前で検索できる。
#[tokio::test]
async fn a_move_makes_the_journal_of_the_day_when_there_is_none() {
    let from = page_id_of(&journal("1999-07-03").await);
    let rows = fill(&from, &["go", "stay"]).await;

    let (status, resp) = move_to_journal("1999-07-04", &[&rows[0].id], "last", None).await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(resp["created"], json!(true));
    assert_eq!(resp["page"]["name"], json!("1999-07-04"));
    let to = page_id_of(&resp);
    assert_eq!(outline_of(&to).await, vs(&["1:go"]));
    let again = journal("1999-07-04").await;
    assert_eq!(again["created"], json!(false));
    assert_eq!(page_id_of(&again), to);
    let found = search("/api/search/pages", "1999-07-04").await;
    assert!(
        field_of_each(&found, "results", "pageTitle").contains(&json!("1999-07-04")),
        "{found}"
    );
}

// 移動先の日が空の1項目だけを持っていれば、その項目は消え、動かした項目だけが残る。
#[tokio::test]
async fn the_single_empty_item_of_the_day_goes_when_items_move_in() {
    let from = page_id_of(&journal("1999-07-05").await);
    let rows = fill(&from, &["go", "stay"]).await;
    let to = page_id_of(&journal("1999-07-06").await);

    let (status, resp) = move_to_journal("1999-07-06", &[&rows[0].id], "first", None).await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(outline_of(&to).await, vs(&["1:go"]));
}

// 動かした項目が最後の項目だった日は、空の項目を1つ残す。ページは項目を1つは持つ。
#[tokio::test]
async fn the_day_the_last_item_leaves_keeps_one_empty_item() {
    let from = page_id_of(&journal("1999-07-07").await);
    let rows = fill(&from, &["go", "  go-child"]).await;
    let to = page_id_of(&journal("1999-07-08").await);
    fill(&to, &["there"]).await;

    let (status, resp) =
        move_to_journal("1999-07-08", &[&rows[0].id, &rows[1].id], "first", None).await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(outline_of(&from).await, vs(&["1:"]));
    assert_eq!(
        outline_of(&to).await,
        vs(&["1:go", "2:go-child", "1:there"])
    );
}

// 断られた移動は、何も書かない。本文も保存されず、日のジャーナルも作られない。
#[tokio::test]
async fn a_refused_move_to_a_day_writes_nothing() {
    let from = page_id_of(&journal("1999-07-09").await);
    let rows = fill(&from, &["go"]).await;

    let not_there = "after:00000000000000000000000000";
    let (status, resp) =
        move_to_journal("1999-07-10", &[&rows[0].id], not_there, Some("typed")).await;

    assert_eq!(status, 400, "{resp}");
    assert_eq!(outline_of(&from).await, vs(&["1:go"]));
    assert_eq!(journal("1999-07-10").await["created"], json!(true));
}

// 日でないジャーナル（週、月、年）への移動は断られる。
#[tokio::test]
async fn items_do_not_move_to_a_week_a_month_or_a_year() {
    let from = page_id_of(&journal("1999-07-11").await);
    let rows = fill(&from, &["go"]).await;

    let (status, resp) = move_to_journal("1999-W28", &[&rows[0].id], "last", None).await;

    assert_eq!(status, 400, "{resp}");
    assert_eq!(outline_of(&from).await, vs(&["1:go"]));
}

// 日への移動では、移動先の日と、項目が出ていった日の変更が、クライアントに1回ずつ知らされる。
#[tokio::test]
async fn the_clients_hear_once_of_each_day_a_move_changes() {
    let from = journal("1999-07-12").await;
    let rows = fill(&page_id_of(&from), &["go", "stay"]).await;
    let to = journal("1999-07-13").await;
    let as_ref = |j: &Value| PageRef {
        id: page_id_of(j),
        title: j["page"]["name"].as_str().unwrap().to_string(),
    };

    let told = times_told(&[&as_ref(&from), &as_ref(&to)], async {
        let (status, resp) = move_to_journal("1999-07-13", &[&rows[0].id], "last", None).await;
        assert_eq!(status, 200, "{resp}");
    })
    .await;

    assert_eq!(told, [1, 1]);
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
    // The apps parse the copy themselves: the export carries no AST
    assert!(exported.get("ast").is_none(), "{exported}");
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

// 項目とその子孫のエクスポートは、その項目たちのエクスポートと同じになる。兄弟は入らない。
#[tokio::test]
async fn a_subtree_exports_like_its_items() {
    let page = new_doc_page("markdown-subtree").await;
    let rows = fill(&page.id, &["one", "  two", "sibling"]).await;

    let by_subtree: Value = client()
        .post(url("/api/items/export-subtree"))
        .query(&[("itemId", rows[0].id.as_str())])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let (_, by_items) = post(
        "/api/items/export",
        json!({ "itemIds": [rows[0].id, rows[1].id] }),
    )
    .await;

    assert_eq!(by_items["treeMarkdown"], by_subtree["treeMarkdown"]);
    assert_eq!(by_subtree["treeMarkdown"], json!("- one\n  - two"));
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
