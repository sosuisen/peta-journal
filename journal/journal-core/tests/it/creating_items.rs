//! Making items: where they go, the indexes that keep them in order, and what their bodies
//! give them.

use journal_core::journal::CreateItemInput;
use serde_json::{json, Value};

use crate::common::items::{answer, has_type, id, input, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// The indexes of the items of a page, in order
    fn idxs_of(&self, page_id: &str) -> Vec<String> {
        self.items(page_id)
            .iter()
            .map(|i| i["idx"].as_str().unwrap().to_string())
            .collect()
    }

    /// Writes the indexes straight into the database, as a database from before would have
    /// them
    fn set_indexes(&self, rows: &[Value], indexes: &[&str]) {
        let conn = self.core.db.conn().unwrap();
        for (row, idx) in rows.iter().zip(indexes) {
            conn.execute(
                "UPDATE items SET idx = ? WHERE id = ?",
                rusqlite::params![idx, id(row)],
            )
            .unwrap();
        }
    }

    /// Gives the items the indexes of the scheme before this one
    fn give_old_indexes(&self, rows: &[Value]) {
        self.set_indexes(rows, &["z0", "z1", "z2", "z3"]);
    }

    fn modified_date(&self, item_id: &str) -> i64 {
        self.core
            .journal
            .get_item(item_id)
            .unwrap()
            .item
            .modified_date
    }
}

/// An index at the limit, and one right after it: nothing short fits between the two
fn tight_gap() -> (String, String) {
    let at_limit = format!("a0{}", "V".repeat(14));
    assert_eq!(at_limit.len(), 16);
    (at_limit.clone(), format!("{at_limit}W"))
}

// 項目は、先頭、末尾、または別の項目の後ろに作られる。
#[test]
fn items_go_first_last_or_after_another_item() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-create");
    let rows = notes.fill(&page, &["a", "b"]);

    notes.create_item(&page, "first", 1, "x");
    notes.create_item(&page, "last", 1, "y");
    notes.create_item(&page, &format!("after:{}", id(&rows[0])), 2, "z");

    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:x", "1:a", "2:z", "1:b", "1:y"])
    );
}

// 狭い隙間に項目を動かしてインデックスが長くなると、その周りだけが付け直される。
#[test]
fn moving_an_item_into_a_tight_gap_reindexes_around_the_gap() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-move-tight");
    let rows = notes.fill(&page, &["a", "b", "c"]);
    let (a, b) = tight_gap();
    notes.set_indexes(&rows, &[&a, &b, "a1"]);

    notes.move_items(
        &[&id(&rows[2])],
        &page,
        &format!("after:{}", id(&rows[0])),
        None,
    );

    assert_eq!(notes.outline_of(&page), vs(&["1:a", "1:c", "1:b"]));
    let idxs = notes.idxs_of(&page);
    assert!(idxs.iter().all(|idx| idx.len() <= 16), "{idxs:?}");
}

// 長いインデックスの隣に旧い形のインデックスがあれば、ページ全体が付け直される。
#[test]
fn an_index_of_the_old_scheme_next_to_a_long_one_makes_the_whole_page_reindex() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-old-next-to-long");
    let rows = notes.fill(&page, &["a", "b", "c"]);
    let (a, b) = tight_gap();
    notes.set_indexes(&rows, &[&a, &b, "z5"]);

    notes.create_item(&page, &format!("after:{}", id(&rows[0])), 1, "x");

    assert_eq!(notes.outline_of(&page), vs(&["1:a", "1:x", "1:b", "1:c"]));
    assert_eq!(notes.idxs_of(&page), vs(&["a0", "a1", "a2", "a3"]));
}

// 旧い形のインデックスを持つページに項目を足すか動かすと、ページの全項目が付け直される。
#[test]
fn a_page_with_indexes_of_the_old_scheme_is_reindexed_when_an_item_is_added_or_moved() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-old-idx");
    let rows = notes.fill(&page, &["a", "b"]);
    notes.give_old_indexes(&rows);

    notes.create_item(&page, &format!("after:{}", id(&rows[0])), 1, "x");

    assert_eq!(notes.outline_of(&page), vs(&["1:a", "1:x", "1:b"]));
    assert_eq!(notes.idxs_of(&page), vs(&["a0", "a0V", "a1"]));

    let rows = notes.items(&page);
    notes.give_old_indexes(&rows);
    notes.move_items(&[&id(&rows[2])], &page, "first", None);

    assert_eq!(notes.outline_of(&page), vs(&["1:b", "1:a", "1:x"]));
    assert_eq!(notes.idxs_of(&page), vs(&["Zz", "a0", "a1"]));
}

// 同じ隙間に入れ続けてインデックスが長くなると、その周りだけが付け直される。
#[test]
fn indexes_grown_long_in_one_gap_are_reindexed_around_that_gap_only() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-long-idx");
    let rows = notes.fill(&page, &["a", "b"]);
    let before = notes.idxs_of(&page);
    let b_before = notes.modified_date(&id(&rows[1]));

    // Each new item goes right under "a", between it and the one before
    for n in 0..90 {
        notes.create_item(
            &page,
            &format!("after:{}", id(&rows[0])),
            1,
            &format!("x{n}"),
        );
    }

    let outline = notes.outline_of(&page);
    assert_eq!(outline.len(), 92);
    assert_eq!(outline[0], "1:a");
    assert_eq!(outline[1], "1:x89");
    assert_eq!(outline[90], "1:x0");
    assert_eq!(outline[91], "1:b");
    let idxs = notes.idxs_of(&page);
    assert_eq!(idxs[0], before[0]);
    assert_eq!(idxs[91], before[1]);
    assert!(idxs.iter().all(|idx| idx.len() <= 16), "{idxs:?}");
    // The items reindexed were not changed: b keeps its index and its modified date
    assert_eq!(notes.modified_date(&id(&rows[1])), b_before);
}

// クライアントが付けたIDがそのまま使われる。
#[test]
fn a_client_id_is_kept() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-client-id");
    // What the frontend makes up for an optimistic item (npm hmtid default layout)
    let made_up = hmtid::Generator::new('_', false).generate().to_string();

    let created = notes
        .core
        .journal
        .create_item(
            input::<CreateItemInput>(
                json!({ "pageId": page, "position": "last", "level": 1, "body": "mine", "id": made_up }),
            ),
            &by_the_client(),
        )
        .unwrap();

    assert_eq!(created.item.id, made_up);
}

// タスクの接頭辞で始まる本文で作った項目はタスクであり、本文に接頭辞は残らない。
#[test]
fn an_item_made_with_a_task_prefix_is_a_task_without_the_prefix() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-task-prefix");

    let created = notes.create_item(&page, "last", 1, "- [ ] buy milk");

    assert_eq!(created["body"], json!("buy milk"));
    assert_eq!(created["types"], json!(["task"]));
}

// 本文のハッシュタグがtagsプロパティとして返る。
#[test]
fn a_hashtag_in_the_body_comes_back_as_a_tag_property() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-hashtag");

    let created = notes.create_item(&page, "last", 1, "remember #urgent");
    let updated = notes.update_item(
        &id(&created),
        json!({ "body": "## remember #urgent #later" }),
    );

    assert_eq!(created["properties"]["tags"], json!(["urgent"]));
    assert_eq!(updated["properties"]["tags"], json!(["urgent", "later"]));
    // The body is parsed into an AST and types
    assert!(has_type(&updated, "heading"), "{updated}");
    let stored = answer(notes.core.journal.get_item(&id(&created)).unwrap());
    assert!(
        stored["item"]["ast"]
            .as_str()
            .unwrap()
            .contains("\"heading\""),
        "{stored}"
    );
}
