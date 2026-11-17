//! What a client does to the items of a core, through the journal, as the REST API does it.
//! The inputs are made from the JSON of the REST requests, and the answers are read as the
//! JSON a client gets.

use journal_core::error::AppResult;
use journal_core::journal::ItemAnswer;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

use super::pages::by_the_client;
use super::Core;

/// An input of the journal, made from the JSON of its REST request
pub fn input<T: DeserializeOwned>(request: Value) -> T {
    serde_json::from_value(request).unwrap()
}

/// An answer of the journal as the JSON a client gets
pub fn answer<T: Serialize>(answer: T) -> Value {
    serde_json::to_value(answer).unwrap()
}

/// The id of an item of an answer
pub fn id(item: &Value) -> String {
    item["id"].as_str().unwrap().to_string()
}

/// Whether the item has the type
pub fn has_type(item: &Value, item_type: &str) -> bool {
    item["types"]
        .as_array()
        .unwrap()
        .contains(&json!(item_type))
}

/// "level:body" of each item, the shape most assertions compare
pub fn outline(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .map(|item| format!("{}:{}", item["level"], item["body"].as_str().unwrap()))
        .collect()
}

/// `list[*].field` of an answer
pub fn field_of_each(answer: &Value, list: &str, field: &str) -> Vec<Value> {
    answer[list]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item[field].clone())
        .collect()
}

pub fn vs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

impl Core {
    /// "level:body" of each item of the page, in page order
    pub fn outline_of(&self, page_id: &str) -> Vec<String> {
        outline(&self.items(page_id))
    }

    /// An item made by the client at the position, as the item of the answer
    pub fn create_item(&self, page_id: &str, position: &str, level: i32, body: &str) -> Value {
        let created = self
            .core
            .journal
            .create_item(
                input(json!({ "pageId": page_id, "position": position, "level": level, "body": body, "id": "" })),
                &by_the_client(),
            )
            .unwrap();
        answer(created.item)
    }

    /// The item after the changes the client asks for in the JSON of the request
    pub fn update_item(&self, item_id: &str, changes: Value) -> Value {
        let updated = self
            .core
            .journal
            .update_item(item_id, input(changes), &by_the_client())
            .unwrap();
        answer(updated.item)
    }

    /// The items moved to the position of the page, as they are after the move
    pub fn move_items(
        &self,
        item_ids: &[&str],
        target_page_id: &str,
        position: &str,
        level_delta: Option<i32>,
    ) -> Vec<Value> {
        let mut request =
            json!({ "itemIds": item_ids, "targetPageId": target_page_id, "position": position });
        if let Some(delta) = level_delta {
            request["levelDelta"] = json!(delta);
        }
        let moved = self
            .core
            .journal
            .move_items(input(request), &by_the_client())
            .unwrap();
        moved.items.into_iter().map(answer).collect()
    }

    /// A source page and a page with the given rows, where the row `root` of the page
    /// transcludes the row `shown` of the source page. Gives both pages with their rows.
    pub fn transclusion(
        &self,
        name: &str,
        source_rows: &[&str],
        shown: usize,
        rows: &[&str],
        root: usize,
    ) -> (String, Vec<Value>, String, Vec<Value>) {
        let source_page = self.new_doc_page(&format!("{name}-source"));
        let source = self.fill(&source_page, source_rows);
        let page = self.new_doc_page(name);
        let own = self.fill(&page, rows);
        self.transclude(&id(&own[root]), &id(&source[shown]));
        (source_page, source, page, own)
    }

    /// A source page with the given rows, and a page whose only item transcludes the first
    /// row. Gives both pages, the rows of the source page and the transcluding item.
    pub fn transcluded_rows(
        &self,
        name: &str,
        source_rows: &[&str],
    ) -> (String, Vec<Value>, String, String) {
        let source_page = self.new_doc_page(&format!("{name}-source"));
        let source = self.fill(&source_page, source_rows);
        let page = self.new_doc_page(name);
        let host = id(&self.fill(&page, &["placeholder"])[0]);
        self.transclude(&host, &id(&source[0]));
        (source_page, source, page, host)
    }

    /// The indexes of the items of a page, in order
    pub fn idxs_of(&self, page_id: &str) -> Vec<String> {
        self.items(page_id)
            .iter()
            .map(|i| i["idx"].as_str().unwrap().to_string())
            .collect()
    }

    /// Makes the item show the source item, and gives the item of the answer
    pub fn transclude(&self, item_id: &str, source_id: &str) -> Value {
        let set = self
            .core
            .journal
            .set_transclusion(
                item_id,
                input(json!({ "sourceItemId": source_id })),
                &by_the_client(),
            )
            .unwrap();
        answer(set.item)
    }

    /// The item as the client reads it
    pub fn item(&self, item_id: &str) -> Value {
        answer(self.core.journal.get_item(item_id).unwrap().item)
    }

    /// Deletes the item, as the client does
    pub fn delete_item(&self, item_id: &str) {
        self.core
            .journal
            .delete_item(item_id, &by_the_client())
            .unwrap();
    }

    /// Sets a property of the item as the client sends it in the JSON of the request
    pub fn set_item_property(
        &self,
        item_id: &str,
        key: &str,
        request: Value,
    ) -> AppResult<ItemAnswer> {
        self.core
            .journal
            .set_item_property(item_id, key, input(request), &by_the_client())
    }

    /// Defines a key with the type its name gives, as the user does in Settings.
    pub fn define_key(&self, key: &str) {
        self.core
            .journal
            .add_property_definition(input(json!({ "key": key })))
            .unwrap();
    }

    /// How often the clients were told that each of the pages changed while `act` ran
    pub fn times_told(&self, pages: &[&str], act: impl FnOnce()) -> Vec<usize> {
        let since = self.told.messages().len();
        act();
        let messages = self.told.messages();
        pages
            .iter()
            .map(|page| {
                messages[since..]
                    .iter()
                    .filter(|m| m["type"] == "pagesUpdated" && m["pageId"] == *page)
                    .count()
            })
            .collect()
    }
}
