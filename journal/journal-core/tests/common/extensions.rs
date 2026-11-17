//! What the tests of the extension functions do to a core, as a client asks it (ADR 009).
//! The answers are read as the JSON a client gets.

use journal_core::journal::{ChangedBy, CreateItemInput};
use serde_json::Value;

use super::Core;

/// The client that makes the changes of the tests
const CLIENT: &str = "7e577e57-0000-4000-8000-000000000000";

/// A change made by the client of the tests
pub fn by_the_client() -> ChangedBy {
    ChangedBy::Client(CLIENT.parse().unwrap())
}

impl Core {
    /// Opens the document page of the title in the top folder, or makes it, and gives the
    /// answer.
    pub fn open_doc(&self, title: &str) -> Value {
        let answer = self
            .core
            .journal
            .get_or_create_doc(Some(title), Some(""))
            .unwrap();
        serde_json::to_value(answer).unwrap()
    }

    /// Opens the journal of the name, or makes it, and gives the answer.
    pub fn open_journal(&self, name: &str) -> Value {
        let answer = self.core.journal.get_or_create_journal(name).unwrap();
        serde_json::to_value(answer).unwrap()
    }

    /// A new document page of the title; it starts with one empty item. Gives its id.
    pub fn new_doc_page(&self, title: &str) -> String {
        self.open_doc(title)["page"]["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    /// The page with its items.
    pub fn page(&self, page_id: &str) -> Value {
        serde_json::to_value(self.core.journal.get_page(page_id).unwrap()).unwrap()
    }

    /// The items of the page.
    pub fn items(&self, page_id: &str) -> Vec<Value> {
        self.page(page_id)["items"].as_array().unwrap().clone()
    }

    /// The items of the page as `level:body`.
    pub fn outline_of(&self, page_id: &str) -> Vec<String> {
        self.items(page_id).iter().map(outline).collect()
    }

    /// Fills a page with items, one at level 1 for each line, replacing the empty item a
    /// new page starts with, and gives the items in page order.
    pub fn fill(&self, page_id: &str, lines: &[&str]) -> Vec<Value> {
        let first = self.items(page_id)[0]["id"].as_str().unwrap().to_string();
        let mut after = first.clone();
        for body in lines {
            let created = self
                .core
                .journal
                .create_item(
                    CreateItemInput {
                        page_id: Some(page_id.to_string()),
                        position: Some(format!("after:{after}")),
                        level: 1,
                        body: Some(body.to_string()),
                        id: None,
                    },
                    &by_the_client(),
                )
                .unwrap();
            after = created.item.id.clone();
        }
        self.core
            .journal
            .delete_item(&first, &by_the_client())
            .unwrap();
        self.items(page_id)
    }

    /// Sets the tags of the page, as the client does. The extensions of a tag added run
    /// afterwards on a thread of their own.
    pub fn set_tags(&self, page_id: &str, tags: &str) {
        self.core
            .journal
            .set_page_property(page_id, "tags", Some("tags"), Some(tags), &by_the_client())
            .unwrap();
    }

    /// The bodies of the page once it has `count` items. A tag add runs its extensions after
    /// the answer, so the items come a little later.
    pub fn bodies_once(&self, page_id: &str, count: usize) -> Vec<String> {
        for _ in 0..100 {
            let items = self.items(page_id);
            if items.len() >= count {
                return items
                    .iter()
                    .map(|i| i["body"].as_str().unwrap().to_string())
                    .collect();
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!("the page did not get {count} items");
    }

    /// The question an extension asked the clients that has the text in its message
    pub fn question_with(&self, text: &str) -> Value {
        self.told.wait_for(|m| {
            m["type"] == "confirm" && m["message"].as_str().is_some_and(|m| m.contains(text))
        })
    }

    /// Answers the question, as the first client to answer does.
    pub fn answer(&self, question: &Value, ok: bool) {
        let request_id = question["requestId"].as_str().unwrap();
        assert!(self.core.confirm.answer(request_id, ok));
    }

    /// Waits until the clients hear that the extensions of a tag add are done with the page.
    pub fn wait_for_the_extensions(&self, page_id: &str) -> Value {
        self.told.wait_for(|m| {
            m["type"] == "pagesUpdated" && m["pageId"] == page_id && m["source"] == "extension"
        })
    }
}

/// An item as `level:body`.
pub fn outline(item: &Value) -> String {
    format!("{}:{}", item["level"], item["body"].as_str().unwrap())
}
