//! Looking for a document page without making it. An app that only reads asks this before it
//! opens a page (ADR 030).

use crate::common::Core;

impl Core {
    /// The path of the document that the title names in the folder, when there is one.
    fn found(&self, title: &str, dir_path: &str) -> Option<String> {
        let page = self.core.page_service.find_doc(title, dir_path).unwrap();
        page.map(|page| format!("{}{}", page.dir_path, page.name))
    }

    fn doc(&self, title: &str, dir_path: &str) {
        self.core
            .page_service
            .get_or_create_doc(Some(title), Some(dir_path))
            .unwrap();
    }

    fn count_of_pages(&self) -> usize {
        self.core.pages.find_all().unwrap().len()
    }
}

// タイトルとフォルダーで、ドキュメントのページが見つかる。
#[test]
fn a_document_is_found_by_its_title_and_its_folder() {
    let notes = Core::new();
    notes.doc("plan", "work/");

    assert_eq!(notes.found("plan", "work/"), Some("work/plan".into()));
}

// フォルダーの名前では、そのフォルダーのindexページが見つかる。
#[test]
fn the_name_of_a_folder_finds_its_index_page() {
    let notes = Core::new();
    notes.doc("plan", "work/");

    assert_eq!(notes.found("work", ""), Some("work/index".into()));
}

// 大文字と小文字だけが違う名前でも、ページが見つかる。
#[test]
fn a_document_is_found_by_a_name_in_another_case() {
    let notes = Core::new();
    notes.doc("Trip", "");

    assert_eq!(notes.found("trip", ""), Some("Trip".into()));
}

// ないページを探しても、何も見つからず、ページは作られない。
#[test]
fn a_document_that_is_not_there_is_not_found_and_not_made() {
    let notes = Core::new();
    notes.doc("Trip", "");
    let before = notes.count_of_pages();

    assert_eq!(notes.found("ghost", "nowhere/"), None);

    assert_eq!(notes.count_of_pages(), before);
}
