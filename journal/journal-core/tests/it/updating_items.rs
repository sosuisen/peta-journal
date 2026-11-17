//! Changing items: their level, their collapsed state and their properties.

use journal_core::error::AppError;
use serde_json::{json, Value};

use crate::common::items::{answer, id};
use crate::common::Core;

// レベルと折りたたみ状態が保存される。
#[test]
fn level_and_collapsed_state_are_saved() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-flags");
    let rows = notes.fill(&page, &["p", "  c"]);

    notes.update_item(&id(&rows[0]), json!({ "collapsed": true }));
    notes.update_item(&id(&rows[1]), json!({ "level": 3 }));

    let after = notes.items(&page);
    assert_eq!(after[0]["collapsed"], json!(true));
    assert_eq!(after[1]["level"], json!(3));
}

// checkbox型のプロパティは、trueとfalseだけを持つ。それ以外の値はfalseになる。
#[test]
fn a_checkbox_property_holds_true_or_false() {
    let notes = Core::new();
    notes.define_key("done_cb");
    let page = notes.new_doc_page("items-checkbox");
    let item = id(&notes.fill(&page, &["checked"])[0]);

    for (sent, held) in [("true", "true"), ("false", "false"), ("yes", "false")] {
        notes
            .set_item_property(
                &item,
                "done_cb",
                json!({ "valueType": "checkbox", "value": sent }),
            )
            .unwrap();
        assert_eq!(notes.items(&page)[0]["properties"]["done_cb"], json!(held));
    }
}

// 項目のユーザー定義のlist型のキーは、そのキーのリストとして保存され、タグには触らない。
#[test]
fn a_list_of_an_item_is_kept_under_its_own_key_apart_from_the_tags() {
    let notes = Core::new();
    notes.define_key("item_list");
    let page = notes.new_doc_page("items-list");
    let item = id(&notes.fill(&page, &["listed #items-list-tag"])[0]);

    // The second value replaces the first, as a value of a scalar does
    let mut last = Value::Null;
    for value in ["x, y", "y,z"] {
        let set = notes
            .set_item_property(
                &item,
                "item_list",
                json!({ "valueType": "list", "value": value }),
            )
            .unwrap();
        last = answer(set);
    }

    assert_eq!(last["item"]["properties"]["item_list"], json!(["y", "z"]));
    let after = notes.items(&page).remove(0);
    assert_eq!(after["properties"]["item_list"], json!(["y", "z"]));
    assert_eq!(after["properties"]["tags"], json!(["items-list-tag"]));
}

// 定義のないキーには、項目の値を書けない。名前に合う型でも、合わない型でも同じである。型は定義するときだけ決まるからである（ADR 047）。
#[test]
fn a_key_with_no_definition_takes_no_value_of_an_item() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-type-by-name");
    let item = id(&notes.fill(&page, &["typed"])[0]);

    for (key, body) in [
        (
            "undefined_note",
            json!({ "valueType": "text", "value": "x" }),
        ),
        (
            "undefined_score_n",
            json!({ "valueType": "number", "value": "1" }),
        ),
        (
            "undefined_member_list",
            json!({ "valueType": "list", "value": "x" }),
        ),
        (
            "undefined_note_list",
            json!({ "valueType": "text", "value": "x" }),
        ),
    ] {
        let set = notes.set_item_property(&item, key, body.clone());
        assert!(
            matches!(set, Err(AppError::BadRequest(_))),
            "{key} {body} -> {set:?}"
        );
    }
}
