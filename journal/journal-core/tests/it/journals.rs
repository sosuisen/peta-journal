//! Journals: the pages of days, weeks, months and years, and the items moved to a day.

use journal_core::error::{AppError, AppResult};
use journal_core::journal::MovedToJournalAnswer;
use serde_json::{json, Value};

use crate::common::items::{answer, field_of_each, id, input, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

/// A core that numbers the weeks as ISO 8601 does, as the server of the tests did
fn iso_core() -> Core {
    let notes = Core::new();
    notes.core.config.set_week_numbering("iso");
    notes
}

/// The id of the page of a journal answer.
fn page_id_of(journal: &Value) -> String {
    journal["page"]["id"].as_str().unwrap().to_string()
}

impl Core {
    /// Moves items to the daily journal of `date`, the way a move key at the edge of a day
    /// does: with the body the editor of the first item has now, when it is not saved yet.
    fn move_to_journal(
        &self,
        date: &str,
        item_ids: &[&str],
        position: &str,
        body: Option<&str>,
    ) -> AppResult<MovedToJournalAnswer> {
        let mut request = json!({ "itemIds": item_ids, "position": position });
        if let Some(body) = body {
            request["body"] = json!(body);
        }
        self.core
            .journal
            .move_items_to_journal(date, input(request), &by_the_client())
    }
}

// 日のジャーナルは空の項目1つで作られ、同じ日付で再び見つかる。
#[test]
fn a_day_is_created_with_one_empty_item_and_found_again_by_its_date() {
    let notes = iso_core();

    let first = notes.open_journal("1999-06-15");
    let again = notes.open_journal("1999-06-15");

    assert_eq!(first["created"], json!(true));
    assert_eq!(first["page"]["type"], json!("journal"));
    assert_eq!(first["page"]["name"], json!("1999-06-15"));
    assert_eq!(field_of_each(&first, "items", "body"), vec![json!("")]);
    assert_eq!(again["created"], json!(false));
    assert_eq!(first["page"]["id"], again["page"]["id"]);
}

// 項目は子ごと、その日のジャーナルの末尾へ動く。要求の本文は、先頭の項目に保存される。日をまたぐ移動が1つの要求で送るものである。
#[test]
fn items_move_to_the_end_of_a_days_journal_with_the_body_the_request_carries() {
    let notes = iso_core();
    let from = page_id_of(&notes.open_journal("1999-07-01"));
    let rows = notes.fill(&from, &["go", "  go-child", "stay"]);
    let to = page_id_of(&notes.open_journal("1999-07-02"));
    notes.fill(&to, &["there"]);

    let moved = answer(
        notes
            .move_to_journal(
                "1999-07-02",
                &[&id(&rows[0]), &id(&rows[1])],
                "last",
                Some("go typed"),
            )
            .unwrap(),
    );

    assert_eq!(moved["created"], json!(false));
    assert_eq!(moved["page"]["id"], json!(to));
    assert_eq!(
        field_of_each(&moved, "items", "body"),
        vec![json!("go typed"), json!("go-child")]
    );
    assert_eq!(
        notes.outline_of(&to),
        vs(&["1:there", "1:go typed", "2:go-child"])
    );
    assert_eq!(notes.outline_of(&from), vs(&["1:stay"]));
}

// その日のジャーナルがなければ、移動が作る。作られたページは、空の項目を持たずに動かした項目だけを持ち、`created`で答え、名前で検索できる。
#[test]
fn a_move_makes_the_journal_of_the_day_when_there_is_none() {
    let notes = iso_core();
    let from = page_id_of(&notes.open_journal("1999-07-03"));
    let rows = notes.fill(&from, &["go", "stay"]);

    let moved = answer(
        notes
            .move_to_journal("1999-07-04", &[&id(&rows[0])], "last", None)
            .unwrap(),
    );

    assert_eq!(moved["created"], json!(true));
    assert_eq!(moved["page"]["name"], json!("1999-07-04"));
    let to = page_id_of(&moved);
    assert_eq!(notes.outline_of(&to), vs(&["1:go"]));
    let again = notes.open_journal("1999-07-04");
    assert_eq!(again["created"], json!(false));
    assert_eq!(page_id_of(&again), to);
    let found = notes.search_pages("1999-07-04");
    assert!(
        field_of_each(&found, "results", "pageTitle").contains(&json!("1999-07-04")),
        "{found}"
    );
}

// 移動先の日が空の1項目だけを持っていれば、その項目は消え、動かした項目だけが残る。
#[test]
fn the_single_empty_item_of_the_day_goes_when_items_move_in() {
    let notes = iso_core();
    let from = page_id_of(&notes.open_journal("1999-07-05"));
    let rows = notes.fill(&from, &["go", "stay"]);
    let to = page_id_of(&notes.open_journal("1999-07-06"));

    notes
        .move_to_journal("1999-07-06", &[&id(&rows[0])], "first", None)
        .unwrap();

    assert_eq!(notes.outline_of(&to), vs(&["1:go"]));
}

// 動かした項目が最後の項目だった日は、空の項目を1つ残す。ページは項目を1つは持つ。
#[test]
fn the_day_the_last_item_leaves_keeps_one_empty_item() {
    let notes = iso_core();
    let from = page_id_of(&notes.open_journal("1999-07-07"));
    let rows = notes.fill(&from, &["go", "  go-child"]);
    let to = page_id_of(&notes.open_journal("1999-07-08"));
    notes.fill(&to, &["there"]);

    notes
        .move_to_journal("1999-07-08", &[&id(&rows[0]), &id(&rows[1])], "first", None)
        .unwrap();

    assert_eq!(notes.outline_of(&from), vs(&["1:"]));
    assert_eq!(
        notes.outline_of(&to),
        vs(&["1:go", "2:go-child", "1:there"])
    );
}

// 断られた移動は、何も書かない。本文も保存されず、日のジャーナルも作られない。
#[test]
fn a_refused_move_to_a_day_writes_nothing() {
    let notes = iso_core();
    let from = page_id_of(&notes.open_journal("1999-07-09"));
    let rows = notes.fill(&from, &["go"]);

    let not_there = "after:00000000000000000000000000";
    let moved = notes.move_to_journal("1999-07-10", &[&id(&rows[0])], not_there, Some("typed"));

    assert!(matches!(moved, Err(AppError::BadRequest(_))), "{moved:?}");
    assert_eq!(notes.outline_of(&from), vs(&["1:go"]));
    assert_eq!(notes.open_journal("1999-07-10")["created"], json!(true));
}

// 日でないジャーナル（週、月、年）への移動は断られる。
#[test]
fn items_do_not_move_to_a_week_a_month_or_a_year() {
    let notes = iso_core();
    let from = page_id_of(&notes.open_journal("1999-07-11"));
    let rows = notes.fill(&from, &["go"]);

    let moved = notes.move_to_journal("1999-W28", &[&id(&rows[0])], "last", None);

    assert!(matches!(moved, Err(AppError::BadRequest(_))), "{moved:?}");
    assert_eq!(notes.outline_of(&from), vs(&["1:go"]));
}

// 日への移動では、移動先の日と、項目が出ていった日の変更が、クライアントに1回ずつ知らされる。
#[test]
fn the_clients_hear_once_of_each_day_a_move_changes() {
    let notes = iso_core();
    let from = page_id_of(&notes.open_journal("1999-07-12"));
    let rows = notes.fill(&from, &["go", "stay"]);
    let to = page_id_of(&notes.open_journal("1999-07-13"));

    let told = notes.times_told(&[&from, &to], || {
        notes
            .move_to_journal("1999-07-13", &[&id(&rows[0])], "last", None)
            .unwrap();
    });

    assert_eq!(told, [1, 1]);
}

// ある日付より前の、直近の日のジャーナルが見つかる。
#[test]
fn the_most_recent_day_before_a_date_is_found() {
    let notes = iso_core();
    notes.open_journal("1999-03-05");
    notes.open_journal("1999-03-10");

    let before = answer(
        notes
            .core
            .journal
            .most_recent_journal_before("1999-03-10")
            .unwrap(),
    );
    let nothing = answer(
        notes
            .core
            .journal
            .most_recent_journal_before("1999-01-01")
            .unwrap(),
    );

    assert_eq!(before["found"], json!(true));
    assert_eq!(before["page"]["name"], json!("1999-03-05"));
    assert_eq!(before["items"].as_array().unwrap().len(), 1);
    assert_eq!(nothing["found"], json!(false));
}

// 週のジャーナルは、1日に1つのリンクを持って作られる。
#[test]
fn a_week_is_created_with_one_link_per_day() {
    let notes = iso_core();

    let week = notes.open_journal("1999-W10");

    assert_eq!(week["created"], json!(true));
    let bodies = field_of_each(&week, "items", "body");
    assert_eq!(bodies.len(), 7);
    assert_eq!(bodies[0], json!("[[1999-03-08]]")); // ISO week 10 of 1999 starts on Monday the 8th
    assert_eq!(bodies[6], json!("[[1999-03-14]]"));
}

// 月のジャーナルは、その月にかかる週に1つずつのリンクを持って作られる。
#[test]
fn a_month_is_created_with_one_link_per_week_that_touches_it() {
    let notes = iso_core();

    let month = notes.open_journal("1999-03");

    assert_eq!(month["created"], json!(true));
    let bodies = field_of_each(&month, "items", "body");
    // March 1999 runs from ISO week 9 (Monday the 1st) to week 13 (Wednesday the 31st)
    assert_eq!(bodies.len(), 5);
    assert_eq!(bodies[0], json!("[[1999-W09]]"));
    assert_eq!(bodies[4], json!("[[1999-W13]]"));
}

// 年をまたぐ週へのリンクは、12月と1月のどちらの月のジャーナルでも、その週の名前である。
#[test]
fn a_week_across_new_year_is_linked_by_its_own_name_from_december_and_from_january() {
    let last_link = |month: &serde_json::Value| field_of_each(month, "items", "body").pop();
    let first_link = |month: &serde_json::Value| field_of_each(month, "items", "body").remove(0);

    // The ISO week of Monday 2024-12-30 holds Thursday 2025-01-02, so it is week 1 of 2025
    let notes = iso_core();
    assert_eq!(
        last_link(&notes.open_journal("2024-12")),
        Some(json!("[[2025-W01]]"))
    );
    assert_eq!(
        first_link(&notes.open_journal("2025-01")),
        json!("[[2025-W01]]")
    );

    // The US week of Sunday 2026-12-27 holds Friday 2027-01-01, and is week 53 of 2026
    let notes = Core::new();
    assert_eq!(
        last_link(&notes.open_journal("2026-12")),
        Some(json!("[[2026-W53]]"))
    );
    assert_eq!(
        first_link(&notes.open_journal("2027-01")),
        json!("[[2026-W53]]")
    );
    // The page of that week lists the seven days from that Sunday
    let week = notes.open_journal("2026-W53");
    let days = field_of_each(&week, "items", "body");
    assert_eq!(days[0], json!("[[2026-12-27]]"));
    assert_eq!(days[6], json!("[[2027-01-02]]"));
}

// 年のジャーナルは、1か月に1つのリンクを持って作られる。
#[test]
fn a_year_is_created_with_one_link_per_month() {
    let notes = iso_core();

    let year = notes.open_journal("1999");

    assert_eq!(year["created"], json!(true));
    let bodies = field_of_each(&year, "items", "body");
    assert_eq!(bodies.len(), 12);
    assert_eq!(bodies[0], json!("[[1999-01]]"));
    assert_eq!(bodies[11], json!("[[1999-12]]"));
}

// ある週より前の、直近の週のジャーナルが見つかる。
#[test]
fn the_most_recent_week_before_a_week_is_found() {
    let notes = iso_core();
    notes.open_journal("1999-W20");
    notes.open_journal("1999-W22");

    let before = answer(
        notes
            .core
            .journal
            .most_recent_weekly_journal_before("1999-W22")
            .unwrap(),
    );

    assert_eq!(before["found"], json!(true));
    assert_eq!(before["page"]["name"], json!("1999-W20"));
}
