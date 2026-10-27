//! The API tests of the Java server, run against the Rust server.

mod common;

use common::*;
use serde_json::{json, Value};

fn vs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// ---- Items: creating ----

#[tokio::test]
async fn items_go_first_last_or_after_another_item() {
    let page = new_doc_page("items-create").await;
    let rows = fill(&page.id, &["a", "b"]).await;

    create_item(&page.id, "first", 1, "x").await;
    create_item(&page.id, "last", 1, "y").await;
    create_item(&page.id, &format!("after:{}", rows[0].id), 2, "z").await;

    assert_eq!(outline_of(&page.id).await, vs(&["1:x", "1:a", "2:z", "1:b", "1:y"]));
}

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

#[tokio::test]
async fn a_hashtag_in_the_body_comes_back_as_a_tag_property() {
    let page = new_doc_page("items-hashtag").await;

    let created = create_item(&page.id, "last", 1, "remember #urgent").await;
    let updated = update_item(&created.id, json!({ "body": "remember #urgent #later" })).await;

    assert_eq!(created.properties["tags"], json!(["urgent"]));
    assert_eq!(updated.properties["tags"], json!(["urgent", "later"]));
}

// ---- Items: updating ----

#[tokio::test]
async fn the_body_is_parsed_into_an_ast_and_types() {
    let page = new_doc_page("items-parse").await;
    let plain = fill(&page.id, &["plain"]).await.remove(0);

    let heading = update_item(&plain.id, json!({ "body": "## Title" })).await;

    assert!(heading.types.contains(&"heading".to_string()), "types: {:?}", heading.types);
    let (_, item) = get(&format!("/api/items/{}", plain.id)).await;
    assert!(item["item"]["ast"].as_str().unwrap().contains("\"heading\""));
}

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

    assert_eq!(items(&page.id).await[0].properties["plan"], json!("2026-03-01"));
}

#[tokio::test]
async fn a_deleted_item_is_gone_from_its_page() {
    let page = new_doc_page("items-delete").await;
    let rows = fill(&page.id, &["keep", "drop"]).await;

    assert_eq!(delete(&format!("/api/items/{}", rows[1].id)).await, 204);

    assert_eq!(outline_of(&page.id).await, vs(&["1:keep"]));
}

// ---- Items: levels ----

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

    assert_eq!(outline_of(&page.id).await, vs(&["1:a", "2:b", "3:b1", "1:c"]));
}

// ---- Items: moving ----

#[tokio::test]
async fn a_subtree_moves_past_its_sibling_on_the_same_page() {
    let page = new_doc_page("items-move").await;
    let rows = fill(&page.id, &["a", "b", "  b1", "c"]).await;

    move_items(&[&rows[1].id, &rows[2].id], &page.id, &format!("after:{}", rows[3].id), None).await;
    assert_eq!(outline_of(&page.id).await, vs(&["1:a", "1:c", "1:b", "2:b1"]));

    move_items(&[&rows[1].id, &rows[2].id], &page.id, "first", None).await;
    assert_eq!(outline_of(&page.id).await, vs(&["1:b", "2:b1", "1:a", "1:c"]));
}

#[tokio::test]
async fn items_move_to_another_page_with_their_levels_shifted() {
    let from = new_doc_page("items-move-from").await;
    let to = new_doc_page("items-move-to").await;
    let rows = fill(&from.id, &["stay", "go", "  go-child"]).await;
    let target = fill(&to.id, &["parent"]).await.remove(0);

    let moved = move_items(&[&rows[1].id, &rows[2].id], &to.id, &format!("after:{}", target.id), Some(1)).await;

    assert_eq!(moved.iter().map(ItemRow::outline).collect::<Vec<_>>(), vs(&["2:go", "3:go-child"]));
    assert_eq!(outline_of(&from.id).await, vs(&["1:stay"]));
    assert_eq!(outline_of(&to.id).await, vs(&["1:parent", "2:go", "3:go-child"]));
}

// ---- Items: tasks ----

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
    assert!(running.properties.contains_key("begin"), "response: {:?}", running.properties);
    assert!(items(&page.id).await[0].properties.contains_key("begin"));

    let (_, resp) = post_empty(&cycle).await;
    let done = ItemRow::of(&resp["item"]);
    assert!(done.properties.contains_key("end"), "response: {:?}", done.properties);
    assert!(items(&page.id).await[0].properties.contains_key("end"));

    assert_eq!(post_empty(&cycle).await.0, 200);
    let normal = items(&page.id).await.remove(0);
    assert!(!normal.types.contains(&"task".to_string()));
    assert!(!normal.properties.contains_key("begin"));
    assert!(!normal.properties.contains_key("end"));
}

#[tokio::test]
async fn tasks_can_be_listed_by_type() {
    let page = new_doc_page("items-task-list").await;
    let item = fill(&page.id, &["listed task"]).await.remove(0);
    assert_eq!(post_empty(&format!("/api/items/{}/cycle-task", item.id)).await.0, 200);

    let (status, resp) = get("/api/items?types=task").await;
    assert_eq!(status, 200);
    let ids: Vec<Value> = field_of_each(&resp, "items", "id");
    assert!(ids.contains(&json!(item.id)));
}

// ---- Items: transclusion ----

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
    assert_eq!(linked.transclude_item_id.as_deref(), Some(source[0].id.as_str()));

    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    let tree = &resp["transcludedTrees"][&source[0].id];
    assert_eq!(field_of_each(&json!({ "t": tree }), "t", "body"), vec![json!("root"), json!("child")]);

    assert_eq!(delete(&format!("/api/items/{}/transclude", host.id)).await, 200);
    assert!(items(&page.id).await[0].transclude_item_id.is_none());
}

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

// ---- Pages ----

#[tokio::test]
async fn a_new_doc_page_starts_with_one_empty_item_and_is_found_again_by_its_title() {
    let title = format!("pages-new-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

    let (_, first) = post("/api/pages/doc", json!({ "title": title, "dirPath": "" })).await;
    let (_, again) = post("/api/pages/doc", json!({ "title": title, "dirPath": "" })).await;

    assert_eq!(first["created"], json!(true));
    assert_eq!(field_of_each(&first, "items", "body"), vec![json!("")]);
    assert_eq!(first["page"]["type"], json!("doc"));
    assert_eq!(again["created"], json!(false));
    assert_eq!(first["page"]["id"], again["page"]["id"]);
}

#[tokio::test]
async fn get_page_returns_the_page_with_its_items_in_order() {
    let page = new_doc_page("pages-get").await;
    fill(&page.id, &["one", "  two", "three"]).await;

    let (status, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(status, 200);

    assert_eq!(resp["page"]["name"], json!(page.title));
    assert_eq!(field_of_each(&resp, "items", "body"), vec![json!("one"), json!("two"), json!("three")]);
    assert_eq!(field_of_each(&resp, "items", "level"), vec![json!(1), json!(2), json!(1)]);
}

#[tokio::test]
async fn renaming_a_page_changes_its_title_and_path() {
    let page = new_doc_page("pages-rename").await;

    let (status, resp) = put(
        &format!("/api/pages/{}", page.id),
        json!({ "title": format!("{}-renamed", page.title), "dirPath": "notes/" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");

    assert_eq!(resp["page"]["name"], json!(format!("{}-renamed", page.title)));
    assert_eq!(resp["page"]["dirPath"], json!("notes/"));
    let (_, again) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(again["page"]["name"], json!(format!("{}-renamed", page.title)));
}

#[tokio::test]
async fn renaming_to_the_name_of_another_page_is_refused_unless_it_is_overwritten() {
    let page = new_doc_page("pages-overwrite").await;
    let other = new_doc_page("pages-overwritten").await;
    fill(&page.id, &["mine"]).await;
    fill(&other.id, &["theirs"]).await;

    let (status, resp) = put(&format!("/api/pages/{}", page.id), json!({ "title": other.title })).await;
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
    assert!(outline_of(&page.id).await.iter().any(|line| line.ends_with(":mine")));
}

#[tokio::test]
async fn the_most_recently_touched_page_comes_first_in_recent_pages() {
    let older = new_doc_page("pages-recent-older").await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let newer = new_doc_page("pages-recent-newer").await;

    // Other tests create pages at the same time, so only the relative order is checked
    let (status, resp) = get("/api/pages/recent?maxResults=200&type=doc").await;
    assert_eq!(status, 200);
    let ids: Vec<Value> = field_of_each(&resp, "pages", "id");

    let pos = |id: &str| ids.iter().position(|v| v == &json!(id));
    assert!(pos(&newer.id).unwrap() < pos(&older.id).unwrap(), "{ids:?}");
}

#[tokio::test]
async fn a_deleted_page_is_gone() {
    let page = new_doc_page("pages-delete").await;

    assert_eq!(delete(&format!("/api/pages/{}", page.id)).await, 204);

    assert_eq!(get(&format!("/api/pages/{}", page.id)).await.0, 404);
}

#[tokio::test]
async fn a_wiki_link_to_a_page_shows_up_in_its_backlinks() {
    let target = new_doc_page("pages-target").await;
    let source = new_doc_page("pages-source").await;
    fill(&source.id, &[&format!("see [[{}]]", target.title)]).await;

    let (status, resp) = get(&format!("/api/pages/{}/backlinks", target.id)).await;
    assert_eq!(status, 200);

    assert_eq!(
        resp["entries"].as_array().unwrap().iter().map(|e| e["sourcePage"]["id"].clone()).collect::<Vec<_>>(),
        vec![json!(source.id)]
    );
    assert_eq!(resp["entries"][0]["sourceItem"]["body"], json!(format!("see [[{}]]", target.title)));
}

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

#[tokio::test]
async fn a_week_is_created_with_one_link_per_day() {
    let week = journal("1999-W10").await;

    assert_eq!(week["created"], json!(true));
    let bodies = field_of_each(&week, "items", "body");
    assert_eq!(bodies.len(), 7);
    assert_eq!(bodies[0], json!("[[1999-03-08]]")); // ISO week 10 of 1999 starts on Monday the 8th
    assert_eq!(bodies[6], json!("[[1999-03-14]]"));
}

#[tokio::test]
async fn the_most_recent_week_before_a_week_is_found() {
    journal("1999-W20").await;
    journal("1999-W22").await;

    let (_, before) = get("/api/journals/weekly/before?week=1999-W22").await;

    assert_eq!(before["found"], json!(true));
    assert_eq!(before["page"]["name"], json!("1999-W20"));
}

#[tokio::test]
async fn a_name_that_is_not_a_journal_is_refused() {
    assert_eq!(post_empty("/api/journals/not-a-date").await.0, 400);
}

#[tokio::test]
async fn an_item_moves_to_another_day() {
    let from = journal("1999-09-01").await["page"]["id"].as_str().unwrap().to_string();
    let to = journal("1999-09-02").await["page"]["id"].as_str().unwrap().to_string();
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

#[tokio::test]
async fn page_names_are_found_by_prefix() {
    let page = new_doc_page("Searchable").await;

    let resp = search("/api/search/pages", &page.title[..12]).await;
    let titles = field_of_each(&resp, "results", "pageTitle");

    assert!(titles.contains(&json!(page.title)), "titles: {titles:?}");
}

#[tokio::test]
async fn item_text_is_found_with_a_snippet() {
    let page = new_doc_page("search-text").await;
    let word = format!("zebra{}", nanos());
    let item = fill(&page.id, &[&format!("the {word} crossing"), "nothing here"]).await.remove(0);

    let resp = search("/api/search/combined", &word).await;

    assert_eq!(field_of_each(&resp, "results", "itemId"), vec![json!(item.id)]);
    assert_eq!(resp["results"][0]["pageId"], json!(page.id));
    // The analyzer cuts letters from digits, so the match is marked word by word
    let snippet = resp["results"][0]["snippet"].as_str().unwrap_or("");
    assert!(snippet.contains("<mark>zebra"), "{snippet}");
}

#[tokio::test]
async fn two_words_must_both_be_in_the_item() {
    let page = new_doc_page("search-and").await;
    let a = format!("apple{}", nanos());
    let b = format!("banana{}", nanos());
    let rows = fill(&page.id, &[&format!("{a} and {b}"), &format!("{a} alone")]).await;

    let resp = search("/api/search/combined", &format!("{a} {b}")).await;

    assert_eq!(field_of_each(&resp, "results", "itemId"), vec![json!(rows[0].id)]);
}

#[tokio::test]
async fn a_hashtag_in_the_body_is_found_as_a_tag() {
    let page = new_doc_page("search-tag").await;
    let tag = format!("tag{}", nanos());
    let item = fill(&page.id, &[&format!("remember #{tag}")]).await.remove(0);

    assert_eq!(items(&page.id).await[0].properties["tags"], json!([tag]));
    let resp = search("/api/search/tags", &tag).await;
    assert_eq!(field_of_each(&resp, "results", "itemId"), vec![json!(item.id)]);
}

#[tokio::test]
async fn a_tag_is_found_in_any_case_of_its_letters() {
    let page = new_doc_page("search-tag-case").await;
    let tag = format!("CaseTag{}", nanos());
    let item = fill(&page.id, &[&format!("remember #{tag}")]).await.remove(0);

    let resp = search("/api/search/tags", &tag.to_lowercase()).await;
    assert_eq!(field_of_each(&resp, "results", "itemId"), vec![json!(item.id)]);
    let resp = search("/api/search/combined", &format!("#{}", tag.to_uppercase())).await;
    assert_eq!(field_of_each(&resp, "results", "itemId"), vec![json!(item.id)]);
}

#[tokio::test]
async fn a_deleted_item_is_no_longer_found() {
    let page = new_doc_page("search-deleted").await;
    let word = format!("ghost{}", nanos());
    let item = fill(&page.id, &[&word, "stays"]).await.remove(0);

    assert_eq!(delete(&format!("/api/items/{}", item.id)).await, 204);

    let resp = search("/api/search/combined", &word).await;
    assert_eq!(field_of_each(&resp, "results", "itemId"), Vec::<Value>::new());
}

// ---- Markdown conversions ----

async fn parse(markdown: &str) -> Value {
    let (status, resp) = post("/api/items/parse", json!({ "markdown": markdown })).await;
    assert_eq!(status, 200, "{resp}");
    resp
}

#[tokio::test]
async fn flat_markdown_is_split_into_items_at_block_boundaries() {
    let resp = parse("# Night\n\n- [ ] Hanon\n\n- [ ] Upload\n").await;

    assert_eq!(
        field_of_each(&resp, "items", "body"),
        vec![json!("# Night"), json!("- [ ] Hanon\n\n- [ ] Upload")]
    );
    assert_eq!(field_of_each(&resp, "items", "level"), vec![json!(1), json!(1)]);
}

#[tokio::test]
async fn metadata_lines_carry_the_level_of_each_item() {
    let resp = parse("# Night\n\n<!-- @@ -->\n- [ ] Hanon\n\n<!-- @@ -->\n- [ ] Upload\n").await;

    assert_eq!(
        field_of_each(&resp, "items", "body"),
        vec![json!("# Night"), json!("- [ ] Hanon"), json!("- [ ] Upload")]
    );
    assert_eq!(field_of_each(&resp, "items", "level"), vec![json!(1), json!(2), json!(2)]);
}

#[tokio::test]
async fn exported_items_come_back_through_import_and_parse_unchanged() {
    let page = new_doc_page("markdown-roundtrip").await;
    let rows = fill(&page.id, &["parent", "  child one", "    grandchild", "  child two", "sibling"]).await;
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();

    let (status, exported) = post("/api/items/export", json!({ "itemIds": ids })).await;
    assert_eq!(status, 200, "{exported}");
    let tree = exported["treeMarkdown"].as_str().unwrap();
    assert!(tree.contains("- parent"), "{tree}");
    assert!(tree.contains("    - grandchild"), "{tree}");
    assert!(exported["ast"].as_str().unwrap().contains("\"type\""));
    assert!(exported["flatMarkdown"].as_str().unwrap().contains("<!-- @"), "flat markdown keeps the item metadata");

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
    server().markdown_dir.join("docs").join(format!("{}.md", page.title))
}

fn read(file: &std::path::Path) -> String {
    std::fs::read_to_string(file).unwrap_or_default()
}

/// The export scheduler starts a few seconds after the server and then runs every second.
async fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !condition() {
        assert!(std::time::Instant::now() < deadline, "timed out waiting until {what}");
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
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

#[tokio::test]
async fn the_status_names_the_folder_and_says_it_is_there() {
    let (status, resp) = get("/api/sync/status").await;
    assert_eq!(status, 200);

    assert_eq!(resp["importEnabled"], json!(true));
    assert_eq!(resp["markdownRoot"], json!(server().markdown_dir.to_string_lossy()));
    assert_eq!(resp["rootAvailable"], json!(true));
}

#[tokio::test]
async fn a_page_is_written_to_its_markdown_file() {
    let page = new_doc_page("sync-export").await;
    fill(&page.id, &["first line", "  second line"]).await;

    wait_until("the page is exported", || read(&file_of(&page)).contains("second line")).await;

    let markdown = read(&file_of(&page));
    assert!(markdown.starts_with(&format!("---\nid: {}\n", page.id)), "{markdown}");
    assert!(markdown.contains("first line"), "{markdown}");
}

#[tokio::test]
async fn a_change_made_to_the_file_is_merged_into_the_page() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("sync-import").await;
    fill(&page.id, &["before the edit"]).await;
    wait_until("the page is exported", || read(&file_of(&page)).contains("before the edit")).await;

    std::fs::write(
        file_of(&page),
        read(&file_of(&page)).replace("before the edit", "edited on disk, a bit longer"),
    )
    .unwrap();
    let result = scan().await;

    assert!(result["imported"].as_i64().unwrap() >= 1, "{result}");
    assert_eq!(outline_of(&page.id).await, vs(&["1:edited on disk, a bit longer"]));
}

#[tokio::test]
async fn a_new_file_in_the_folder_becomes_a_page() {
    let _folder = FOLDER_SCAN.lock().await;
    let name = format!("sync-new-{}", nanos());
    let docs = server().markdown_dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join(format!("{name}.md")), "hello from disk\n\nsecond block\n").unwrap();

    let result = scan().await;

    assert!(result["imported"].as_i64().unwrap() >= 1, "{result}");
    let resp = search("/api/search/pages", &name).await;
    let ids = field_of_each(&resp, "results", "pageId");
    assert_eq!(ids.len(), 1, "the new page is found by its file name: {resp}");
    assert_eq!(
        outline_of(ids[0].as_str().unwrap()).await,
        vs(&["1:hello from disk", "1:second block"])
    );
}

#[tokio::test]
async fn an_unchanged_file_is_left_alone() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("sync-unchanged").await;
    fill(&page.id, &["stable"]).await;
    wait_until("the page is exported", || read(&file_of(&page)).contains("stable")).await;
    scan().await;

    // Other tests may be exporting their own pages at the same time; this page itself must stay as it is
    scan().await;

    assert_eq!(outline_of(&page.id).await, vs(&["1:stable"]));
}

// ---- Health and config ----

#[tokio::test]
async fn health_and_config() {
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
    resp["definitions"].as_array().unwrap().iter().find(|d| d["key"] == key)
}

#[tokio::test]
async fn property_definitions_list_the_system_keys_and_accept_user_keys() {
    let (status, resp) = get("/api/config/property-definitions").await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["propertyDefinitions"]["zoom"], json!("number"));
    assert_eq!(definition(&resp, "tags").unwrap()["system"], json!(true));
    assert_eq!(definition(&resp, "tags").unwrap()["page"], json!(true));
    assert_eq!(definition(&resp, "zoom").unwrap()["page"], json!(false));
    assert_eq!(definition(&resp, "modified").unwrap()["system"], json!(true));
    assert_eq!(definition(&resp, "due").unwrap()["system"], json!(true));
    assert_eq!(definition(&resp, "cancelled").unwrap()["page"], json!(false));
    assert_eq!(definition(&resp, "created").unwrap()["page"], json!(true));
    assert_eq!(definition(&resp, "published").unwrap()["system"], json!(true));
    assert_eq!(definition(&resp, "published").unwrap()["page"], json!(true));
    assert_eq!(resp["propertyDefinitions"]["published"], json!("datetime"));
    assert_eq!(definition(&resp, "atlas_bounds").unwrap()["system"], json!(true));
    assert_eq!(definition(&resp, "atlas_bounds").unwrap()["page"], json!(true));
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

#[tokio::test]
async fn a_property_can_be_removed_from_an_item() {
    let page = new_doc_page("items-property-delete").await;
    let item = fill(&page.id, &["task-like"]).await.remove(0);
    put(
        &format!("/api/items/{}/properties/plan", item.id),
        json!({ "valueType": "date", "value": "2026-03-01" }),
    )
    .await;

    let (status, resp) = send(reqwest::Method::DELETE, &format!("/api/items/{}/properties/plan", item.id), None).await;
    assert_eq!(status, 200, "{resp}");

    assert!(resp["item"]["properties"].get("plan").is_none());
    assert!(items(&page.id).await[0].properties.get("plan").is_none());
}

#[tokio::test]
async fn an_item_property_change_alone_rewrites_the_page_file() {
    let page = new_doc_page("items-property-export").await;
    let item = fill(&page.id, &["planned"]).await.remove(0);
    wait_until("the page file exists", || read(&file_of(&page)).contains("planned")).await;

    put(
        &format!("/api/items/{}/properties/plan", item.id),
        json!({ "valueType": "date", "value": "2026-03-01" }),
    )
    .await;

    wait_until("the plan is in the file", || read(&file_of(&page)).contains("plan=\"2026-03-01\"")).await;
}

#[tokio::test]
async fn page_properties_are_set_removed_and_written_to_the_front_matter() {
    let page = new_doc_page("page-properties").await;
    post("/api/config/property-definitions", json!({ "key": "owner", "valueType": "text" })).await;

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

    wait_until("the front matter has the properties", || {
        let text = read(&file_of(&page));
        text.contains("owner: \"me: now\"\n") && text.contains("tags:\n  - a\n  - b\n---\n")
    })
    .await;

    let (status, resp) = send(reqwest::Method::DELETE, &format!("/api/pages/{}/properties/owner", page.id), None).await;
    assert_eq!(status, 200, "{resp}");
    assert!(resp["page"]["properties"].get("owner").is_none());
    assert_eq!(resp["page"]["properties"]["tags"], json!(["a", "b"]));

    wait_until("the owner left the front matter", || !read(&file_of(&page)).contains("owner:")).await;
}

/// Held by a test that changes the tag directory, and by the tests of tag pages. A tag page
/// made while the directory is changed would be looked for in the wrong folder later.
static TAGS_DIR: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The id of a doc page by its name.
async fn doc_page_named(name: &str) -> Option<String> {
    let (_, resp) = get("/api/pages/recent?maxResults=200&type=doc").await;
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

    let (status, resp) = send(reqwest::Method::DELETE, &format!("/api/pages/{}/properties/tags", second.id), None).await;
    assert_eq!(status, 200, "{resp}");
    assert!(doc_page_named(tag).await.is_none());
}

/// The ids of the doc pages with this name in any case of its letters.
async fn doc_pages_named_ignoring_case(name: &str) -> Vec<String> {
    let (_, resp) = get("/api/pages/recent?maxResults=200&type=doc").await;
    resp["pages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["name"].as_str().unwrap().eq_ignore_ascii_case(name))
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn tags_that_differ_only_in_case_share_one_tag_page() {
    let _tags_dir = TAGS_DIR.lock().await;
    let first = new_doc_page("case-tag-pages-first").await;
    let second = new_doc_page("case-tag-pages-second").await;

    set_page_tags(&first.id, "Case-Tag-Pages").await;
    set_page_tags(&second.id, "case-tag-pages").await;
    assert_eq!(doc_pages_named_ignoring_case("case-tag-pages").await.len(), 1);

    // The other case still uses the page
    set_page_tags(&first.id, "case-tag-pages-other").await;
    assert_eq!(doc_pages_named_ignoring_case("case-tag-pages").await.len(), 1);

    set_page_tags(&second.id, "case-tag-pages-other").await;
    assert!(doc_pages_named_ignoring_case("case-tag-pages").await.is_empty());
}

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

    let added = create_item(&page.id, &format!("after:{}", item.id), 1, &format!("#{tag}")).await;
    assert!(doc_page_named(tag).await.is_some());
    assert_eq!(delete(&format!("/api/items/{}", added.id)).await, 204);
    assert!(doc_page_named(tag).await.is_none());
}

#[tokio::test]
async fn renaming_a_tag_page_renames_its_tag_in_page_tags_and_bodies() {
    let _tags_dir = TAGS_DIR.lock().await;
    let old = format!("renamed-tag-old-{}", nanos());
    let new = format!("renamed-tag-new-{}", nanos());
    let tagged = new_doc_page("renamed-tag-page").await;
    set_page_tags(&tagged.id, &format!("{old},other")).await;
    let page = new_doc_page("renamed-tag-body").await;
    let item = items(&page.id).await.into_iter().next().unwrap();
    update_item(&item.id, json!({ "body": format!("see #{old} and `#{old}`") })).await;
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

#[tokio::test]
async fn a_tag_page_cannot_take_a_title_that_is_not_a_tag() {
    let _tags_dir = TAGS_DIR.lock().await;
    let tag = format!("untitled-tag-{}", nanos());
    let page = new_doc_page("untitled-tag").await;
    set_page_tags(&page.id, &tag).await;
    let tag_page = doc_page_named(&tag).await.unwrap();

    let (status, resp) = put(&format!("/api/pages/{tag_page}"), json!({ "title": "two words" })).await;
    assert_eq!(status, 400, "{resp}");
    assert_eq!(doc_page_named(&tag).await, Some(tag_page));
}

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

#[tokio::test]
async fn the_tags_of_a_file_edited_outside_keep_their_tag_pages() {
    let _tags_dir = TAGS_DIR.lock().await;
    let _folder = FOLDER_SCAN.lock().await;
    let page_tag = "file-tag-pages-page";
    let body_tag = "file-tag-pages-body";
    let page = new_doc_page("file-tag-pages").await;
    fill(&page.id, &["before the edit"]).await;
    wait_until("the page is exported", || read(&file_of(&page)).contains("before the edit")).await;

    edit_file(&page, |text| {
        text.replacen("---\n\n", &format!("tags:\n  - \"#{page_tag}\"\n---\n\n"), 1)
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
            .map(|l| if l.contains(body_tag) { "no tag now\n".to_string() } else { format!("{l}\n") })
            .collect()
    });
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert!(resp["page"]["properties"].get("tags").is_none(), "{resp}");
    assert!(doc_page_named(page_tag).await.is_none());
    assert!(doc_page_named(body_tag).await.is_none());
}

#[tokio::test]
async fn a_file_written_with_the_old_tag_key_keeps_its_tags() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("file-old-tag-key").await;
    fill(&page.id, &["before the edit"]).await;
    wait_until("the page is exported", || read(&file_of(&page)).contains("before the edit")).await;

    edit_file(&page, |text| {
        text.replacen("---\n\n", "tag:\n  - old-key\n---\n\n", 1)
            .replace("before the edit", "after the edit")
    });
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(resp["page"]["properties"]["tags"], json!(["old-key"]));
    wait_until("the file has the new key", || read(&file_of(&page)).contains("tags:\n  - old-key\n")).await;
}

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
    let (status, _) = send(reqwest::Method::DELETE, &format!("/api/pages/{}/properties/created", page.id), None).await;
    assert_eq!(status, 400);

    wait_until("the front matter has the new created date", || {
        read(&file_of(&page)).contains("created: 2020-01-02T03:04:05Z\n")
    })
    .await;
}

#[tokio::test]
async fn the_children_of_an_index_page_carry_their_scalar_properties() {
    // A unique directory so other tests' pages stay out of the listing
    let dir = format!("{}/", new_doc_page("children-props").await.title);
    let (status, resp) = post("/api/pages/doc", json!({ "title": "a-page", "dirPath": dir })).await;
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
    let child = resp["pages"].as_array().unwrap().iter().find(|p| p["id"] == page_id).unwrap();
    assert_eq!(child["properties"]["published"], json!("2026-10-20T09:00:00Z"));

    let (status, resp) = get("/api/pages/recent?maxResults=50").await;
    assert_eq!(status, 200, "{resp}");
    let recent = resp["pages"].as_array().unwrap().iter().find(|p| p["id"] == page_id).unwrap();
    assert_eq!(recent["properties"]["published"], json!("2026-10-20T09:00:00Z"));
}

#[tokio::test]
async fn the_children_of_an_index_page_carry_their_tags_but_no_other_list() {
    let _tags_dir = TAGS_DIR.lock().await;
    let dir = format!("{}/", new_doc_page("children-tags").await.title);
    let (status, resp) = post("/api/pages/doc", json!({ "title": "a-page", "dirPath": dir })).await;
    assert_eq!(status, 200, "{resp}");
    let page_id = resp["page"]["id"].as_str().unwrap().to_string();
    let parent_id = resp["page"]["parentId"].as_str().unwrap().to_string();
    set_page_tags(&page_id, "children-tag-b, children-tag-a").await;
    let group = format!("atlas_group_{}", nanos());
    put(
        &format!("/api/pages/{page_id}/properties/{group}"),
        json!({ "valueType": "list", "value": "x,y" }),
    )
    .await;

    let (_, resp) = get(&format!("/api/pages/{parent_id}/children")).await;
    let child = resp["pages"].as_array().unwrap().iter().find(|p| p["id"] == page_id).unwrap();
    assert_eq!(child["properties"]["tags"], json!(["children-tag-b", "children-tag-a"]));
    assert!(child["properties"].get(&group).is_none(), "{child}");
}

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

    let (status, resp) = send(reqwest::Method::DELETE, "/api/config/property-definitions/rating", None).await;
    assert_eq!(status, 200, "{resp}");
    assert!(definition(&resp, "rating").is_none(), "{resp}");

    assert!(items(&page.id).await[0].properties.get("rating").is_none());
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert!(resp["page"]["properties"].get("rating").is_none(), "{resp}");
    assert!(resp["page"]["modifiedDate"].as_i64().unwrap() >= modified_before);

    // System keys and unknown keys are refused
    let (status, _) = send(reqwest::Method::DELETE, "/api/config/property-definitions/tags", None).await;
    assert_eq!(status, 400);
    let (status, _) = send(reqwest::Method::DELETE, "/api/config/property-definitions/rating", None).await;
    assert_eq!(status, 400);
}

// ---- the atlas page of a directory ----

async fn atlas_of(dir: &str) -> String {
    let (status, resp) = post("/api/pages/doc", json!({ "title": "index_atlas", "dirPath": dir })).await;
    assert_eq!(status, 200, "{resp}");
    resp["page"]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn an_atlas_page_is_not_found_by_its_name_but_its_items_are() {
    let dir = format!("{}/", new_doc_page("atlas-search").await.title);
    let atlas_id = atlas_of(&dir).await;
    let word = format!("compass{}", nanos());
    fill(&atlas_id, &[&format!("the {word} rose")]).await;

    for query in [format!("{dir}index_atlas"), "index_atlas".to_string()] {
        let resp = search("/api/search/pages", &query).await;
        let ids = field_of_each(&resp, "results", "pageId");
        assert!(!ids.contains(&json!(atlas_id)), "{query}: {ids:?}");
    }
    let resp = search("/api/search/combined", &word).await;
    assert_eq!(resp["results"][0]["pageId"], json!(atlas_id));
}

#[tokio::test]
async fn the_atlas_page_cannot_be_renamed_or_moved() {
    let dir = format!("{}/", new_doc_page("atlas-rename").await.title);
    let atlas_id = atlas_of(&dir).await;

    let (status, resp) = put(&format!("/api/pages/{atlas_id}"), json!({ "title": "map" })).await;
    assert_eq!(status, 400, "{resp}");
    let (status, resp) = put(&format!("/api/pages/{atlas_id}"), json!({ "dirPath": "elsewhere/" })).await;
    assert_eq!(status, 400, "{resp}");
}

#[tokio::test]
async fn no_other_page_can_take_the_atlas_name() {
    let page = new_doc_page("atlas-reserved").await;

    let (status, resp) = put(&format!("/api/pages/{}", page.id), json!({ "title": "index_atlas" })).await;
    assert_eq!(status, 400, "{resp}");
}

#[tokio::test]
async fn a_bounds_property_holds_four_numbers() {
    let (status, resp) = post(
        "/api/config/property-definitions",
        json!({ "key": "area", "valueType": "bounds" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(definition(&resp, "area").unwrap()["valueType"], json!("bounds"));

    let page = new_doc_page("bounds").await;
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/area", page.id),
        json!({ "valueType": "bounds", "value": " 10, 20.5,160 ,-56 " }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["properties"]["area"], json!("10,20.5,160,-56"));

    for value in ["1,2,3", "1,2,3,4,5", "1,2,x,4", "", "1,2,3,inf"] {
        let (status, resp) = put(
            &format!("/api/pages/{}/properties/area", page.id),
            json!({ "valueType": "bounds", "value": value }),
        )
        .await;
        assert_eq!(status, 400, "{value} -> {resp}");
    }
}

#[tokio::test]
async fn a_page_takes_its_bounds_on_the_atlas() {
    let page = new_doc_page("atlas-bounds").await;
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/atlas_bounds", page.id),
        json!({ "valueType": "bounds", "value": "24, 24, 160, 56" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["properties"]["atlas_bounds"], json!("24,24,160,56"));

    let (status, resp) = post(
        "/api/config/property-definitions",
        json!({ "key": "atlas_bounds", "valueType": "bounds" }),
    )
    .await;
    assert_eq!(status, 400, "{resp}");
}

#[tokio::test]
async fn an_atlas_keeps_a_card_group_as_a_list_of_ids_out_of_the_settings() {
    let page = new_doc_page("atlas-group").await;
    let key = format!("atlas_group_{}", nanos());
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/{key}", page.id),
        json!({ "valueType": "list", "value": "id-b, id-a" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    // The order is kept, and no tag is made of an id
    assert_eq!(resp["page"]["properties"][&key], json!(["id-b", "id-a"]));
    assert!(resp["page"]["properties"].get("tags").is_none(), "{resp}");
    wait_until("the group is written as a list", || {
        read(&file_of(&page)).contains(&format!("{key}:\n  - id-b\n  - id-a\n"))
    })
    .await;

    let (_, resp) = get("/api/config/property-definitions").await;
    assert!(definition(&resp, &key).is_none(), "{resp}");
    assert!(resp["propertyDefinitions"].get(&key).is_none(), "{resp}");

    let (status, resp) = send(
        reqwest::Method::DELETE,
        &format!("/api/pages/{}/properties/{key}", page.id),
        None,
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert!(resp["page"]["properties"].get(&key).is_none(), "{resp}");
}

#[tokio::test]
async fn the_atlas_of_a_file_edited_outside_is_taken_into_the_page() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("file-atlas").await;
    let key = format!("atlas_group_{}", nanos());
    fill(&page.id, &["before the edit"]).await;
    wait_until("the page is exported", || read(&file_of(&page)).contains("before the edit")).await;

    edit_file(&page, |text| {
        text.replacen(
            "---\n\n",
            &format!("atlas_bounds: \"-10, 20.5,120,40\"\n{key}:\n  - id-b\n  - id-a\n---\n\n"),
            1,
        )
    });
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(resp["page"]["properties"]["atlas_bounds"], json!("-10,20.5,120,40"));
    assert_eq!(resp["page"]["properties"][&key], json!(["id-b", "id-a"]));
    wait_until("the file has the bounds as exported", || {
        read(&file_of(&page)).contains("atlas_bounds: \"-10,20.5,120,40\"\n")
    })
    .await;

    edit_file(&page, |text| {
        text.lines()
            .filter(|l| !l.starts_with("atlas_bounds") && !l.starts_with(&key) && !l.starts_with("  - id-"))
            .map(|l| format!("{l}\n"))
            .collect()
    });
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert!(resp["page"]["properties"].get("atlas_bounds").is_none(), "{resp}");
    assert!(resp["page"]["properties"].get(&key).is_none(), "{resp}");
}

#[tokio::test]
async fn a_group_title_stays_out_of_the_settings_and_follows_its_file() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("atlas-title").await;
    let key = format!("atlas_title_{}", nanos());
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/{key}", page.id),
        json!({ "valueType": "text", "value": "top:Before" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    wait_until("the title is exported", || read(&file_of(&page)).contains(&format!("{key}: top:Before\n"))).await;
    let (_, resp) = get("/api/config/property-definitions").await;
    assert!(definition(&resp, &key).is_none(), "{resp}");

    edit_file(&page, |text| text.replace("top:Before", "left:After"));
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(resp["page"]["properties"][&key], json!("left:After"));

    edit_file(&page, |text| {
        text.lines().filter(|l| !l.starts_with(&key)).map(|l| format!("{l}\n")).collect()
    });
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert!(resp["page"]["properties"].get(&key).is_none(), "{resp}");
}

#[tokio::test]
async fn a_callout_stays_out_of_the_settings_and_follows_its_file() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("atlas-callout").await;
    let key = format!("atlas_callout_{}", nanos());
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/{key}", page.id),
        json!({ "valueType": "text", "value": "10,20,top:Hello" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    wait_until("the callout is exported", || {
        read(&file_of(&page)).contains(&format!("{key}: 10,20,top:Hello\n"))
    })
    .await;
    let (_, resp) = get("/api/config/property-definitions").await;
    assert!(definition(&resp, &key).is_none(), "{resp}");

    edit_file(&page, |text| text.replace("10,20,top:Hello", "10,20,left:Bye"));
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(resp["page"]["properties"][&key], json!("10,20,left:Bye"));

    let status = delete(&format!("/api/pages/{}/properties/{key}", page.id)).await;
    assert!((200..300).contains(&status), "{status}");
    wait_until("the callout leaves the file", || !read(&file_of(&page)).contains(&key)).await;
}

#[tokio::test]
async fn bounds_in_a_file_that_are_not_bounds_are_written_over_by_the_page() {
    let _folder = FOLDER_SCAN.lock().await;
    let page = new_doc_page("file-bad-bounds").await;
    put(
        &format!("/api/pages/{}/properties/atlas_bounds", page.id),
        json!({ "valueType": "bounds", "value": "1,2,3,4" }),
    )
    .await;
    wait_until("the bounds are exported", || read(&file_of(&page)).contains("atlas_bounds: 1,2,3,4\n")).await;

    edit_file(&page, |text| text.replace("atlas_bounds: 1,2,3,4", "atlas_bounds: here"));
    scan().await;
    let (_, resp) = get(&format!("/api/pages/{}", page.id)).await;
    assert_eq!(resp["page"]["properties"]["atlas_bounds"], json!("1,2,3,4"));
    wait_until("the file has the bounds again", || read(&file_of(&page)).contains("atlas_bounds: 1,2,3,4\n")).await;
}

#[tokio::test]
async fn a_value_is_checked_by_the_type_of_its_definition_not_the_type_sent() {
    let page = new_doc_page("defined-type").await;
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/atlas_bounds", page.id),
        json!({ "valueType": "text", "value": "not bounds" }),
    )
    .await;
    assert_eq!(status, 400, "{resp}");
    let (status, resp) = put(
        &format!("/api/pages/{}/properties/atlas_bounds", page.id),
        json!({ "valueType": "text", "value": "1, 2, 3, 4" }),
    )
    .await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(resp["page"]["properties"]["atlas_bounds"], json!("1,2,3,4"));

    let item = fill(&page.id, &["zoomed"]).await.remove(0);
    let (status, resp) = put(
        &format!("/api/items/{}/properties/zoom", item.id),
        json!({ "valueType": "text", "value": "large" }),
    )
    .await;
    assert_eq!(status, 400, "{resp}");
}

// ---- Extension functions ----

#[tokio::test]
async fn the_files_of_a_function_are_listed_by_name_without_its_folders() {
    let dir = server().root.join("config").join("functions").join("fn-list");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    for name in ["b.md", "a.md", "index.html"] {
        std::fs::write(dir.join(name), "x").unwrap();
    }

    let (status, resp) = get("/api/functions/fn-list").await;
    assert_eq!(status, 200, "{resp}");
    assert_eq!(strings(&resp), vs(&["a.md", "b.md", "index.html"]));
}

#[tokio::test]
async fn a_missing_function_has_no_file_list() {
    let (status, _) = get("/api/functions/fn-missing").await;
    assert_eq!(status, 404);
}
