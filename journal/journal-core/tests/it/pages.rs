//! Pages: what counts as a change of a page.

use serde_json::json;

use crate::common::items::answer;
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Sets a property of the page, as the client does, and gives the page of the answer
    fn set_page_property(
        &self,
        page_id: &str,
        key: &str,
        value_type: &str,
        value: &str,
    ) -> serde_json::Value {
        let changed = self
            .core
            .journal
            .set_page_property(
                page_id,
                key,
                Some(value_type),
                Some(value),
                &by_the_client(),
            )
            .unwrap();
        answer(changed)["page"].clone()
    }

    /// Gives the page a modified date in the past, so a later change shows as a new date.
    fn date_the_page_in_the_past(&self, page_id: &str) -> i64 {
        let page = self.set_page_property(page_id, "modified", "datetime", "2021-02-03T04:05:06Z");
        page["modifiedDate"].as_i64().unwrap()
    }
}

// ページにプロパティを設定すると、ページの変更日が新しくなる。
#[test]
fn setting_a_property_counts_as_a_change_of_the_page() {
    let notes = Core::new();
    let page = notes.new_doc_page("pages-property-date");
    let old = notes.date_the_page_in_the_past(&page);

    let changed = notes.set_page_property(&page, "tags", "tags", "dated");

    assert_ne!(changed["modifiedDate"], json!(old));
}
