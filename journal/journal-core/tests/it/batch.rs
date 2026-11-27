//! Several changes of items applied in one transaction, as an undo replays them.

use journal_core::error::AppError;
use journal_core::journal::BatchAnswer;
use serde_json::{json, Value};

use crate::common::items::{answer, field_of_each, has_type, id, input, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Applies operations as one batch, the way an undo sends what it replays
    pub fn batch(&self, ops: Value) -> Result<BatchAnswer, AppError> {
        self.core
            .journal
            .apply_batch(input(json!({ "ops": ops })), &by_the_client())
    }

    /// Applies a batch as `batch` does, and gives the answer as the JSON a client gets
    fn batched(&self, ops: Value) -> Value {
        answer(self.batch(ops).unwrap())
    }
}

fn is_bad_request(batch: &Result<BatchAnswer, AppError>) -> bool {
    matches!(batch, Err(AppError::BadRequest(_)))
}

// 更新の操作は、本文、階層、折りたたみを書き、書いた項目を答える。
#[test]
fn a_batch_update_writes_the_body_the_level_and_the_collapsed_state() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-update");
    let rows = notes.fill(&page, &["top", "old"]);

    let batch = notes.batched(json!([
        { "op": "update", "id": id(&rows[1]), "body": "new", "level": 2, "collapsed": true },
    ]));

    assert_eq!(field_of_each(&batch, "items", "body"), vec![json!("new")]);
    let stored = notes.items(&page);
    assert_eq!(notes.outline_of(&page), vs(&["1:top", "2:new"]));
    assert_eq!(stored[1]["collapsed"], json!(true));
}

// バッチがページの項目をすべて消すと、同じ要求で空の項目が1つ作られ、答えの`created`に入る。
#[test]
fn a_batch_that_empties_the_page_makes_one_empty_item_and_answers_it() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-empties");
    let rows = notes.fill(&page, &["only"]);

    let batch = notes.batched(json!([{ "op": "delete", "id": id(&rows[0]) }]));

    let left = notes.items(&page);
    assert_eq!(notes.outline_of(&page), vs(&["1:"]));
    assert_eq!(
        field_of_each(&batch, "created", "id"),
        vec![json!(id(&left[0]))]
    );
    assert_eq!(batch["items"], json!([]));
}

// 作成の操作は、名指しされたIDの項目を、指定の位置と階層に作る。削除した項目を、同じIDで作り直せる。
#[test]
fn a_batch_create_makes_the_item_again_with_the_id_it_names() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-create");
    let rows = notes.fill(&page, &["a", "  gone", "c"]);
    notes.delete_item(&id(&rows[1]));

    let batch = notes.batched(json!([{
        "op": "create", "id": id(&rows[1]), "pageId": page,
        "position": format!("after:{}", id(&rows[0])), "level": 2, "body": "back",
    }]));

    assert_eq!(
        field_of_each(&batch, "items", "id"),
        vec![json!(id(&rows[1]))]
    );
    let stored = notes.items(&page);
    assert_eq!(notes.outline_of(&page), vs(&["1:a", "2:back", "1:c"]));
    assert_eq!(id(&stored[1]), id(&rows[1]));
}

// 削除したタスクを、タスクでない項目として同じIDで作り直すと、タスクの種類は残らない。削除のUndoが、消す前の種類だけを戻すためである。
#[test]
fn an_item_made_again_with_its_id_does_not_keep_the_types_it_had_before() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-types");
    let rows = notes.fill(&page, &["a", "task"]);
    notes.batched(json!([{"op": "update", "id": id(&rows[1]), "task": {"task": true}}]));
    assert!(has_type(&notes.item(&id(&rows[1])), "task"));
    notes.delete_item(&id(&rows[1]));

    notes.batched(json!([{
        "op": "create", "id": id(&rows[1]), "pageId": page,
        "position": format!("after:{}", id(&rows[0])), "level": 1, "body": "plain",
    }]));

    assert!(!has_type(&notes.item(&id(&rows[1])), "task"));
}

// 作成の操作は、写す元の項目を名指しすると、トランスクルージョンとして作り直す。削除のUndoが、トランスクルージョンを戻すためである。
#[test]
fn a_batch_create_makes_a_transclusion_again_with_the_source_it_names() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("items-batch-source");
    let source = notes.fill(&source_page, &["source"]);
    let page = notes.new_doc_page("items-batch-transcluder");
    let rows = notes.fill(&page, &["a", "shown"]);
    notes.transclude(&id(&rows[1]), &id(&source[0]));
    notes.delete_item(&id(&rows[1]));

    notes.batched(json!([{
        "op": "create", "id": id(&rows[1]), "pageId": page,
        "position": format!("after:{}", id(&rows[0])), "level": 1, "body": "source",
        "transcludeItemId": id(&source[0]),
    }]));

    let back = notes.item(&id(&rows[1]));
    assert_eq!(back["transcludeItemId"], json!(id(&source[0])));
    assert_eq!(back["body"], json!("source"));
}

// 作成の操作は、同じバッチで後に作り直す項目も、写す元にできる。元の項目とそれを写す項目を一緒に消した削除のUndoが、断られないためである。
#[test]
fn a_batch_create_can_transclude_an_item_made_again_later_in_the_batch() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-both");
    let rows = notes.fill(&page, &["a", "shown", "source"]);
    notes.transclude(&id(&rows[1]), &id(&rows[2]));
    notes.batched(json!([
        { "op": "delete", "id": id(&rows[1]) },
        { "op": "delete", "id": id(&rows[2]) },
    ]));

    notes.batched(json!([
        { "op": "create", "id": id(&rows[1]), "pageId": page,
          "position": format!("after:{}", id(&rows[0])), "level": 1, "body": "source",
          "transcludeItemId": id(&rows[2]) },
        { "op": "create", "id": id(&rows[2]), "pageId": page,
          "position": format!("after:{}", id(&rows[1])), "level": 1, "body": "source" },
    ]));

    assert_eq!(
        notes.item(&id(&rows[1]))["transcludeItemId"],
        json!(id(&rows[2]))
    );
}

// 作成の操作は、写す元の項目がもうないときは、写さずに本文だけの項目として作り直す。Undoが断られて、項目が戻らなくなるよりよいためである。
#[test]
fn a_batch_create_whose_source_is_gone_makes_the_item_without_the_transclusion() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-source-gone");
    let rows = notes.fill(&page, &["a", "shown", "source"]);
    notes.transclude(&id(&rows[1]), &id(&rows[2]));
    notes.delete_item(&id(&rows[1]));
    notes.delete_item(&id(&rows[2]));

    notes.batched(json!([{
        "op": "create", "id": id(&rows[1]), "pageId": page,
        "position": format!("after:{}", id(&rows[0])), "level": 1, "body": "source",
        "transcludeItemId": id(&rows[2]),
    }]));

    let back = notes.item(&id(&rows[1]));
    assert_eq!(back["transcludeItemId"], Value::Null);
    assert_eq!(back["body"], json!("source"));
}

// 作成の操作は、プロパティを渡すと、その値も書く。1つの値のキーには文字列、リストのキーには文字列の並びを書く。削除のUndoが、プロパティを戻すためである。
#[test]
fn a_batch_create_writes_the_properties_it_is_given() {
    let notes = Core::new();
    notes.define_key("status");
    notes.define_key("owners_list");
    let page = notes.new_doc_page("items-batch-properties");
    let rows = notes.fill(&page, &["a", "gone"]);
    notes.delete_item(&id(&rows[1]));

    notes.batched(json!([{
        "op": "create", "id": id(&rows[1]), "pageId": page,
        "position": format!("after:{}", id(&rows[0])), "level": 1, "body": "back",
        "properties": { "status": "done", "owners_list": ["ann", "bob"] },
    }]));

    let back = notes.item(&id(&rows[1]));
    assert_eq!(back["properties"]["status"], json!("done"));
    assert_eq!(back["properties"]["owners_list"], json!(["ann", "bob"]));
}

// 更新の操作は、渡したプロパティを書き、名前を挙げたプロパティを消す。ほかのプロパティは変わらない。アトラスの1回の操作を、1回の送信にまとめるためである。
#[test]
fn a_batch_update_writes_and_drops_the_properties_it_names() {
    let notes = Core::new();
    notes.define_key("status");
    notes.define_key("owners_list");
    let page = notes.new_doc_page("items-batch-update-properties");
    let rows = notes.fill(&page, &["a"]);
    let item = id(&rows[0]);

    notes.batched(json!([{
        "op": "update", "id": item,
        "properties": { "status": "open", "owners_list": ["ann", "bob"], "atlas_side": "left" },
    }]));
    notes.batched(json!([{
        "op": "update", "id": item,
        "properties": { "status": "done" },
        "droppedProperties": ["atlas_side"],
    }]));

    let written = notes.item(&item)["properties"].clone();
    assert_eq!(
        written,
        json!({ "status": "done", "owners_list": ["ann", "bob"] })
    );
}

// 更新の操作が、定義のないキーや、型に合わない値を渡すと、断られて何も書かれない。
#[test]
fn a_batch_update_with_a_property_that_does_not_fit_is_refused_and_writes_nothing() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-update-bad-properties");
    let rows = notes.fill(&page, &["a"]);
    let item = id(&rows[0]);

    let no_definition = notes.batch(json!([{
        "op": "update", "id": item, "body": "changed", "properties": { "no_such_key": "x" },
    }]));
    let not_bounds = notes.batch(json!([{
        "op": "update", "id": item, "body": "changed", "properties": { "atlas_bounds": "x" },
    }]));

    assert!(is_bad_request(&no_definition), "{no_definition:?}");
    assert!(is_bad_request(&not_bounds), "{not_bounds:?}");
    assert_eq!(notes.item(&item)["body"], json!("a"));
}

// 作り直した項目は、もう「ここで消した項目」としては覚えられていない。Markdownの取り込みが、その項目を消されたものとして扱わないためである。
#[test]
fn an_item_made_again_is_no_longer_remembered_as_deleted() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-undelete");
    let rows = notes.fill(&page, &["a", "gone"]);
    notes.delete_item(&id(&rows[1]));
    let deleted_dates = || {
        let conn = notes.core.db.conn().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM deleted_items WHERE id = ?",
            [&id(&rows[1])],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
    };
    assert_eq!(deleted_dates(), 1);

    notes.batched(json!([{
        "op": "create", "id": id(&rows[1]), "pageId": page,
        "position": format!("after:{}", id(&rows[0])), "level": 1, "body": "back",
    }]));

    assert_eq!(deleted_dates(), 0);
}

// 削除の操作は、まとめて1つのグループとして消される。親が子より先に並んでいてもよい。消した項目は答えに入らない。
#[test]
fn the_deletes_of_a_batch_go_as_one_group() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-delete");
    let rows = notes.fill(&page, &["keep", "parent", "  child"]);

    let batch = notes.batched(json!([
        { "op": "delete", "id": id(&rows[1]) },
        { "op": "delete", "id": id(&rows[2]) },
    ]));

    assert_eq!(batch["items"], json!([]));
    // An item is left on the page, so no empty item is made
    assert_eq!(batch["created"], json!([]));
    assert_eq!(notes.outline_of(&page), vs(&["1:keep"]));
}

// 更新の操作は、タスクの状態も、時刻と予定日ごと書ける。タスクの切り替えの undo が、本文と同じ操作で戻すためである。
#[test]
fn a_batch_update_writes_the_task_state_with_its_times() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-task");
    let item = id(&notes.fill(&page, &["buy milk"])[0]);

    let batch = notes.batched(json!([{
        "op": "update", "id": item, "body": "buy oat milk",
        "task": {
            "task": true,
            "begin": "2026-01-02T03:04:05",
            "end": "2026-01-02T04:05:06",
            "plan": "2026-01-02",
        },
    }]));

    let answered = batch["items"][0].clone();
    let stored = notes.items(&page).remove(0);
    for row in [answered, stored] {
        assert_eq!(row["body"], json!("buy oat milk"));
        assert!(has_type(&row, "task"));
        assert_eq!(row["properties"]["begin"], json!("2026-01-02T03:04:05"));
        assert_eq!(row["properties"]["end"], json!("2026-01-02T04:05:06"));
        assert_eq!(row["properties"]["plan"], json!("2026-01-02"));
    }
}

// 更新の操作は、指定されなかった時刻と予定日を消し、項目を通常の項目に戻せる。
#[test]
fn a_batch_update_removes_what_the_task_state_does_not_give() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-task-unset");
    let item = id(&notes.fill(&page, &["buy milk"])[0]);
    let full = json!({ "task": true, "begin": "2026-01-02T03:04:05Z", "end": "2026-01-02T04:05:06Z", "plan": "2026-01-02" });
    notes.batched(json!([{"op": "update", "id": item, "task": full}]));

    notes.batched(json!([{"op": "update", "id": item, "task": {"task": false}}]));

    let row = notes.items(&page).remove(0);
    assert!(!has_type(&row, "task"));
    for key in ["begin", "end", "plan"] {
        assert!(row["properties"].get(key).is_none(), "{key} is still there");
    }
}

// 通常の項目に時刻を付ける更新は断られ、同じバッチのほかの操作も書かれない。cycle-taskも、そのような状態を作らない。
#[test]
fn a_batch_that_gives_times_to_a_normal_item_is_refused_and_writes_nothing() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-task-refused");
    let rows = notes.fill(&page, &["first", "second"]);

    for times in [json!({ "begin": "b" }), json!({ "end": "e" })] {
        let batch = notes.batch(json!([
            {"op": "update", "id": id(&rows[0]), "body": "changed"},
            {"op": "update", "id": id(&rows[1]), "task": times},
        ]));
        assert!(is_bad_request(&batch), "{batch:?}");
    }

    let stored = notes.items(&page);
    assert_eq!(stored[0]["body"], json!("first"));
    assert_eq!(stored[1]["properties"], json!({}));
}

// 操作は並んだ順に行われる。先に作った項目を、後の操作が位置の基準にも更新の相手にもできる。
#[test]
fn the_operations_of_a_batch_are_applied_in_their_order() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-order");
    let rows = notes.fill(&page, &["a", "d"]);
    let ids = hmtid::Generator::new('_', false);
    let (b, c) = (ids.generate().to_string(), ids.generate().to_string());

    notes.batched(json!([
        {"op": "create", "id": b, "pageId": page,
         "position": format!("after:{}", id(&rows[0])), "level": 1, "body": "b"},
        {"op": "create", "id": c, "pageId": page,
         "position": format!("after:{b}"), "level": 2, "body": "c"},
        {"op": "update", "id": b, "body": "B"},
    ]));

    assert_eq!(notes.outline_of(&page), vs(&["1:a", "1:B", "2:c", "1:d"]));
}

// 同じ項目を2回書くバッチは、その項目を最後の姿で1回だけ答える。
#[test]
fn an_item_written_twice_in_a_batch_is_answered_once_as_it_is_at_the_end() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-twice");
    let item = id(&notes.fill(&page, &["old"])[0]);

    let batch = notes.batched(json!([
        {"op": "update", "id": item, "body": "middle"},
        {"op": "update", "id": item, "body": "new"},
    ]));

    assert_eq!(field_of_each(&batch, "items", "body"), vec![json!("new")]);
}

// ない項目を名指しする操作があるバッチは断られ、ほかの操作も書かれない。
#[test]
fn a_batch_that_names_an_item_that_is_gone_is_refused_and_writes_nothing() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-missing");
    let rows = notes.fill(&page, &["stays", "gone"]);
    notes.delete_item(&id(&rows[1]));

    let updated = notes.batch(json!([
        {"op": "update", "id": id(&rows[0]), "body": "changed"},
        {"op": "update", "id": id(&rows[1]), "body": "back"},
    ]));
    assert!(is_bad_request(&updated), "{updated:?}");
    let deleted = notes.batch(json!([
        {"op": "update", "id": id(&rows[0]), "body": "changed"},
        {"op": "delete", "id": id(&rows[1])},
    ]));
    assert!(matches!(deleted, Err(AppError::NotFound(_))), "{deleted:?}");

    assert_eq!(notes.outline_of(&page), vs(&["1:stays"]));
}

// 置けない階層への作成や更新があるバッチは断られ、ほかの操作も書かれない。
#[test]
fn a_batch_with_a_level_that_does_not_fit_is_refused_and_writes_nothing() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-level");
    let rows = notes.fill(&page, &["a", "b"]);
    let made_up = hmtid::Generator::new('_', false).generate().to_string();

    let created = notes.batch(json!([
        {"op": "update", "id": id(&rows[0]), "body": "changed"},
        {"op": "create", "id": made_up, "pageId": page,
         "position": format!("after:{}", id(&rows[0])), "level": 3, "body": "too deep"},
    ]));
    assert!(is_bad_request(&created), "{created:?}");
    let updated = notes.batch(json!([
        {"op": "update", "id": id(&rows[0]), "body": "changed"},
        {"op": "update", "id": id(&rows[1]), "level": 0},
    ]));
    assert!(is_bad_request(&updated), "{updated:?}");

    assert_eq!(notes.outline_of(&page), vs(&["1:a", "1:b"]));
}

// 子孫を残す削除があるバッチは断られ、ほかの操作も書かれない。
#[test]
fn a_batch_whose_deletes_leave_a_descendant_behind_is_refused_and_writes_nothing() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-orphan");
    let rows = notes.fill(&page, &["parent", "  child", "other"]);

    let batch = notes.batch(json!([
        {"op": "update", "id": id(&rows[2]), "body": "changed"},
        {"op": "delete", "id": id(&rows[0])},
    ]));
    assert!(is_bad_request(&batch), "{batch:?}");

    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:parent", "2:child", "1:other"])
    );
}

// バッチがいくつ項目を書いても、ページの変更は、ページごとに1回だけクライアントに知らされる。
#[test]
fn the_clients_hear_of_each_page_of_a_batch_once() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-event");
    let rows = notes.fill(&page, &["a", "b"]);
    let second_page = notes.new_doc_page("items-batch-event-second");
    let second = id(&notes.fill(&second_page, &["c"])[0]);

    let told = notes.times_told(&[&page, &second_page], || {
        notes.batched(json!([
            {"op": "update", "id": id(&rows[0]), "body": "A"},
            {"op": "update", "id": second, "body": "C"},
            {"op": "delete", "id": id(&rows[1])},
        ]));
    });

    assert_eq!(told, [1, 1]);
}

// トランスクルードされている項目を更新すると、本文が変わらなくても、その写しを見せているページの変更が知らされる。
#[test]
fn the_clients_hear_of_the_pages_that_transclude_an_item_a_batch_updates() {
    let notes = Core::new();
    let (_, source, page, _) = notes.transcluded_rows("items-batch-transcluder", &["buy milk"]);

    let told = notes.times_told(&[&page], || {
        notes.batched(json!([
            {"op": "update", "id": id(&source[0]), "task": {"task": true}},
        ]));
    });

    assert_ne!(told, [0]);
}

// 移動の操作は、項目のまとまりを、指定の位置へ動かし、動かした項目を答える。移動の undo が位置を戻すためである。
#[test]
fn a_batch_move_puts_a_group_where_its_position_says() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-move");
    let rows = notes.fill(&page, &["a", "  a1", "b", "  b1"]);

    let batch = notes.batched(json!([{
        "op": "move", "ids": [id(&rows[2]), id(&rows[3])], "pageId": page, "position": "first",
    }]));
    assert_eq!(
        field_of_each(&batch, "items", "id"),
        vec![json!(id(&rows[2])), json!(id(&rows[3]))]
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:b", "2:b1", "1:a", "2:a1"]));

    notes.batched(json!([{
        "op": "move", "ids": [id(&rows[2]), id(&rows[3])], "pageId": page,
        "position": format!("after:{}", id(&rows[1])),
    }]));
    assert_eq!(notes.outline_of(&page), vs(&["1:a", "2:a1", "1:b", "2:b1"]));
}

// 階層の更新と移動を1つのバッチで行える。論理アウトデントの undo が、階層と位置を一緒に戻すためである。
#[test]
fn a_batch_updates_a_level_and_then_moves_the_item() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-level-move");
    let rows = notes.fill(&page, &["parent", "  first", "  last", "moved"]);

    let batch = notes.batched(json!([
        {"op": "update", "id": id(&rows[3]), "level": 2},
        {"op": "move", "ids": [id(&rows[3])], "pageId": page,
         "position": format!("after:{}", id(&rows[1]))},
    ]));

    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:parent", "2:first", "2:moved", "2:last"])
    );
    assert_eq!(
        field_of_each(&batch, "items", "id"),
        vec![json!(id(&rows[3]))]
    );
}

// 置けない位置への移動があるバッチは断られ、ほかの操作も書かれない。
#[test]
fn a_batch_with_a_move_that_does_not_fit_is_refused_and_writes_nothing() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-batch-move-refused");
    let rows = notes.fill(&page, &["a", "  a1", "b"]);

    // A child cannot be the first item of a page
    let first = notes.batch(json!([
        {"op": "update", "id": id(&rows[2]), "body": "changed"},
        {"op": "move", "ids": [id(&rows[1])], "pageId": page, "position": "first"},
    ]));
    assert!(is_bad_request(&first), "{first:?}");
    // An item cannot go after itself
    let after_itself = notes.batch(json!([
        {"op": "update", "id": id(&rows[2]), "body": "changed"},
        {"op": "move", "ids": [id(&rows[2])], "pageId": page,
         "position": format!("after:{}", id(&rows[2]))},
    ]));
    assert!(is_bad_request(&after_itself), "{after_itself:?}");

    assert_eq!(notes.outline_of(&page), vs(&["1:a", "2:a1", "1:b"]));
}
