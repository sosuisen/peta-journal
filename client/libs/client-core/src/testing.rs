//! Item fixtures for tests, like `core/testing/item-fixture.ts`.

use crate::effects::{ApiOp, ApiOutput};
use crate::model::{DocPage, Item, ItemRef, Page, PageWithItems};
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

/// What a row of the test harness shows of a body: the `RichText` that a native shell draws
/// (ADR 042), with no folder of tags.
pub fn shown(body: &str) -> crate::RichText {
    use crate::BodyModel;
    crate::RichText::of(crate::asts::parse(body), "")
}

/// What a fake server answers to a doc lookup or a page load: the page with these items.
/// A doc lookup and a page load have answers of different types.
pub fn page_answer(op: &ApiOp, page: Page, items: Vec<Item>) -> ApiOutput {
    match op {
        ApiOp::GetOrCreateDoc { .. } => ApiOutput::DocPage(DocPage {
            page,
            items,
            created: false,
        }),
        _ => ApiOutput::Page(PageWithItems {
            page,
            items,
            transcluded_trees: Default::default(),
        }),
    }
}
