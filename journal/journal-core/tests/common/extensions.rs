//! What the tests of the extension functions do to a core, as a client asks it (ADR 009).
//! The answers are read as the JSON a client gets.

use serde_json::Value;

use super::Core;

impl Core {
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
