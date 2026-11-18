//! Deleting items: a group with its descendants in one request, or one item alone.

use journal_core::error::AppError;
use journal_core::journal::DeletedItemsAnswer;
use serde_json::{json, Value};

use crate::common::items::{answer, field_of_each, id, input, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Deletes the items as one group, as the client asks
    fn delete_group(&self, item_ids: &[String]) -> Result<DeletedItemsAnswer, AppError> {
        self.core
            .journal
            .delete_items(input(json!({ "itemIds": item_ids })), &by_the_client())
    }

    /// Deletes as `delete_group` does, and gives the answer as the JSON a client gets
    fn deleted_group(&self, item_ids: &[String]) -> Value {
        answer(self.delete_group(item_ids).unwrap())
    }
}

/// The ids of the rows at the indexes
fn ids_at(rows: &[Value], at: &[usize]) -> Vec<String> {
    at.iter().map(|&i| id(&rows[i])).collect()
}

// グループ削除は、項目を子孫ごと1回のリクエストで消す。
#[test]
fn a_group_of_items_is_deleted_with_its_descendants_in_one_request() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-group-delete");
    let rows = notes.fill(
        &page,
        &[
            "keep",
            "parent",
            "  child",
            "    grandchild",
            "sibling",
            "last",
        ],
    );

    let deleted = notes.deleted_group(&ids_at(&rows, &[1, 2, 3, 4]));

    assert_eq!(notes.outline_of(&page), vs(&["1:keep", "1:last"]));
    // Items are left on the page, so no empty item is made
    assert_eq!(deleted["created"], json!([]));
}

// 子孫がそろっていないグループは断られ、何も消えない。
#[test]
fn a_group_that_leaves_a_descendant_behind_is_refused_and_nothing_is_deleted() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-group-delete-refuse");
    let rows = notes.fill(&page, &["parent", "  child", "    grandchild", "sibling"]);

    let orphaning = notes.delete_group(&ids_at(&rows, &[0, 1, 3]));
    assert!(
        matches!(orphaning, Err(AppError::BadRequest(_))),
        "{orphaning:?}"
    );
    let empty = notes.delete_group(&[]);
    assert!(matches!(empty, Err(AppError::BadRequest(_))), "{empty:?}");

    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:parent", "2:child", "3:grandchild", "1:sibling"])
    );
}

// グループ削除でページの項目がすべて消えると、同じ要求で空の項目が1つ作られ、答えに入る。
#[test]
fn a_group_delete_that_empties_the_page_makes_one_empty_item_and_answers_it() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-group-delete-all");
    let rows = notes.fill(&page, &["a", "  a1"]);

    let deleted = notes.deleted_group(&ids_at(&rows, &[0, 1]));

    let left = notes.items(&page);
    assert_eq!(notes.outline_of(&page), vs(&["1:"]));
    assert_eq!(
        field_of_each(&deleted, "created", "id"),
        vec![json!(id(&left[0]))]
    );
    assert_eq!(deleted["created"][0]["pageId"], json!(page));
}

// トランスクルージョンをグループ削除すると、その項目だけが消え、元の項目とその子は元のページに残る。
#[test]
fn a_group_delete_of_a_transclusion_deletes_only_that_item_and_keeps_its_source() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("items-group-delete-source");
    let source = notes.fill(&source_page, &["root", "  child"]);
    let page = notes.new_doc_page("items-group-delete-transclusion");
    let rows = notes.fill(&page, &["host", "  own child", "after"]);
    notes.transclude(&id(&rows[0]), &id(&source[0]));

    notes.deleted_group(&ids_at(&rows, &[0, 1]));

    assert_eq!(notes.outline_of(&page), vs(&["1:after"]));
    assert_eq!(notes.outline_of(&source_page), vs(&["1:root", "2:child"]));
}

// 単体の削除は、子を持つ項目を断る。子のない項目は消える。
#[test]
fn a_single_delete_refuses_an_item_with_children() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-single-delete");
    let rows = notes.fill(&page, &["parent", "  child"]);

    let parent = notes
        .core
        .journal
        .delete_item(&id(&rows[0]), &by_the_client());
    assert!(matches!(parent, Err(AppError::BadRequest(_))), "{parent:?}");
    assert_eq!(notes.outline_of(&page), vs(&["1:parent", "2:child"]));

    notes.delete_item(&id(&rows[1]));
    assert_eq!(notes.outline_of(&page), vs(&["1:parent"]));
}

// 単体の削除でページの最後の項目が消えると、同じ要求で空の項目が1つ作られる。
#[test]
fn a_single_delete_of_the_last_item_leaves_one_empty_item_on_the_page() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-single-delete-last");
    let rows = notes.fill(&page, &["only"]);

    notes.delete_item(&id(&rows[0]));

    let left = notes.items(&page);
    assert_eq!(notes.outline_of(&page), vs(&["1:"]));
    assert_ne!(id(&left[0]), id(&rows[0]));
}
