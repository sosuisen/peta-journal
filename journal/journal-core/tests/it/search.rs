//! Search: page names, the text of items, tags, and the filters of the combined search.

use chrono::TimeZone;
use journal_core::clock::Clock;
use serde_json::{json, Value};

use crate::common::items::{answer, field_of_each, id};
use crate::common::Core;

impl Core {
    /// Sets the clock of the core to the local time given, such as `2026-03-12T10:00:00`. The
    /// date filters read a day in the time zone of the OS, so the tests pass in any zone.
    fn set_time(&self, local: &str) {
        let naive = chrono::NaiveDateTime::parse_from_str(local, "%Y-%m-%dT%H:%M:%S").unwrap();
        let at = chrono::Local
            .from_local_datetime(&naive)
            .earliest()
            .unwrap()
            .timestamp_millis();
        self.clock.advance(at - self.clock.now_millis());
    }

    /// The ids of the items the combined search finds, in the order it gives them.
    fn found_items(&self, query: &str) -> Vec<Value> {
        field_of_each(&self.search_combined(query), "results", "itemId")
    }

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

/// The ids of the pages the page name search finds, in its order.
fn page_ids(found: &Value) -> Vec<Value> {
    field_of_each(found, "results", "pageId")
}

// フォルダー名で検索すると、そのフォルダーのindexページが、名前がその語で始まるページより先に出る。
#[test]
fn a_folder_name_finds_the_index_page_of_the_folder_first() {
    let notes = Core::new();
    let index = notes.open_doc_in("index", "deep/garden/")["page"]["id"].clone();
    // Changed last, so it would come first by its date
    let other = notes.new_doc_page("garden-tools");

    let found = page_ids(&notes.search_pages("garden"));

    assert_eq!(found, vec![index, json!(other)]);
}

// フォルダーを含めたパスの前半で検索すると、そのページが見つかる。
#[test]
fn the_start_of_a_path_with_its_folder_finds_the_page() {
    let notes = Core::new();
    let page = notes.open_doc_in("idea", "notes/")["page"]["id"].clone();

    assert!(page_ids(&notes.search_pages("notes/ide")).contains(&page));
}

// 名前の途中の文字でも、そのページが見つかる。
#[test]
fn a_part_in_the_middle_of_a_name_finds_the_page() {
    let notes = Core::new();
    let page = notes.new_doc_page("weekly-review");

    assert_eq!(page_ids(&notes.search_pages("kly-rev")), vec![json!(page)]);
}

// 日本語の名前の途中の文字でも、そのページが見つかる。検索の索引は日本語の語を切り出さないので、ページのパスの部分一致で探す。
#[test]
fn a_part_of_a_japanese_name_finds_the_page() {
    let notes = Core::new();
    let page = notes.new_doc_page("旅の日記帳");

    assert_eq!(page_ids(&notes.search_pages("日記")), vec![json!(page)]);
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

// `in:`の絞り込みでは、フォルダーと名前をつないだパスがその文字で始まるページの項目だけが見つかる。
#[test]
fn in_finds_only_the_items_of_the_pages_whose_folder_and_name_start_with_it() {
    let notes = Core::new();
    let inside = notes.open_doc_in("plan", "project/")["page"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let outside = notes.open_doc_in("plan", "other/")["page"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let wanted = id(&notes.fill(&inside, &["inside"])[0]);
    notes.fill(&outside, &["outside"]);

    assert_eq!(notes.found_items("in:project/pl"), vec![json!(wanted)]);
}

// `key=value`の絞り込みでは、そのプロパティにその値を持つ項目だけが見つかる。
#[test]
fn a_key_and_value_find_only_the_items_with_that_value() {
    let notes = Core::new();
    notes.define_key("status");
    let page = notes.new_doc_page("search-property");
    let items = notes.fill(&page, &["done one", "open one", "no status"]);
    for (item, status) in [(&items[0], "done"), (&items[1], "open")] {
        notes
            .set_item_property(
                &id(item),
                "status",
                json!({ "valueType": "text", "value": status }),
            )
            .unwrap();
    }

    assert_eq!(notes.found_items("status=done"), vec![json!(id(&items[0]))]);
}

/// A page whose items were made at the times given, one item for each, in that order. The
/// date filters read a date in Asia/Tokyo for now (#854), so the times are given there.
fn items_made_at(notes: &Core, times: &[&str]) -> Vec<Value> {
    notes.set_time(times[0]);
    let page = notes.new_doc_page("search-dates");
    let mut items = notes.fill(&page, &[times[0]]);
    for time in &times[1..] {
        notes.set_time(time);
        let after = format!("after:{}", id(items.last().unwrap()));
        items.push(notes.create_item(&page, &after, 1, time));
    }
    items.iter().map(|i| json!(id(i))).collect()
}

impl Core {
    /// The items of the page of `items_made_at` that the date filter finds. The core makes
    /// items of its own at the real time, which a filter such as `>2026-04-03` finds too.
    fn found_on_date_page(&self, filter: &str) -> Vec<Value> {
        self.found_items(&format!("{filter} in:search-dates"))
    }
}

// `created:`に日を書くと、その日に作った項目だけが見つかる。
#[test]
fn created_with_a_day_finds_the_items_made_that_day() {
    let notes = Core::new();
    let items = items_made_at(
        &notes,
        &[
            "2026-04-04T23:59:00",
            "2026-04-05T00:00:00",
            "2026-04-05T23:59:00",
            "2026-04-06T00:00:00",
        ],
    );

    assert_eq!(
        notes.found_on_date_page("created:2026-04-05"),
        vec![items[1].clone(), items[2].clone()]
    );
}

// `created:`に月を書くと、その月に作った項目だけが見つかる。
#[test]
fn created_with_a_month_finds_the_items_made_that_month() {
    let notes = Core::new();
    let items = items_made_at(
        &notes,
        &[
            "2026-03-31T23:59:00",
            "2026-04-01T00:00:00",
            "2026-04-30T23:59:00",
            "2026-05-01T00:00:00",
        ],
    );

    assert_eq!(
        notes.found_on_date_page("created:2026-04"),
        vec![items[1].clone(), items[2].clone()]
    );
}

// 12月を書くと、その年の12月の項目だけが見つかり、翌年の1月の項目は見つからない。
#[test]
fn created_with_december_ends_at_the_new_year() {
    let notes = Core::new();
    let items = items_made_at(
        &notes,
        &[
            "2026-11-30T23:59:00",
            "2026-12-31T23:59:00",
            "2027-01-01T00:00:00",
        ],
    );

    assert_eq!(
        notes.found_on_date_page("created:2026-12"),
        vec![items[1].clone()]
    );
}

// `..`は始めの日から終わりの日まで、`>`はその日のあと、`<`はその日の前に作った項目を見つける。
#[test]
fn a_range_after_and_before_find_the_items_made_in_them() {
    let notes = Core::new();
    let items = items_made_at(
        &notes,
        &[
            "2026-04-01T12:00:00",
            "2026-04-02T12:00:00",
            "2026-04-03T12:00:00",
            "2026-04-04T12:00:00",
        ],
    );

    assert_eq!(
        notes.found_on_date_page("created:2026-04-02..2026-04-03"),
        vec![items[1].clone(), items[2].clone()]
    );
    assert_eq!(
        notes.found_on_date_page("created:>2026-04-03"),
        vec![items[3].clone()]
    );
    assert_eq!(
        notes.found_on_date_page("created:<2026-04-02"),
        vec![items[0].clone()]
    );
}

// `modified:`は、作った日ではなく、最後に変えた日で項目を見つける。
#[test]
fn modified_finds_the_items_by_the_day_they_were_last_changed() {
    let notes = Core::new();
    let items = items_made_at(&notes, &["2026-04-01T12:00:00", "2026-04-01T12:01:00"]);
    notes.set_time("2026-04-09T12:00:00");
    notes.update_item(items[0].as_str().unwrap(), json!({ "body": "changed" }));

    assert_eq!(
        notes.found_on_date_page("modified:2026-04-09"),
        vec![items[0].clone()]
    );
    assert_eq!(
        notes.found_on_date_page("created:2026-04-09"),
        Vec::<Value>::new()
    );
}
