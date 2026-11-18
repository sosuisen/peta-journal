//! Transclusions: an item that shows another item with its tree, and the copy it keeps.

use serde_json::json;

use crate::common::items::{field_of_each, id, input, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

// トランスクルージョンは、ページとともに参照元の木を返す。
#[test]
fn a_transclusion_shows_the_source_tree_with_the_page() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("items-transclusion-source");
    let source = notes.fill(&source_page, &["root", "  child"]);
    let page = notes.new_doc_page("items-transclusion");
    let host = id(&notes.fill(&page, &["placeholder"])[0]);

    let linked = notes.transclude(&host, &id(&source[0]));
    assert_eq!(linked["transcludeItemId"], json!(id(&source[0])));

    let tree = &notes.page(&page)["transcludedTrees"][id(&source[0])];
    assert_eq!(
        field_of_each(&json!({ "items": tree }), "items", "body"),
        vec![json!("root"), json!("child")]
    );

    notes
        .core
        .journal
        .remove_transclusion(&host, &by_the_client())
        .unwrap();
    assert!(notes.items(&page)[0]["transcludeItemId"].is_null());
}

// 削除の前に、どの項目が他の項目からトランスクルードされているかを尋ねられる。
#[test]
fn the_items_other_items_transclude_are_told_apart_before_a_deletion() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("items-transcluded-source");
    let source = notes.fill(&source_page, &["shown elsewhere", "kept here"]);
    let page = notes.new_doc_page("items-transcluded");
    let host = id(&notes.fill(&page, &["placeholder"])[0]);
    notes.transclude(&host, &id(&source[0]));

    let transcluded = notes
        .core
        .journal
        .transcluded_items(input(
            json!({ "itemIds": [id(&source[0]), id(&source[1]), host] }),
        ))
        .unwrap();

    assert_eq!(transcluded.item_ids, [id(&source[0])]);
}

// トランスクルージョンを外すと、項目の本文は空になり、保存済みのASTも空になる。
#[test]
fn removing_a_transclusion_leaves_an_empty_body_and_an_empty_ast() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("items-transclusion-remove-source");
    let source = notes.fill(&source_page, &["**root**"]);
    let page = notes.new_doc_page("items-transclusion-remove");
    let host = id(&notes.fill(&page, &["placeholder"])[0]);
    notes.transclude(&host, &id(&source[0]));
    let shown = notes.item(&host);
    assert_ne!(shown["ast"], json!(""), "{shown}");

    notes
        .core
        .journal
        .remove_transclusion(&host, &by_the_client())
        .unwrap();

    let removed = notes.item(&host);
    assert_eq!(removed["body"], json!(""));
    assert_eq!(removed["ast"], json!(""));
}

// トランスクルージョンの設定は項目の変更として数えられ、日付での同期がそれを拾う。
#[test]
fn setting_a_transclusion_counts_as_a_change_of_the_item() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("items-transclusion-change-source");
    let source = notes.fill(&source_page, &["root"]);
    let page = notes.new_doc_page("items-transclusion-change");
    let host = id(&notes.fill(&page, &["placeholder"])[0]);
    let modified_before = field_of_each(&notes.page(&page), "items", "modifiedDate").remove(0);
    notes.clock.advance(5);

    notes.transclude(&host, &id(&source[0]));

    let modified_after = field_of_each(&notes.page(&page), "items", "modifiedDate").remove(0);
    assert!(
        modified_after.as_i64() > modified_before.as_i64(),
        "{modified_before} -> {modified_after}"
    );
}

// 参照元の項目を編集すると、トランスクルードしている側の保存済みの写しも更新される。
#[test]
fn editing_a_source_item_updates_the_stored_copy_in_its_transcluders() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("items-transclusion-edit-source");
    let source = id(&notes.fill(&source_page, &["before"])[0]);
    let page = notes.new_doc_page("items-transclusion-edit");
    let host = id(&notes.fill(&page, &["placeholder"])[0]);
    notes.transclude(&host, &source);

    notes.update_item(&source, json!({ "body": "after" }));

    // The exported file of the host page is built from the stored copy, not from the source
    assert_eq!(notes.items(&page)[0]["body"], json!("after"));
    let exported = notes
        .core
        .journal
        .export_page_markdown(Some(&page))
        .unwrap();
    assert!(
        exported.flat_markdown.contains("after"),
        "{}",
        exported.flat_markdown
    );
}

// 参照元の木を削除すると、トランスクルージョンは解除され、そのときの子孫もトランスクルードしている側に普通の項目として残る。
// 子孫の本文、AST、折りたたみ、型、ハッシュタグ、入れ子のトランスクルージョンは、そのまま写される。
#[test]
fn deleting_a_source_item_leaves_its_descendants_as_a_copy_in_its_transcluders() {
    let notes = Core::new();
    let other_page = notes.new_doc_page("items-transclusion-delete-other");
    let other = id(&notes.fill(&other_page, &["other"])[0]);
    let source_page = notes.new_doc_page("items-transclusion-delete-source");
    let source = notes.fill(&source_page, &["root", "  # child #kept", "    grandchild"]);
    notes.update_item(&id(&source[1]), json!({ "collapsed": true }));
    notes.transclude(&id(&source[2]), &other);
    let page = notes.new_doc_page("items-transclusion-delete");
    let host = notes.fill(&page, &["intro", "  host", "    own child", "after"]);
    notes.transclude(&id(&host[1]), &id(&source[0]));
    let child_ast = field_of_each(&notes.page(&source_page), "items", "ast").remove(1);

    // The whole tree goes in one group: the copies are made before any of it is deleted
    notes
        .core
        .journal
        .delete_items(
            input(json!({ "itemIds": [id(&source[0]), id(&source[1]), id(&source[2])] })),
            &by_the_client(),
        )
        .unwrap();

    // The copies follow the host item and come before its own children
    assert_eq!(
        notes.outline_of(&page),
        vs(&[
            "1:intro",
            "2:root",
            "3:# child #kept",
            "4:other",
            "3:own child",
            "1:after"
        ])
    );
    let copies = notes.items(&page);
    assert!(copies[1]["transcludeItemId"].is_null());
    assert_eq!(copies[2]["types"], json!(["heading"]));
    assert_eq!(copies[2]["collapsed"], json!(true));
    assert_eq!(copies[2]["properties"]["tags"], json!(["kept"]));
    assert_eq!(copies[3]["transcludeItemId"], json!(other));
    assert_eq!(
        field_of_each(&notes.page(&page), "items", "ast")[2],
        child_ast
    );
}
