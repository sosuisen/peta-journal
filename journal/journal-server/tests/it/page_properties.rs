//! The properties of a page taken in from the front matter of its file. A device that got a
//! page only from its file holds them, so the file keeps them when that device writes it.

use hmtid::Hmtid;

use super::atlas_sync::{two_devices, Device};

const PAGE: &str = "docs/d/a.md";

impl Device {
    fn set_page_property(&self, page_id: &Hmtid, key: &str, value_type: &str, value: &str) {
        self.state
            .page_service
            .set_property(page_id, key, value_type, value)
            .unwrap();
    }

    /// The scalar property of the page, as the database holds it
    fn page_property(&self, page_id: &Hmtid, key: &str) -> Option<String> {
        let (scalars, _) = self.state.page_service.page_properties(page_id).unwrap();
        scalars.get(page_id.as_str())?.get(key).cloned()
    }
}

// ファイルだけからできたページは、front matterのプロパティを持ち、書き直してもファイルに残す。
#[test]
fn a_page_made_from_a_file_has_the_properties_of_its_front_matter_and_writes_them_again() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.doc("a");
    a.set_page_property(&page, "slug", "text", "my-page");

    b.receive(&a, PAGE);

    assert_eq!(b.page_property(&page, "slug").as_deref(), Some("my-page"));
    b.edit(&page, "edited on B");
    assert!(b.file(PAGE).contains("slug: my-page\n"));
}
