//! The pages whose change the clients hear of when an item changes. A client that shows one
//! of them reads it again. The changes here are made as an MCP tool makes them.

use journal_core::journal::{ChangedBy, MoveItemsInput, TransclusionInput, UpdateItemInput};
use serde_json::Value;

use crate::common::Core;

impl Core {
    /// How many messages the clients got so far
    fn heard(&self) -> usize {
        self.told.messages().len()
    }

    /// The message about the page that the clients got after the first `since` messages
    fn told_of(&self, since: usize, page_id: &str) -> Value {
        let messages = self.told.messages();
        messages[since..]
            .iter()
            .find(|m| m["type"] == "pagesUpdated" && m["pageId"] == page_id)
            .cloned()
            .unwrap_or_else(|| panic!("no word of {page_id} in {:?}", &messages[since..]))
    }

    fn id_of(item: &Value) -> String {
        item["id"].as_str().unwrap().to_string()
    }

    /// An item, its page, and another page whose only item transcludes the item.
    fn transcluded(&self, name: &str) -> (String, String, String) {
        let source_page = self.new_doc_page(&format!("{name}-source"));
        let source = Core::id_of(&self.fill(&source_page, &["the source"])[0]);
        let page = self.new_doc_page(name);
        let host = Core::id_of(&self.fill(&page, &["placeholder"])[0]);
        self.core
            .journal
            .set_transclusion(
                &host,
                TransclusionInput {
                    source_item_id: Some(source.clone()),
                },
                &ChangedBy::Mcp,
            )
            .unwrap();
        (source, source_page, page)
    }
}

// トランスクルードされている項目を変えると、その写しを見せているページの変更も知らされる。
#[test]
fn the_clients_hear_of_the_pages_that_transclude_an_item_updated_over_mcp() {
    let notes = Core::new();
    let (source, _, page) = notes.transcluded("mcp-tell-transcluder");
    let since = notes.heard();

    notes
        .core
        .journal
        .update_item(
            &source,
            UpdateItemInput {
                body: Some("changed".to_string()),
                ..UpdateItemInput::default()
            },
            &ChangedBy::Mcp,
        )
        .unwrap();

    assert_eq!(notes.told_of(since, &page)["source"], "mcp");
}

// 項目を消すと、その項目があったページと、その写しを見せていたページの変更が知らされる。
#[test]
fn the_clients_hear_of_the_pages_that_showed_an_item_deleted_over_mcp() {
    let notes = Core::new();
    let (source, source_page, page) = notes.transcluded("mcp-tell-delete");
    let since = notes.heard();

    notes
        .core
        .journal
        .delete_item(&source, &ChangedBy::Mcp)
        .unwrap();

    let its_page = notes.told_of(since, &source_page);
    let transcluder = notes.told_of(since, &page);
    assert_eq!(its_page["source"], "mcp");
    assert_eq!(transcluder["source"], "mcp");
}

// 項目をほかのページへ移すと、出たページと入ったページの両方の変更が知らされる。
#[test]
fn the_clients_hear_of_both_pages_of_a_move_over_mcp() {
    let notes = Core::new();
    let from = notes.new_doc_page("mcp-tell-move-from");
    let to = notes.new_doc_page("mcp-tell-move-to");
    let item = Core::id_of(&notes.fill(&from, &["a"])[0]);
    let since = notes.heard();

    notes
        .core
        .journal
        .move_items(
            MoveItemsInput {
                item_ids: Some(vec![item]),
                target_page_id: Some(to.clone()),
                position: Some("last".to_string()),
                ..MoveItemsInput::default()
            },
            &ChangedBy::Mcp,
        )
        .unwrap();

    assert_eq!(notes.told_of(since, &from)["source"], "mcp");
    assert_eq!(notes.told_of(since, &to)["source"], "mcp");
}
