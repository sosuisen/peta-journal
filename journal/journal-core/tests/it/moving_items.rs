//! Moving items: their levels shifted in place, and the items moved on their page or to
//! another, with the body their editor has now.

use journal_core::error::AppError;
use journal_core::journal::ItemsAnswer;
use serde_json::{json, Value};

use crate::common::items::{id, input, outline, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Moves items the way a key in an editor does: with the body the editor of the first
    /// item has now
    fn move_edited(
        &self,
        item_ids: &[&str],
        page_id: &str,
        position: &str,
        body: &str,
    ) -> Result<ItemsAnswer, AppError> {
        self.move_with(json!({ "itemIds": item_ids, "targetPageId": page_id, "position": position, "body": body }))
    }

    fn move_with(&self, request: Value) -> Result<ItemsAnswer, AppError> {
        self.core
            .journal
            .move_items(input(request), &by_the_client())
    }

    fn shift(&self, item_ids: &[&str], delta: i32) {
        self.core
            .journal
            .shift_levels(
                input(json!({ "itemIds": item_ids, "delta": delta })),
                &by_the_client(),
            )
            .unwrap();
    }
}

/// The bodies of the items of the answer
fn bodies(moved: ItemsAnswer) -> Vec<Value> {
    moved
        .items
        .into_iter()
        .map(|item| json!(item.body))
        .collect()
}

// ---- levels ----

// shift-levelは、複数の項目のレベルを相対量だけ動かす。
#[test]
fn shift_level_moves_a_group_by_a_relative_amount() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-shift");
    let rows = notes.fill(&page, &["a", "b", "  b1", "c"]);

    notes.shift(&[&id(&rows[1]), &id(&rows[2])], 1);

    assert_eq!(notes.outline_of(&page), vs(&["1:a", "2:b", "3:b1", "1:c"]));
}

// shift-levelは項目をいくつも変えるが、ページの変更は1回だけクライアントに知らされる。
#[test]
fn the_clients_hear_of_the_page_once_when_a_group_is_shifted() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-shift-event");
    let rows = notes.fill(&page, &["a", "b", "c"]);

    let told = notes.times_told(&[&page], || {
        notes.shift(&[&id(&rows[1]), &id(&rows[2])], 1);
    });

    assert_eq!(told, [1]);
}

// ---- moving ----

// 部分木は、同じページの兄弟を越えて移動する。
#[test]
fn a_subtree_moves_past_its_sibling_on_the_same_page() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-move");
    let rows = notes.fill(&page, &["a", "b", "  b1", "c"]);
    let subtree = [id(&rows[1]), id(&rows[2])];
    let subtree = [subtree[0].as_str(), subtree[1].as_str()];

    notes.move_items(&subtree, &page, &format!("after:{}", id(&rows[3])), None);
    assert_eq!(notes.outline_of(&page), vs(&["1:a", "1:c", "1:b", "2:b1"]));

    notes.move_items(&subtree, &page, "first", None);
    assert_eq!(notes.outline_of(&page), vs(&["1:b", "2:b1", "1:a", "1:c"]));
}

// 項目は、レベルをずらして別のページへ移動する。
#[test]
fn items_move_to_another_page_with_their_levels_shifted() {
    let notes = Core::new();
    let from = notes.new_doc_page("items-move-from");
    let to = notes.new_doc_page("items-move-to");
    let rows = notes.fill(&from, &["stay", "go", "  go-child"]);
    let target = notes.fill(&to, &["parent"]).remove(0);

    let moved = notes.move_items(
        &[&id(&rows[1]), &id(&rows[2])],
        &to,
        &format!("after:{}", id(&target)),
        Some(1),
    );

    assert_eq!(outline(&moved), vs(&["2:go", "3:go-child"]));
    assert_eq!(notes.outline_of(&from), vs(&["1:stay"]));
    assert_eq!(
        notes.outline_of(&to),
        vs(&["1:parent", "2:go", "3:go-child"])
    );
}

// 最後の項目が別のページへ移動したページは、空の項目を1つ持つ。ページは項目を1つは持つ（ADR 021）。
#[test]
fn the_page_the_last_items_leave_keeps_one_empty_item() {
    let notes = Core::new();
    let from = notes.new_doc_page("last-items-leave-from");
    let to = notes.new_doc_page("last-items-leave-to");
    let rows = notes.fill(&from, &["go", "  go-child"]);
    notes.fill(&to, &["there"]);

    notes.move_items(&[&id(&rows[0]), &id(&rows[1])], &to, "first", None);

    assert_eq!(notes.outline_of(&from), vs(&["1:"]));
    assert_eq!(
        notes.outline_of(&to),
        vs(&["1:go", "2:go-child", "1:there"])
    );
}

// 移動の要求が本文を持っていれば、先頭の項目はその本文で保存されて移動する。エディタに打って、まだ保存していない文である。
#[test]
fn a_move_saves_the_body_it_carries_to_the_first_item() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-move-unsaved");
    let rows = notes.fill(&page, &["a", "b", "  b1", "c"]);

    let moved = notes
        .move_edited(&[&id(&rows[1]), &id(&rows[2])], &page, "first", "b typed")
        .unwrap();

    assert_eq!(bodies(moved), vec![json!("b typed"), json!("b1")]);
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:b typed", "2:b1", "1:a", "1:c"])
    );
}

// 断られた移動は、持ってきた本文も保存しない。移動と保存は、1つのトランザクションである。
#[test]
fn a_move_that_is_refused_does_not_save_the_body_it_carries() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-move-unsaved-refused");
    let rows = notes.fill(&page, &["a", "b"]);
    let other = notes.new_doc_page("items-move-unsaved-other");
    let elsewhere = notes.fill(&other, &["elsewhere"]).remove(0);

    let not_on_the_page = format!("after:{}", id(&elsewhere));
    let moved = notes.move_edited(&[&id(&rows[1])], &page, &not_on_the_page, "typed");

    assert!(matches!(moved, Err(AppError::BadRequest(_))), "{moved:?}");
    assert_eq!(notes.outline_of(&page), vs(&["1:a", "1:b"]));
}

// トランスクルージョンを動かす要求の本文は、元の項目に保存される。トランスクルージョンに打った文は、元の項目の文である。
#[test]
fn the_body_a_move_of_a_transclusion_carries_is_saved_to_its_source_item() {
    let notes = Core::new();
    let (source_page, _, page, own) =
        notes.transclusion("items-move-copy", &["shown"], 0, &["top", "copy"], 1);

    let moved = notes
        .move_edited(&[&id(&own[1])], &page, "first", "shown typed")
        .unwrap();

    assert_eq!(notes.outline_of(&source_page), vs(&["1:shown typed"]));
    assert_eq!(notes.outline_of(&page), vs(&["1:shown typed", "1:top"]));
    assert_eq!(bodies(moved), vec![json!("shown typed")]);
}

// 移動の要求の本文が閉じていないブロックで終われば、移動は断られ、項目は動かない。
#[test]
fn a_move_with_a_body_that_has_an_unclosed_block_is_refused() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-move-fence");
    let rows = notes.fill(&page, &["a", "b"]);

    let moved = notes.move_edited(&[&id(&rows[1])], &page, "first", "```js\ncode");

    assert!(matches!(moved, Err(AppError::BadRequest(_))), "{moved:?}");
    assert_eq!(notes.outline_of(&page), vs(&["1:a", "1:b"]));
}

// 階層の差と本文を持つ移動は、本文を保存し、階層を変えて動かす。論理アウトデントが、1つの要求で送るものである。
#[test]
fn a_move_with_a_level_delta_saves_the_body_it_carries_and_shifts_the_levels() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-move-outdent");
    let rows = notes.fill(&page, &["p", "  a", "  b", "    c", "  d"]);

    let moved = notes
        .move_with(json!({
            "itemIds": [id(&rows[2]), id(&rows[3])], "targetPageId": page,
            "position": format!("after:{}", id(&rows[4])), "levelDelta": -1, "body": "b typed"
        }))
        .unwrap();

    assert_eq!(bodies(moved), vec![json!("b typed"), json!("c")]);
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:p", "2:a", "2:d", "1:b typed", "2:c"])
    );
}

// トランスクルージョンを本文と一緒に動かすと、本文が保存された元のページの変更も、クライアントに知らされる。どちらのページも1回ずつである。
#[test]
fn the_clients_hear_once_of_each_page_when_a_moved_transclusion_carries_a_body() {
    let notes = Core::new();
    let (source_page, _, page, own) =
        notes.transclusion("items-move-copy-event", &["shown"], 0, &["top", "copy"], 1);

    let told = notes.times_told(&[&source_page, &page], || {
        notes
            .move_edited(&[&id(&own[1])], &page, "first", "shown typed")
            .unwrap();
    });

    assert_eq!(told, [1, 1]);
}
