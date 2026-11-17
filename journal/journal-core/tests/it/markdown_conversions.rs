//! Items exported as tree markdown and Flat Markdown, and taken back.

use serde_json::{json, Value};

use crate::common::items::{answer, field_of_each, id, input};
use crate::common::Core;

impl Core {
    /// The items as tree markdown and Flat Markdown, as the JSON a client gets
    fn export(&self, item_ids: &[String]) -> Value {
        answer(
            self.core
                .journal
                .export_tree_markdown(input(json!({ "itemIds": item_ids })))
                .unwrap(),
        )
    }
}

// エクスポートした項目は、インポートと解析を経て元のまま戻る。
#[test]
fn exported_items_come_back_through_import_and_parse_unchanged() {
    let notes = Core::new();
    let page = notes.new_doc_page("markdown-roundtrip");
    let rows = notes.fill(
        &page,
        &[
            "parent",
            "  child one",
            "    grandchild",
            "  child two",
            "sibling",
        ],
    );
    let ids: Vec<String> = rows.iter().map(id).collect();

    let exported = notes.export(&ids);
    let tree = exported["treeMarkdown"].as_str().unwrap();
    assert!(tree.contains("- parent"), "{tree}");
    assert!(tree.contains("    - grandchild"), "{tree}");
    assert!(
        exported["flatMarkdown"]
            .as_str()
            .unwrap()
            .contains("<!-- @"),
        "flat markdown keeps the item metadata"
    );

    let imported = notes
        .core
        .journal
        .import_tree_markdown(input(json!({ "treeMarkdown": tree })));
    let parsed = answer(
        notes
            .core
            .journal
            .parse_flat_markdown(input(json!({ "markdown": imported.flat_markdown })))
            .unwrap(),
    );

    assert_eq!(
        field_of_each(&parsed, "items", "body"),
        rows.iter().map(|r| r["body"].clone()).collect::<Vec<_>>()
    );
    assert_eq!(
        field_of_each(&parsed, "items", "level"),
        rows.iter().map(|r| r["level"].clone()).collect::<Vec<_>>()
    );
}

// 項目とその子孫のエクスポートは、その項目たちのエクスポートと同じになる。兄弟は入らない。
#[test]
fn a_subtree_exports_like_its_items() {
    let notes = Core::new();
    let page = notes.new_doc_page("markdown-subtree");
    let rows = notes.fill(&page, &["one", "  two", "sibling"]);

    let by_subtree = answer(
        notes
            .core
            .journal
            .export_subtree_markdown(Some(&id(&rows[0])))
            .unwrap(),
    );
    let by_items = notes.export(&[id(&rows[0]), id(&rows[1])]);

    assert_eq!(by_items["treeMarkdown"], by_subtree["treeMarkdown"]);
    assert_eq!(by_subtree["treeMarkdown"], json!("- one\n  - two"));
}
