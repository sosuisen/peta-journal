//! Item fixtures for tests, like `core/testing/item-fixture.ts`.

use crate::model::{Item, ItemRef};
use std::sync::Arc;

pub fn plain_item(id: &str, level: i32, body: &str) -> Item {
    Item {
        id: id.to_string(),
        page_id: "page1".to_string(),
        dir_path: String::new(),
        name: "test-page".to_string(),
        item_type: "doc".to_string(),
        created_date: 0,
        modified_date: 0,
        idx: id.to_string(),
        level,
        body: body.to_string(),
        collapsed: false,
        types: Vec::new(),
        properties: Default::default(),
        transclude_item_id: None,
    }
}

pub fn item_with(id: &str, level: i32, change: impl FnOnce(&mut Item)) -> Item {
    let mut item = plain_item(id, level, "");
    change(&mut item);
    item
}

pub fn item(id: &str, level: i32) -> ItemRef {
    Arc::new(plain_item(id, level, ""))
}
