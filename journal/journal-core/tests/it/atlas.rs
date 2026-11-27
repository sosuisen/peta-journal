//! The atlas page of a directory: its cards follow the pages of the directory, and the user
//! adds link cards, groups and callouts to it.

use journal_core::error::AppError;
use serde_json::{json, Value};

use crate::common::items::{field_of_each, id};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// A doc page in the directory. Gives its id.
    fn doc_in(&self, dir: &str, title: &str) -> String {
        self.open_doc_in(title, dir)["page"]["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    /// A directory with the pages `b` and `a` and the folder `z`, and its atlas page.
    fn atlas_dir(&self, prefix: &str) -> (String, String) {
        self.new_doc_page(prefix);
        let dir = format!("{prefix}/");
        self.doc_in(&dir, "b");
        self.doc_in(&dir, "a");
        self.doc_in(&format!("{dir}z/"), "c");
        let atlas_id = self.atlas_of(&dir);
        (dir, atlas_id)
    }

    /// Gives the page another title or folder, as the client does
    fn move_page(
        &self,
        page_id: &str,
        title: Option<&str>,
        dir: Option<&str>,
    ) -> Result<(), AppError> {
        self.core
            .journal
            .update_page(page_id, title, dir, false, &by_the_client())
            .map(|_| ())
    }
}

fn is_bad_request<T: std::fmt::Debug>(result: &Result<T, AppError>) -> bool {
    matches!(result, Err(AppError::BadRequest(_)))
}

/// The request that sets a value of a key
fn value(value_type: &str, value: &str) -> Value {
    json!({ "valueType": value_type, "value": value })
}

// アトラスページは、名前で検索しても見つからない。
#[test]
fn an_atlas_page_is_not_found_by_its_name() {
    let notes = Core::new();
    notes.new_doc_page("atlas-search");
    let dir = "atlas-search/";
    let atlas_id = notes.atlas_of(dir);

    for query in [format!("{dir}index_atlas"), "index_atlas".to_string()] {
        let found = notes.search_pages(&query);
        let ids = field_of_each(&found, "results", "pageId");
        assert!(!ids.contains(&json!(atlas_id)), "{query}: {ids:?}");
    }
}

// アトラスページは改名も移動もできない。
#[test]
fn the_atlas_page_cannot_be_renamed_or_moved() {
    let notes = Core::new();
    notes.new_doc_page("atlas-rename");
    let atlas_id = notes.atlas_of("atlas-rename/");

    let renamed = notes.move_page(&atlas_id, Some("map"), None);
    assert!(is_bad_request(&renamed), "{renamed:?}");
    let moved = notes.move_page(&atlas_id, None, Some("elsewhere/"));
    assert!(is_bad_request(&moved), "{moved:?}");
}

// 他のページはアトラスの名前を取れない。
#[test]
fn no_other_page_can_take_the_atlas_name() {
    let notes = Core::new();
    let page = notes.new_doc_page("atlas-reserved");

    let renamed = notes.move_page(&page, Some("index_atlas"), None);

    assert!(is_bad_request(&renamed), "{renamed:?}");
}

// カードを置いても、そのページは変わらない。
#[test]
fn placing_a_card_leaves_its_page_as_it_was() {
    let notes = Core::new();
    let (dir, atlas_id) = notes.atlas_dir("atlas-place");
    let a = notes.doc_in(&dir, "a");
    let before = notes.page(&a)["page"]["modifiedDate"].clone();
    let card = id(&notes.create_item(&atlas_id, "last", 1, &format!("[[{dir}a]]")));

    notes
        .set_item_property(&card, "atlas_bounds", value("text", " 24, 24, 0 ,40 "))
        .unwrap();

    assert_eq!(
        notes.item(&card)["properties"]["atlas_bounds"],
        json!("24,24,0,40")
    );
    assert_eq!(notes.page(&a)["page"]["modifiedDate"], before);
    let on_the_page = notes.core.journal.set_page_property(
        &a,
        "atlas_bounds",
        Some("bounds"),
        Some("1,2,3,4"),
        &by_the_client(),
    );
    assert!(is_bad_request(&on_the_page), "{on_the_page:?}");
    let not_bounds = notes.set_item_property(&card, "atlas_bounds", value("text", "not bounds"));
    assert!(is_bad_request(&not_bounds), "{not_bounds:?}");
}

// dateとdatetimeの値も、型に合うかが検査される。dateは年月日、datetimeはRFC 3339の日時である。
#[test]
fn a_date_and_a_datetime_are_checked_by_their_types() {
    let notes = Core::new();
    notes.define_key("checked_due_d");
    notes.define_key("checked_seen_dt");
    let page = notes.new_doc_page("checked-dates");
    let item = id(&notes.fill(&page, &["dated"])[0]);
    let set =
        |key: &str, value: &str| notes.set_item_property(&item, key, json!({ "value": value }));

    set("checked_due_d", "2026-03-01").unwrap();
    set("checked_seen_dt", "2026-03-01T10:00:00Z").unwrap();
    for (key, value) in [
        ("checked_due_d", "soon"),
        ("checked_due_d", "2026-13-01"),
        ("checked_seen_dt", "2026-03-01"),
        ("checked_seen_dt", "2026-03-01 10:00"),
    ] {
        let set = set(key, value);
        assert!(is_bad_request(&set), "{key} {value} -> {set:?}");
    }
}
