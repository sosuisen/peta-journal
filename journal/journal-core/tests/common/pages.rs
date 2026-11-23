//! What a client does to the pages of a core, through the journal. The answers are read as
//! the JSON a client gets.

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

    /// Opens the document page of the title in the folder, or makes it, and gives the answer.
    pub fn open_doc_in(&self, title: &str, dir_path: &str) -> Value {
        let answer = self
            .core
            .journal
            .get_or_create_doc(Some(title), Some(dir_path))
            .unwrap();
        serde_json::to_value(answer).unwrap()
    }

    /// Sets the tags of the page, as the client does. The extensions of a tag added run
    /// afterwards on a thread of their own.
    pub fn set_tags(&self, page_id: &str, tags: &str) {
        self.core
            .journal
            .set_page_property(page_id, "tags", Some("tags"), Some(tags), &by_the_client())
            .unwrap();
    }

    /// The pages whose names the query finds, at most 10, as the JSON a client gets. The
    /// index commits first, as the app has it commit every little while.
    pub fn search_pages(&self, query: &str) -> Value {
        self.core.tantivy.commit_pending();
        let found = self
            .core
            .journal
            .search_page_names(query, Some(10))
            .unwrap();
        serde_json::to_value(found).unwrap()
    }

    /// The id of a doc page by its name.
    pub fn doc_page_named(&self, name: &str) -> Option<String> {
        let recent = serde_json::to_value(
            self.core
                .journal
                .recent_pages(Some(2000), Some("doc"))
                .unwrap(),
        )
        .unwrap();
        recent["pages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == serde_json::json!(name))
            .map(|p| p["id"].as_str().unwrap().to_string())
    }

    /// The atlas page of the folder, made as the app makes it. Gives its id.
    pub fn atlas_of(&self, dir: &str) -> String {
        self.open_doc_in("index_atlas", dir)["page"]["id"]
            .as_str()
            .unwrap()
            .to_string()
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

    /// Fills a page with items given as "body" or "  body" (two spaces per level below 1),
    /// replacing the empty item a new page starts with, and gives the items in page order.
    pub fn fill(&self, page_id: &str, lines: &[&str]) -> Vec<Value> {
        let first = self.items(page_id)[0]["id"].as_str().unwrap().to_string();
        let mut after = first.clone();
        for line in lines {
            let body = line.trim_start();
            let level = 1 + (line.len() - body.len()) as i32 / 2;
            let created = self
                .core
                .journal
                .create_item(
                    CreateItemInput {
                        page_id: Some(page_id.to_string()),
                        position: Some(journal_api_types::ItemPosition::after(after)),
                        level,
                        body: Some(body.to_string()),
                        id: None,
                    },
                    &by_the_client(),
                )
                .unwrap();
            after = created.item.item.id.clone();
        }
        self.core
            .journal
            .delete_item(&first, &by_the_client())
            .unwrap();
        self.items(page_id)
    }
}
