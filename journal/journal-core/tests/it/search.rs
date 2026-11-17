//! Search: page names, the text of items, and tags.

use serde_json::{json, Value};

use crate::common::items::{answer, field_of_each, id};
use crate::common::Core;

impl Core {
    /// The pages and items the combined search finds, at most 10. The index commits first,
    /// as the app has it commit every little while.
    fn search_combined(&self, query: &str) -> Value {
        self.core.tantivy.commit_pending();
        answer(self.core.journal.search_combined(query, Some(10)).unwrap())
    }

    /// The items that have the tag, at most 10
    fn search_tags(&self, tag: &str) -> Value {
        answer(self.core.journal.search_by_tag(tag, Some(10)).unwrap())
    }
}

// ページ名は前方一致で見つかる。
#[test]
fn page_names_are_found_by_prefix() {
    let notes = Core::new();
    notes.new_doc_page("Searchable-page");

    let found = notes.search_pages("Searchable-");
    let titles = field_of_each(&found, "results", "pageTitle");

    assert!(
        titles.contains(&json!("Searchable-page")),
        "titles: {titles:?}"
    );
}

// 項目の本文は、抜粋付きで見つかる。
#[test]
fn item_text_is_found_with_a_snippet() {
    let notes = Core::new();
    let page = notes.new_doc_page("search-text");
    let item = id(&notes.fill(&page, &["the zebra123 crossing", "nothing here"])[0]);

    let found = notes.search_combined("zebra123");

    assert_eq!(
        field_of_each(&found, "results", "itemId"),
        vec![json!(item)]
    );
    assert_eq!(found["results"][0]["pageId"], json!(page));
    // The analyzer cuts letters from digits, so the match is marked word by word
    let snippet = found["results"][0]["snippet"].as_str().unwrap_or("");
    assert!(snippet.contains("<mark>zebra"), "{snippet}");
}

// 2語の検索では、両方の語を含む項目だけが見つかる。
#[test]
fn two_words_must_both_be_in_the_item() {
    let notes = Core::new();
    let page = notes.new_doc_page("search-and");
    let rows = notes.fill(&page, &["apple and banana", "apple alone"]);

    let found = notes.search_combined("apple banana");

    assert_eq!(
        field_of_each(&found, "results", "itemId"),
        vec![json!(id(&rows[0]))]
    );
}

// 本文のハッシュタグはタグとして見つかる。
#[test]
fn a_hashtag_in_the_body_is_found_as_a_tag() {
    let notes = Core::new();
    let page = notes.new_doc_page("search-tag");
    let item = id(&notes.fill(&page, &["remember #searched"])[0]);

    assert_eq!(
        notes.items(&page)[0]["properties"]["tags"],
        json!(["searched"])
    );
    let found = notes.search_tags("searched");
    assert_eq!(
        field_of_each(&found, "results", "itemId"),
        vec![json!(item)]
    );
}

// タグは大文字と小文字を区別せずに見つかる。
#[test]
fn a_tag_is_found_in_any_case_of_its_letters() {
    let notes = Core::new();
    let page = notes.new_doc_page("search-tag-case");
    let item = id(&notes.fill(&page, &["remember #CaseTag"])[0]);

    let found = notes.search_tags("casetag");
    assert_eq!(
        field_of_each(&found, "results", "itemId"),
        vec![json!(item)]
    );
    let found = notes.search_combined("#CASETAG");
    assert_eq!(
        field_of_each(&found, "results", "itemId"),
        vec![json!(item)]
    );
}

// 削除した項目は見つからなくなる。
#[test]
fn a_deleted_item_is_no_longer_found() {
    let notes = Core::new();
    let page = notes.new_doc_page("search-deleted");
    let item = id(&notes.fill(&page, &["ghost", "stays"])[0]);
    // The item is in the index before it goes
    notes.core.tantivy.commit_pending();

    notes.delete_item(&item);

    let found = notes.search_combined("ghost");
    assert_eq!(
        field_of_each(&found, "results", "itemId"),
        Vec::<Value>::new()
    );
}
