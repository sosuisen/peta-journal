//! Merging an item into the one above it, as Backspace at the start of an item does.

use journal_core::error::AppError;
use journal_core::journal::ItemAnswer;
use serde_json::{json, Value};

use crate::common::items::{field_of_each, id, input, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Merges items as the client asks in the JSON of the request
    fn merge(&self, request: Value) -> Result<ItemAnswer, AppError> {
        self.core
            .journal
            .merge_items(input(request), &by_the_client())
    }
}

// 結合すると、消える項目の本文が受け取る項目の末尾につながり、消える項目の子は受け取る項目の子になる。
#[test]
fn merging_appends_the_body_and_hands_the_children_to_the_receiving_item() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-merge");
    let rows = notes.fill(
        &page,
        &["first", "  first child", "second", "  second child"],
    );

    let merged = notes
        .merge(json!({ "itemId": id(&rows[2]), "intoId": id(&rows[0]) }))
        .unwrap();

    assert_eq!(merged.item.item.body, "firstsecond");
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:firstsecond", "2:first child", "2:second child"])
    );
    assert_eq!(
        field_of_each(&notes.page(&page), "items", "parentItemId")[2],
        json!(id(&rows[0]))
    );
}

// 要求が本文を運んできた項目は、保存されている本文ではなく、その本文で結合される。エディタに入力してまだ保存していない文が入っている。運んでこなかった項目は、保存されている本文で結合される。
#[test]
fn merging_joins_the_bodies_the_request_carries_and_the_stored_ones_for_the_rest() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-merge-unsaved");
    let rows = notes.fill(&page, &["first", "second", "third"]);

    let merged = notes
        .merge(json!({ "itemId": id(&rows[1]), "intoId": id(&rows[0]), "body": "second typed" }))
        .unwrap();
    assert_eq!(merged.item.item.body, "firstsecond typed");

    notes
        .merge(json!({ "itemId": id(&rows[2]), "intoId": id(&rows[0]), "intoBody": "typed " }))
        .unwrap();
    assert_eq!(notes.outline_of(&page), vs(&["1:typed third"]));
}

// 結合した本文が閉じていないブロックで終われば、結合は断られ、2つの項目はそのまま残る。閉じる記号を足すと、後ろの本文がブロックの中身になってしまう（ADR 050）。
#[test]
fn a_merge_that_leaves_a_block_unclosed_is_refused() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-merge-fence");
    let rows = notes.fill(&page, &["```\ncode\n```", "after"]);

    let merged = notes.merge(json!({ "itemId": id(&rows[1]), "intoId": id(&rows[0]) }));

    assert!(matches!(merged, Err(AppError::BadRequest(_))), "{merged:?}");
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:```\ncode\n```", "1:after"])
    );
}

// トランスクルージョンは結合できない。消える側でも受け取る側でも断られる。
#[test]
fn a_transclusion_is_not_merged_on_either_side() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("items-merge-refuse-source");
    let source = id(&notes.fill(&source_page, &["source"])[0]);
    let page = notes.new_doc_page("items-merge-refuse");
    let rows = notes.fill(&page, &["plain", "host", "below"]);
    notes.transclude(&id(&rows[1]), &source);

    let away = notes.merge(json!({ "itemId": id(&rows[1]), "intoId": id(&rows[0]) }));
    assert!(matches!(away, Err(AppError::BadRequest(_))), "{away:?}");
    let into = notes.merge(json!({ "itemId": id(&rows[2]), "intoId": id(&rows[1]) }));
    assert!(matches!(into, Err(AppError::BadRequest(_))), "{into:?}");
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:plain", "1:source", "1:below"])
    );
}

// トランスクルードされている項目を結合で消すと、埋め込み側には本文と子孫の写しが普通の項目として残る。
#[test]
fn merging_away_a_source_item_leaves_a_copy_in_its_transcluders() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("items-merge-source");
    let source = notes.fill(&source_page, &["above", "source", "  child"]);
    let page = notes.new_doc_page("items-merge-host");
    let host = id(&notes.fill(&page, &["host"])[0]);
    notes.transclude(&host, &id(&source[1]));

    notes
        .merge(json!({ "itemId": id(&source[1]), "intoId": id(&source[0]) }))
        .unwrap();

    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:abovesource", "2:child"])
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:source", "2:child"]));
    assert!(notes
        .items(&page)
        .iter()
        .all(|i| i["transcludeItemId"].is_null()));
}

// 受け取る項目がトランスクルードされていれば、結合した本文が埋め込み側の写しにも入る。
#[test]
fn merging_into_a_transcluded_item_updates_the_stored_copy_in_its_transcluders() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-merge-into-source");
    let rows = notes.fill(&page, &["first", "second"]);
    let host_page = notes.new_doc_page("items-merge-into-host");
    let host = id(&notes.fill(&host_page, &["placeholder"])[0]);
    notes.transclude(&host, &id(&rows[0]));
    let modified_before = field_of_each(&notes.page(&host_page), "items", "modifiedDate").remove(0);
    notes.clock.advance(5);

    notes
        .merge(json!({ "itemId": id(&rows[1]), "intoId": id(&rows[0]) }))
        .unwrap();

    assert_eq!(notes.items(&host_page)[0]["body"], json!("firstsecond"));
    // The copy carries the parsed form too, and counts as changed, so a sync by date picks it up
    let source = notes.page(&page);
    let copy = notes.page(&host_page);
    assert_eq!(
        field_of_each(&copy, "items", "ast")[0],
        field_of_each(&source, "items", "ast")[0]
    );
    let modified_after = field_of_each(&copy, "items", "modifiedDate").remove(0);
    assert!(
        modified_after.as_i64() > modified_before.as_i64(),
        "{modified_before} -> {modified_after}"
    );
}
