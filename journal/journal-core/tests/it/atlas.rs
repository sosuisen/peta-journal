//! The atlas page of a directory: its cards follow the pages of the directory, and the user
//! adds link cards, groups and callouts to it.

use journal_core::error::AppError;
use journal_core::journal::ItemsAnswer;
use serde_json::{json, Value};

use crate::common::items::{answer, field_of_each, id, input, outline, vs};
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

/// The items of an answer of the atlas
fn rows_of(changed: Result<ItemsAnswer, AppError>) -> Vec<Value> {
    answer(changed.unwrap())["items"]
        .as_array()
        .unwrap()
        .clone()
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

// リンクカードは名前の順に置かれ、リンク先の変更と削除ができる。
#[test]
fn a_link_card_takes_its_place_by_its_name_and_can_be_relinked_and_removed() {
    let notes = Core::new();
    let (dir, atlas_id) = notes.atlas_dir("atlas-link");
    let journal = &notes.core.journal;

    let rows = rows_of(journal.add_atlas_link(
        &atlas_id,
        input(json!({ "target": "[[elsewhere/aa]]", "bounds": "1,2,0,40" })),
    ));
    assert_eq!(
        outline(&rows),
        vs(&[
            &format!("1:[[{dir}z/index]]"),
            &format!("1:[[{dir}a]]"),
            "1:[[elsewhere/aa]]",
            &format!("1:[[{dir}b]]"),
        ])
    );
    let link = rows[2].clone();
    assert_eq!(link["properties"]["atlas_bounds"], json!("1,2,0,40"));

    let rows = rows_of(journal.set_atlas_link(
        &atlas_id,
        &id(&link),
        input(json!({ "target": "far/zz" })),
    ));
    assert_eq!(outline(&rows).last().unwrap(), "1:[[far/zz]]");
    assert_eq!(rows.last().unwrap()["id"], link["id"]);

    journal.remove_atlas_item(&atlas_id, &id(&link)).unwrap();
    assert_eq!(notes.outline_of(&atlas_id).len(), 3);
}

// カードに落としたカードはグループになり、グループは残り1枚で終わる。
#[test]
fn cards_dropped_on_a_card_make_a_group_that_ends_with_its_second_last_card() {
    let notes = Core::new();
    let (dir, atlas_id) = notes.atlas_dir("atlas-group");
    let a = notes.doc_in(&dir, "a");
    let b = notes.doc_in(&dir, "b");
    let journal = &notes.core.journal;
    let link = rows_of(journal.add_atlas_link(
        &atlas_id,
        input(json!({ "target": "elsewhere/l", "bounds": "0,0,0,40" })),
    ))
    .into_iter()
    .find(|r| r["body"] == "[[elsewhere/l]]")
    .unwrap();
    let link = id(&link);

    let rows = rows_of(
        journal.group_atlas_cards(&atlas_id, input(json!({ "cards": [b, link], "target": a }))),
    );
    assert_eq!(
        outline(&rows),
        vs(&[
            "1:",
            &format!("2:[[{dir}a]]"),
            &format!("2:[[{dir}b]]"),
            "2:[[elsewhere/l]]",
            &format!("1:[[{dir}z/index]]"),
        ])
    );
    let group = id(&rows[0]);
    assert_eq!(rows[0]["properties"]["atlas_type"], json!("group"));

    let rows = rows_of(journal.set_atlas_group_title(
        &atlas_id,
        &group,
        input(json!({ "side": "left", "title": "Tools" })),
    ));
    assert_eq!(outline(&rows)[0], "1:Tools");
    assert_eq!(rows[0]["properties"]["atlas_side"], json!("left"));

    journal
        .ungroup_atlas_cards(&atlas_id, input(json!({ "cards": [link] })))
        .unwrap();
    let rows = rows_of(journal.ungroup_atlas_cards(&atlas_id, input(json!({ "cards": [b] }))));
    assert!(rows.iter().all(|r| r["level"] == 1), "{rows:?}");
    assert!(rows.iter().all(|r| r["id"] != json!(group)), "{rows:?}");
}

// コールアウトは、親の下にまとめて末尾に置かれる。
#[test]
fn the_callouts_go_last_under_their_parent() {
    let notes = Core::new();
    let (_, atlas_id) = notes.atlas_dir("atlas-callout");
    let journal = &notes.core.journal;

    let rows = rows_of(journal.add_atlas_callout(
        &atlas_id,
        input(json!({ "bounds": "10,20,0,60", "side": "left", "text": "Hello" })),
    ));
    let n = rows.len();
    assert_eq!(outline(&rows[n - 2..]), vs(&["1:callout", "2:Hello"]));
    let callout = rows[n - 1].clone();
    assert_eq!(callout["properties"]["atlas_type"], json!("callout"));
    assert_eq!(callout["properties"]["atlas_side"], json!("left"));
    assert_eq!(callout["properties"]["atlas_bounds"], json!("10,20,0,60"));

    // A new text is a change of the item: it is parsed again and dated anew
    let before = notes.item(&id(&callout));
    notes.clock.advance(5);
    let rows = rows_of(journal.set_atlas_callout(
        &atlas_id,
        &id(&callout),
        input(json!({ "side": "bottom", "text": "## Bye" })),
    ));
    let renamed = rows.last().unwrap();
    assert_eq!(outline(&rows).last().unwrap(), "2:## Bye");
    assert!(
        renamed["types"]
            .as_array()
            .unwrap()
            .contains(&json!("heading")),
        "{renamed}"
    );
    let after = notes.item(&id(&callout));
    assert!(after["ast"].as_str().unwrap().contains("Bye"));
    assert!(
        after["modifiedDate"].as_i64() > before["modifiedDate"].as_i64(),
        "{before} {after}"
    );

    let rows = rows_of(journal.set_atlas_callout(
        &atlas_id,
        &id(&callout),
        input(json!({ "side": "bottom", "text": " " })),
    ));
    assert_eq!(rows.len(), n - 2, "{rows:?}");
}

// コールアウトの文が閉じていないブロックで終われば、コールアウトは足されない（ADR 050）。
#[test]
fn a_callout_with_an_unclosed_block_is_refused() {
    let notes = Core::new();
    let (_, atlas_id) = notes.atlas_dir("atlas-callout-unclosed");
    let before = notes.outline_of(&atlas_id);

    let added = notes.core.journal.add_atlas_callout(
        &atlas_id,
        input(json!({ "bounds": "10,20,0,60", "side": "left", "text": "```\nabc" })),
    );

    assert!(is_bad_request(&added), "{added:?}");
    assert_eq!(notes.outline_of(&atlas_id), before);
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
