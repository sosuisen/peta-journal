use crate::effects::{ApiError, ApiOp, ApiOutput, AtlasChange, FolderChoice};
use crate::model::{
    BatchOpInput, DocPage, ItemPosition, JournalBefore, MovedToJournal, Page, PageWithItems,
    PasteCursor, PasteMode, PropertyValue, ServerConfig, TaskState, TreeExport,
};
use crate::rest::{rest_call, HttpReply, RestCall};
use serde::Serialize;
use serde_json::{json, Value};

fn call(op: ApiOp) -> RestCall {
    rest_call(op).expect("this operation is a REST call")
}

fn reply(status: u16, body: Value) -> HttpReply {
    HttpReply {
        status,
        body: body.to_string(),
    }
}

fn answer(op: ApiOp, body: Value) -> ApiOutput {
    call(op).decode(&reply(200, body)).unwrap()
}

fn json_of(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}

fn item_json(id: &str, body: &str) -> Value {
    json!({
        "id": id, "pageId": "p", "parentItemId": null, "dirPath": "", "name": "n", "type": "doc",
        "createdDate": 0, "modifiedDate": 0, "idx": "a", "level": 1, "body": body, "ast": "",
        "collapsed": false, "types": [], "properties": {}, "transcludeItemId": null,
    })
}

fn id(item_id: &str) -> String {
    item_id.to_string()
}

fn atlas(change: AtlasChange) -> ApiOp {
    ApiOp::ChangeAtlas {
        atlas_id: id("m"),
        change,
    }
}

// 読み取りの問いは、サーバーが読む名前で、パスの後ろに付く。操作の場所そのものは、サーバーと同じ表（journal-api-typesのroutes）から取るので、ここでは確かめない。
#[test]
fn a_question_goes_after_the_path_under_the_names_the_server_reads() {
    let cases = [
        (
            ApiOp::GetMostRecentJournalBefore { before: id("d") },
            "/api/journals/before?date=d",
        ),
        (
            ApiOp::GetMostRecentWeeklyJournalBefore { before: id("w") },
            "/api/journals/weekly/before?week=w",
        ),
        (
            ApiOp::SearchCombined {
                query: id("q"),
                max_results: 5,
            },
            "/api/search/combined?q=q&maxResults=5",
        ),
        (
            ApiOp::SearchPageNames {
                query: id("q"),
                max_results: 5,
            },
            "/api/search/pages?q=q&maxResults=5",
        ),
        (
            ApiOp::GetRecentPages {
                max_results: 5,
                page_type: None,
            },
            "/api/pages/recent?maxResults=5",
        ),
        (
            ApiOp::GetRecentPages {
                max_results: 5,
                page_type: Some(id("doc")),
            },
            "/api/pages/recent?maxResults=5&type=doc",
        ),
        (
            ApiOp::FindPage {
                journal: Some(id("j")),
                title: String::new(),
                dir_path: String::new(),
            },
            "/api/pages/find?journal=j",
        ),
        (
            ApiOp::FindPage {
                journal: None,
                title: id("t"),
                dir_path: id("d/"),
            },
            "/api/pages/find?title=t&dirPath=d%2F",
        ),
    ];
    for (op, path) in cases {
        let request = call(op.clone()).request;
        assert_eq!(
            (request.method, request.path.as_str()),
            ("GET", path),
            "{op:?}"
        );
    }
}

// ページを探した答えは、見つかったページか、ないことである。
#[test]
fn finding_a_page_answers_the_page_or_none() {
    let find = || ApiOp::FindPage {
        journal: None,
        title: "t".into(),
        dir_path: String::new(),
    };
    let page = json!({
        "id": "p", "parentId": "", "dirPath": "", "name": "t", "type": "doc",
        "createdDate": 0, "modifiedDate": 0, "url": "/t", "properties": {}
    });

    let found = answer(find(), json!({ "page": page }));
    let none = answer(find(), json!({ "page": null }));

    assert!(matches!(found, ApiOutput::FoundPage(Some(p)) if p.id == "p"));
    assert_eq!(none, ApiOutput::FoundPage(None));
}

// 操作は、自分の値をパスと本文に入れて送る。パスには、どのページや項目かと、プロパティの名前が、この順で入る。
#[test]
fn an_operation_puts_its_values_into_the_path_and_the_body() {
    let property = json!({ "valueType": "text", "value": "v" });
    let cases = [
        (
            ApiOp::GetOrCreateDoc {
                title: id("t"),
                dir_path: id("d/"),
            },
            ("POST", "/api/pages/doc"),
            Some(json!({ "title": "t", "dirPath": "d/" })),
        ),
        (
            ApiOp::CycleTask { item_id: id("a") },
            ("POST", "/api/items/a/cycle-task"),
            None,
        ),
        (
            ApiOp::UpsertItemProperty {
                item_id: id("a"),
                key: id("k"),
                value_type: id("text"),
                value: id("v"),
            },
            ("PUT", "/api/items/a/properties/k"),
            Some(property.clone()),
        ),
        (
            ApiOp::UpsertPageProperty {
                page_id: id("p"),
                key: id("k"),
                value_type: id("text"),
                value: id("v"),
            },
            ("PUT", "/api/pages/p/properties/k"),
            Some(property),
        ),
        (
            ApiOp::AddPropertyDefinition {
                key: id("k"),
                value_type: id("text"),
            },
            ("POST", "/api/config/property-definitions"),
            Some(json!({ "key": "k", "valueType": "text" })),
        ),
        (
            ApiOp::DeletePropertyDefinition { key: id("k") },
            ("DELETE", "/api/config/property-definitions/k"),
            None,
        ),
        (
            ApiOp::SetTransclusion {
                item_id: id("a"),
                source_item_id: id("s"),
            },
            ("POST", "/api/items/a/transclude"),
            Some(json!({ "sourceItemId": "s" })),
        ),
        (
            ApiOp::RemoveTransclusion { item_id: id("a") },
            ("DELETE", "/api/items/a/transclude"),
            None,
        ),
        (
            ApiOp::AnswerConfirm {
                request_id: id("r"),
                ok: true,
            },
            ("POST", "/api/confirm/r"),
            Some(json!({ "ok": true })),
        ),
        (
            ApiOp::OpenExportFolder { page_id: id("p") },
            ("POST", "/api/pages/p/open-folder"),
            None,
        ),
    ];
    for (op, place, body) in cases {
        let request = call(op.clone()).request;
        assert_eq!((request.method, request.path.as_str()), place, "{op:?}");
        assert_eq!(request.body, body, "{op:?}");
    }
}

// 地図の変更は、変更の種類ごとの場所へ、その内容を送る。
#[test]
fn a_change_of_an_atlas_goes_to_its_own_endpoint_with_its_fields() {
    let cases = vec![
        (
            AtlasChange::AddLink {
                target: id("t"),
                bounds: id("b"),
            },
            "POST",
            "/api/pages/m/atlas/links",
            Some(json!({ "target": "t", "bounds": "b" })),
        ),
        (
            AtlasChange::SetLink {
                item_id: id("a"),
                target: id("t"),
            },
            "PUT",
            "/api/pages/m/atlas/links/a",
            Some(json!({ "target": "t" })),
        ),
        (
            AtlasChange::Group {
                cards: vec![id("a")],
                target: id("t"),
            },
            "POST",
            "/api/pages/m/atlas/group",
            Some(json!({ "cards": ["a"], "target": "t" })),
        ),
        (
            AtlasChange::Ungroup {
                cards: vec![id("a")],
            },
            "POST",
            "/api/pages/m/atlas/ungroup",
            Some(json!({ "cards": ["a"] })),
        ),
        (
            AtlasChange::SetGroupTitle {
                item_id: id("a"),
                side: id("s"),
                title: id("t"),
            },
            "PUT",
            "/api/pages/m/atlas/groups/a",
            Some(json!({ "side": "s", "title": "t" })),
        ),
        (
            AtlasChange::AddCallout {
                bounds: id("b"),
                side: id("s"),
                text: id("x"),
            },
            "POST",
            "/api/pages/m/atlas/callouts",
            Some(json!({ "bounds": "b", "side": "s", "text": "x" })),
        ),
        (
            AtlasChange::SetCallout {
                item_id: id("a"),
                side: id("s"),
                text: id("x"),
            },
            "PUT",
            "/api/pages/m/atlas/callouts/a",
            Some(json!({ "side": "s", "text": "x" })),
        ),
        (
            AtlasChange::Remove { item_id: id("a") },
            "DELETE",
            "/api/pages/m/atlas/items/a",
            None,
        ),
    ];
    for (change, method, path, body) in cases {
        let request = call(atlas(change)).request;
        assert_eq!(request.method, method);
        assert_eq!(request.path, path);
        assert_eq!(request.body, body);
    }
}

// 項目の作成は、位置をサーバーの形にして送り、返された項目を答える。
#[test]
fn creating_an_item_posts_its_position_and_answers_the_item() {
    let rest = call(ApiOp::CreateItem {
        page_id: id("p"),
        position: ItemPosition::after("a"),
        level: 2,
        body: "本文".into(),
        id: None,
    });
    assert_eq!(
        (rest.request.method, rest.request.path.as_str()),
        ("POST", "/api/items")
    );
    assert_eq!(
        rest.request.body,
        Some(json!({ "pageId": "p", "position": "after:a", "level": 2, "body": "本文" }))
    );
    let output = rest
        .decode(&reply(200, json!({ "item": item_json("new", "本文") })))
        .unwrap();
    let ApiOutput::Item(item) = output else {
        panic!("not an item: {output:?}");
    };
    assert_eq!(item.body, "本文");
}

// 貼り付けは、本文とカーソルと文字列と読み方を送り、書かれた項目とカーソルを答える。
#[test]
fn a_paste_posts_the_body_the_cursor_the_text_and_the_mode_and_answers_what_was_written() {
    let rest = call(ApiOp::PasteText {
        item_id: id("a"),
        body: "ab".into(),
        cursor: 1,
        text: "本文".into(),
        mode: PasteMode::Flat,
    });
    assert_eq!(
        (rest.request.method, rest.request.path.as_str()),
        ("POST", "/api/items/a/paste")
    );
    assert_eq!(
        rest.request.body,
        Some(json!({ "body": "ab", "cursor": 1, "text": "本文", "mode": "flat" }))
    );
    let output = rest
        .decode(&reply(
            200,
            json!({
                "items": [item_json("a", "a本文"), item_json("n", "b")],
                "cursor": { "itemId": "n", "position": 2 },
            }),
        ))
        .unwrap();
    let ApiOutput::Pasted(pasted) = output else {
        panic!("not a paste: {output:?}");
    };
    let bodies: Vec<&str> = pasted.items.iter().map(|i| i.body.as_str()).collect();
    assert_eq!(bodies, ["a本文", "b"]);
    assert_eq!(
        pasted.cursor,
        PasteCursor {
            item_id: id("n"),
            position: 2
        }
    );
}

// 結合は、項目に入力してまだ保存していない本文があれば、その本文も送る。保存済みの項目の本文は送らない。
#[test]
fn a_merge_posts_the_bodies_that_are_not_saved_yet() {
    let rest = call(ApiOp::MergeItems {
        item_id: id("b"),
        into_id: id("a"),
        body: Some("typed".into()),
        into_body: None,
    });
    assert_eq!(
        rest.request.body,
        Some(json!({ "itemId": "b", "intoId": "a", "body": "typed" }))
    );

    let rest = call(ApiOp::MergeItems {
        item_id: id("b"),
        into_id: id("a"),
        body: None,
        into_body: Some("typed".into()),
    });
    assert_eq!(
        rest.request.body,
        Some(json!({ "itemId": "b", "intoId": "a", "intoBody": "typed" }))
    );
}

// 移動は、動かす項目に入力してまだ保存していない本文があれば、その本文も送る。保存済みなら送らない。
#[test]
fn a_move_posts_the_body_that_is_not_saved_yet() {
    let moved = |body: Option<&str>| {
        call(ApiOp::MoveItems {
            item_ids: vec![id("a"), id("a1")],
            target_page_id: id("p"),
            position: ItemPosition::First,
            level_delta: None,
            body: body.map(str::to_string),
        })
        .request
        .body
    };
    assert_eq!(
        moved(Some("typed")),
        Some(json!({
            "itemIds": ["a", "a1"], "targetPageId": "p", "position": "first",
            "body": "typed",
        }))
    );
    assert_eq!(
        moved(None),
        Some(json!({
            "itemIds": ["a", "a1"], "targetPageId": "p", "position": "first",
        }))
    );
}

// 別の日への移動は、その日のジャーナルの移動の入口へ送る。動かす項目に入力してまだ保存していない本文があれば、その本文も送る。
#[test]
fn a_move_to_another_day_posts_to_the_move_of_that_days_journal_with_the_body_not_saved_yet() {
    let moved = |body: Option<&str>| {
        call(ApiOp::MoveItemsToJournal {
            file_name: "2026-02-11".into(),
            item_ids: vec![id("a"), id("a1")],
            position: ItemPosition::Last,
            body: body.map(str::to_string),
        })
        .request
    };
    let with_body = moved(Some("typed"));
    assert_eq!(with_body.method, "POST");
    assert_eq!(with_body.path, "/api/journals/2026-02-11/move");
    assert_eq!(
        with_body.body,
        Some(json!({ "itemIds": ["a", "a1"], "position": "last", "body": "typed" }))
    );
    assert_eq!(
        moved(None).body,
        Some(json!({ "itemIds": ["a", "a1"], "position": "last" }))
    );
}

// Stash への送付は、その項目の stash の入口へ送る。入力してまだ保存していない本文があれば、その本文も送る。答えからは、見出しと、動いた項目を読む。
#[test]
fn a_stash_posts_to_the_stash_of_the_item_with_the_body_not_saved_yet() {
    let sent = |body: Option<&str>| {
        call(ApiOp::StashItem {
            item_id: id("a"),
            body: body.map(str::to_string),
        })
        .request
    };
    let with_body = sent(Some("typed"));
    assert_eq!(with_body.method, "POST");
    assert_eq!(with_body.path, "/api/items/a/stash");
    assert_eq!(with_body.body, Some(json!({ "body": "typed" })));
    assert_eq!(sent(None).body, Some(json!({})));
    let output = call(ApiOp::StashItem {
        item_id: id("a"),
        body: None,
    })
    .decode(&reply(
        200,
        json!({ "heading": item_json("h", "# Stash"), "created": true, "items": [item_json("a", "todo")] }),
    ))
    .unwrap();
    let ApiOutput::Stashed(stashed) = output else {
        panic!("not a stash: {output:?}");
    };
    assert_eq!(stashed.heading.body, "# Stash");
    assert!(stashed.created);
    assert_eq!(stashed.items[0].id, "a");
}

// トランスクルージョンが見せている項目の分割は、そのトランスクルージョンも送る。
#[test]
fn a_split_of_an_item_a_transclusion_shows_posts_the_transclusion_too() {
    let rest = call(ApiOp::SplitItem {
        item_id: id("a"),
        root_id: Some(id("r")),
        body: "ab".into(),
        cursor: 1,
    });
    assert_eq!(
        rest.request.body,
        Some(json!({ "body": "ab", "cursor": 1, "rootId": "r" }))
    );
}

// 分割は、本文とカーソルを送り、元の項目と作られた項目を答える。
#[test]
fn a_split_posts_the_body_and_the_cursor_and_answers_the_item_and_the_item_made() {
    let rest = call(ApiOp::SplitItem {
        item_id: id("a"),
        root_id: None,
        body: "本文ab".into(),
        cursor: 2,
    });
    assert_eq!(
        (rest.request.method, rest.request.path.as_str()),
        ("POST", "/api/items/a/split")
    );
    assert_eq!(
        rest.request.body,
        Some(json!({ "body": "本文ab", "cursor": 2 }))
    );
    let output = rest
        .decode(&reply(
            200,
            json!({ "item": item_json("a", "本文"), "created": item_json("n", "ab") }),
        ))
        .unwrap();
    let ApiOutput::Split(split) = output else {
        panic!("not a split: {output:?}");
    };
    assert_eq!(
        (split.item.body.as_str(), split.created.body.as_str()),
        ("本文", "ab")
    );
}

// バッチの作成は、トランスクルージョンの元とプロパティを、サーバーが読む名前と形で送る。プロパティの値は、文字列か文字列の並びである。
#[test]
fn a_batch_create_posts_its_transclusion_and_properties_as_the_server_reads_them() {
    let rest = call(ApiOp::ApplyBatch {
        ops: vec![BatchOpInput::Create {
            id: id("n"),
            page_id: id("p"),
            position: ItemPosition::First,
            level: 1,
            body: id("text"),
            transclude_item_id: Some(id("s")),
            properties: [
                ("status".to_string(), PropertyValue::Text(id("done"))),
                (
                    "owners_list".to_string(),
                    PropertyValue::List(vec![id("ann")]),
                ),
            ]
            .into(),
        }],
    });
    assert_eq!(
        rest.request.body,
        Some(json!({ "ops": [
            { "op": "create", "id": "n", "pageId": "p", "position": "first", "level": 1,
              "body": "text", "transcludeItemId": "s",
              "properties": { "owners_list": ["ann"], "status": "done" } },
        ]}))
    );
}

// バッチは、操作をサーバーが読む形の JSON で順に送り、書かれた項目を答える。変えないフィールドは送らない。
#[test]
fn a_batch_posts_its_operations_as_the_json_the_server_reads_and_answers_the_items_written() {
    let rest = call(ApiOp::ApplyBatch {
        ops: vec![
            BatchOpInput::Update {
                id: id("a"),
                body: Some(id("x")),
                level: Some(2),
                collapsed: Some(true),
                task: Some(TaskState {
                    task: true,
                    begin: Some("b".into()),
                    end: None,
                    plan: None,
                }),
            },
            BatchOpInput::Update {
                id: id("s"),
                body: None,
                level: None,
                collapsed: None,
                task: None,
            },
            BatchOpInput::Create {
                id: id("n"),
                page_id: id("p"),
                position: ItemPosition::after("a"),
                level: 2,
                body: id("new"),
                transclude_item_id: None,
                properties: Default::default(),
            },
            BatchOpInput::Move {
                ids: vec![id("m"), id("m1")],
                page_id: id("p"),
                position: ItemPosition::First,
            },
            BatchOpInput::Delete { id: id("d") },
        ],
    });
    assert_eq!(
        (rest.request.method, rest.request.path.as_str()),
        ("POST", "/api/items/batch")
    );
    assert_eq!(
        rest.request.body,
        Some(json!({ "ops": [
            { "op": "update", "id": "a", "body": "x", "level": 2, "collapsed": true,
              "task": { "task": true, "begin": "b" } },
            { "op": "update", "id": "s" },
            { "op": "create", "id": "n", "pageId": "p", "position": "after:a", "level": 2,
              "body": "new" },
            { "op": "move", "ids": ["m", "m1"], "pageId": "p", "position": "first" },
            { "op": "delete", "id": "d" },
        ]}))
    );
    let output = rest
        .decode(&reply(
            200,
            json!({ "items": [item_json("a", "x")], "created": [item_json("e", "")] }),
        ))
        .unwrap();
    let ApiOutput::Batched(batched) = output else {
        panic!("not the answer of a batch: {output:?}");
    };
    assert_eq!(batched.items[0].body, "x");
    assert_eq!(batched.created[0].id, "e");
}

// 貼り付けの読み方は、サーバーが読む名前で送られる。
#[test]
fn the_mode_of_a_paste_goes_under_the_name_the_server_reads() {
    let name_of = |mode| {
        let rest = call(ApiOp::PasteText {
            item_id: id("a"),
            body: String::new(),
            cursor: 0,
            text: String::new(),
            mode,
        });
        rest.request.body.unwrap()["mode"].clone()
    };
    assert_eq!(name_of(PasteMode::Auto), "auto");
    assert_eq!(name_of(PasteMode::Flat), "flat");
    assert_eq!(name_of(PasteMode::Raw), "raw");
}

// 更新は、変えるフィールドだけを送る。
#[test]
fn an_update_sends_only_the_fields_that_change() {
    let request = call(ApiOp::UpdateItem {
        item_id: id("a"),
        body: Some(id("x")),
        level: None,
        collapsed: None,
    })
    .request;
    assert_eq!(
        (request.method, request.path.as_str()),
        ("PUT", "/api/items/a")
    );
    assert_eq!(request.body, Some(json!({ "body": "x" })));
}

// 設定の更新も、変えるフィールドだけを送る。
#[test]
fn a_config_update_sends_only_the_fields_that_change() {
    let request = call(ApiOp::UpdateConfig {
        markdown_dir: None,
        week_numbering: Some(id("iso")),
        logical_outdent: None,
        tags_dir: None,
        home_link: None,
    })
    .request;
    assert_eq!(request.body, Some(json!({ "weekNumbering": "iso" })));
}

// ページの更新は、上書きするときだけoverwriteを送る。
#[test]
fn a_page_update_asks_to_overwrite_only_when_told() {
    let body = |overwrite| {
        call(ApiOp::UpdatePage {
            page_id: id("p"),
            title: Some(id("t")),
            dir_path: None,
            overwrite,
        })
        .request
        .body
    };
    assert_eq!(body(false), Some(json!({ "title": "t" })));
    assert_eq!(body(true), Some(json!({ "title": "t", "overwrite": true })));
}

// 降格も、指示されたときだけ上書きを求める。名前を別のページが持っていれば、エラーではなく答えになる。
#[test]
fn a_demotion_asks_to_overwrite_only_when_told_and_a_taken_name_is_an_answer() {
    let demote = |overwrite| {
        call(ApiOp::DemotePage {
            page_id: id("p"),
            overwrite,
        })
    };
    assert_eq!(demote(false).request.body, Some(json!({})));
    assert_eq!(
        demote(true).request.body,
        Some(json!({ "overwrite": true }))
    );
    assert_eq!(
        demote(false).decode(&reply(409, json!({ "error": "taken: x" }))),
        Ok(ApiOutput::PageNameTaken("taken: x".into()))
    );
}

// 検索語はエンコードされ、スペースや日本語を含んでもURLが壊れない。
#[test]
fn a_search_query_is_encoded_into_the_url() {
    let request = call(ApiOp::SearchCombined {
        query: "a b&日".into(),
        max_results: 20,
    })
    .request;
    assert_eq!(
        request.path,
        "/api/search/combined?q=a%20b%26%E6%97%A5&maxResults=20"
    );
}

// エンコードはencodeURIComponentと同じで、記号の一部はそのまま残る。
#[test]
fn the_encoding_leaves_the_same_marks_as_encode_uri_component() {
    let request = call(ApiOp::GetOrCreateJournal {
        file_name: "-_.!~*'()/?#".into(),
    })
    .request;
    assert_eq!(request.path, "/api/journals/-_.!~*'()%2F%3F%23");
}

// プロパティの名前もエンコードされる。
#[test]
fn a_property_key_is_encoded_into_the_path() {
    let request = call(ApiOp::DeleteItemProperty {
        item_id: id("a"),
        key: "a/b".into(),
    })
    .request;
    assert_eq!(request.path, "/api/items/a/properties/a%2Fb");
}

// 答えは、操作ごとの形からコアの型になる。
#[test]
fn every_answer_becomes_the_output_of_its_operation() {
    let page = Page::default();
    let cases = vec![
        (
            ApiOp::GetPageById { page_id: id("p") },
            json_of(<PageWithItems>::default()),
            ApiOutput::Page(<PageWithItems>::default()),
        ),
        (
            ApiOp::GetOrCreateDoc {
                title: "t".into(),
                dir_path: String::new(),
            },
            json!({ "page": json_of(page.clone()), "items": [], "created": true }),
            ApiOutput::DocPage(DocPage {
                page: page.clone(),
                items: vec![],
                created: true,
            }),
        ),
        (
            ApiOp::MoveItemsToJournal {
                file_name: "2026-02-11".into(),
                item_ids: vec![],
                position: ItemPosition::Last,
                body: None,
            },
            json!({ "page": json_of(page.clone()), "created": true, "items": [] }),
            ApiOutput::MovedToJournal(MovedToJournal {
                page: page.clone(),
                created: true,
                items: vec![],
            }),
        ),
        (
            ApiOp::GetMostRecentJournalBefore { before: id("d") },
            json_of(<JournalBefore>::default()),
            ApiOutput::JournalBefore(<JournalBefore>::default()),
        ),
        (
            ApiOp::DeleteItems { item_ids: vec![] },
            json!({ "created": [] }),
            ApiOutput::Items(vec![]),
        ),
        (
            ApiOp::ShiftItemLevels {
                item_ids: vec![],
                delta: 1,
            },
            json!({ "items": [] }),
            ApiOutput::Items(vec![]),
        ),
        (
            ApiOp::GetChildPages { parent_id: id("p") },
            json!({ "pages": [json_of(&page)] }),
            ApiOutput::Pages(vec![page.clone()]),
        ),
        (
            ApiOp::DeletePageProperty {
                page_id: id("p"),
                key: id("k"),
            },
            json!({ "page": json_of(&page) }),
            ApiOutput::PageInfo(page.clone()),
        ),
        (
            ApiOp::GetBacklinks { page_id: id("p") },
            json!({ "entries": [] }),
            ApiOutput::Backlinks(vec![]),
        ),
        (
            ApiOp::SearchPageNames {
                query: id("q"),
                max_results: 1,
            },
            json!({ "results": [] }),
            ApiOutput::SearchResults(vec![]),
        ),
        (
            ApiOp::GetConfig,
            json_of(ServerConfig::default()),
            ApiOutput::Config(ServerConfig::default()),
        ),
        (
            ApiOp::GetPropertyDefinitions,
            json!({ "definitions": [] }),
            ApiOutput::PropertyDefinitions(vec![]),
        ),
        (
            ApiOp::BrowseFolder,
            json!({ "selected": true, "path": "f" }),
            ApiOutput::Folder(FolderChoice {
                selected: true,
                path: id("f"),
            }),
        ),
        (
            ApiOp::ExportTreeMarkdown { item_ids: vec![] },
            json_of(TreeExport::default()),
            ApiOutput::TreeExport(TreeExport::default()),
        ),
        (
            ApiOp::TranscludedItems { item_ids: vec![] },
            json!({ "itemIds": ["a"] }),
            ApiOutput::ItemIds(vec![id("a")]),
        ),
        (
            ApiOp::LaunchUrl { url: id("x://y") },
            json!({ "success": true }),
            ApiOutput::Launched(true),
        ),
    ];
    for (op, body, expected) in cases {
        assert_eq!(answer(op.clone(), body), expected, "{op:?}");
    }
}

// 本文のない成功は、Doneを答える。
#[test]
fn a_success_without_a_body_answers_done() {
    let output = call(ApiOp::DeletePage { page_id: id("p") }).decode(&HttpReply {
        status: 204,
        body: String::new(),
    });
    assert_eq!(output, Ok(ApiOutput::Done));
}

// サーバーのエラーは、そのメッセージをcoreへ渡す。
#[test]
fn a_server_error_carries_its_message() {
    let result = call(ApiOp::DeletePage { page_id: id("p") })
        .decode(&reply(400, json!({ "error": "Level must be 1" })));
    assert_eq!(result, Err(ApiError("Level must be 1".into())));
}

// メッセージのないエラーは、HTTPの状態を伝える。
#[test]
fn an_error_without_a_message_names_the_status() {
    let result = call(ApiOp::DeletePage { page_id: id("p") }).decode(&HttpReply {
        status: 500,
        body: "oops".into(),
    });
    assert_eq!(result, Err(ApiError("HTTP 500".into())));
}

// 成功でも読めない答えは、エラーになる。
#[test]
fn an_answer_that_cannot_be_read_is_an_error() {
    let result = call(ApiOp::GetConfig).decode(&HttpReply {
        status: 200,
        body: "not json".into(),
    });
    assert!(result.is_err());
}

// 名前の重複（409）はエラーではなく、coreが利用者に尋ねるための答えになる。
#[test]
fn a_taken_page_name_is_an_answer_not_an_error() {
    let output = call(ApiOp::UpdatePage {
        page_id: id("p"),
        title: Some(id("x")),
        dir_path: None,
        overwrite: false,
    })
    .decode(&reply(409, json!({ "error": "taken: x" })));
    assert_eq!(output, Ok(ApiOutput::PageNameTaken("taken: x".into())));
}

// 名前が重複しなければ、更新されたページを答える。
#[test]
fn a_page_update_answers_the_page() {
    let page = Page::default();
    let output = answer(
        ApiOp::UpdatePage {
            page_id: id("p"),
            title: Some(id("x")),
            dir_path: None,
            overwrite: false,
        },
        json!({ "page": json_of(&page) }),
    );
    assert_eq!(output, ApiOutput::PageInfo(page));
}

// 他の操作の409は、ただのエラーである。
#[test]
fn a_conflict_on_another_operation_is_an_error() {
    let result = call(ApiOp::DeletePage { page_id: id("p") })
        .decode(&reply(409, json!({ "error": "taken: x" })));
    assert_eq!(result, Err(ApiError("taken: x".into())));
}

// アップロードの本体はシェルだけが持っているので、RESTの呼び出しにならない。
#[test]
fn an_upload_is_left_to_the_shell() {
    assert!(rest_call(ApiOp::UploadAsset { file: 1 }).is_none());
}
