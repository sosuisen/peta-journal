//! Pasting a text into an item at the cursor: one piece goes into the body, several make
//! items below it.

use journal_core::error::{AppError, AppResult};
use journal_core::journal::PasteAnswer;
use serde_json::{json, Value};

use crate::common::items::{answer, field_of_each, id, input, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Pastes into an item the way an editor does: with the body and the cursor it has now
    fn paste(
        &self,
        item_id: &str,
        body: &str,
        cursor: usize,
        text: &str,
        mode: &str,
    ) -> AppResult<PasteAnswer> {
        self.core.journal.paste(
            item_id,
            input(json!({ "body": body, "cursor": cursor, "text": text, "mode": mode })),
            &by_the_client(),
        )
    }

    /// Pastes as `paste` does, and gives the answer as the JSON a client gets
    fn pasted(&self, item_id: &str, body: &str, cursor: usize, text: &str, mode: &str) -> Value {
        answer(self.paste(item_id, body, cursor, text, mode).unwrap())
    }
}

// 1つの断片は、カーソルの位置で項目の本文に入り、カーソルはその直後に来る。
#[test]
fn one_pasted_piece_goes_into_the_body_at_the_cursor() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-one");
    let rows = notes.fill(&page, &["ab"]);

    let pasted = notes.pasted(&id(&rows[0]), "ab", 1, "X", "auto");

    assert_eq!(field_of_each(&pasted, "items", "body"), vec![json!("aXb")]);
    assert_eq!(
        pasted["cursor"],
        json!({ "itemId": id(&rows[0]), "position": 2 })
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:aXb"]));
}

// 複数の断片は下に新しい項目になり、最後の断片がカーソルより後ろの文字を受け取る。
#[test]
fn several_pasted_pieces_become_items_below_and_the_last_takes_the_text_after_the_cursor() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-several");
    let rows = notes.fill(&page, &["ab", "  child", "next"]);

    let pasted = notes.pasted(&id(&rows[0]), "ab", 1, "X\n\nY\n\nZ", "auto");

    assert_eq!(
        field_of_each(&pasted, "items", "body"),
        vec![json!("aX"), json!("Y"), json!("Zb")]
    );
    let created = &pasted["items"][2];
    assert_eq!(
        pasted["cursor"],
        json!({ "itemId": created["id"], "position": 1 })
    );
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:aX", "1:Y", "1:Zb", "2:child", "1:next"])
    );
}

// メタデータの行が決める階層は、最初の断片からの差として、貼り付け先の項目の階層に足される。
#[test]
fn pasted_levels_are_added_to_the_level_of_the_item_as_the_distance_from_the_first_piece() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-levels");
    let rows = notes.fill(&page, &["top", "  host"]);

    let flat = "<!-- @@ -->\nA\n\n<!-- @@@ -->\nB\n\n<!-- @@@@ -->\nC";
    notes.pasted(&id(&rows[1]), "host", 4, flat, "auto");

    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:top", "2:hostA", "3:B", "4:C"])
    );
}

// 最初の断片より浅い断片は、貼り付け先の項目と同じ階層になる。
#[test]
fn a_pasted_piece_above_the_first_one_gets_the_level_of_the_item() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-shallow");
    let rows = notes.fill(&page, &["top", "  host"]);

    let flat = "<!-- @@@ -->\nA\n\n<!-- @@ -->\nB";
    notes.pasted(&id(&rows[1]), "host", 4, flat, "auto");

    assert_eq!(notes.outline_of(&page), vs(&["1:top", "2:hostA", "2:B"]));
}

// 貼り付け先の本文は、保存済みのものではなく、要求が運んできたものである。まだ保存されていない文字も残る。
#[test]
fn the_paste_goes_into_the_body_the_request_carries_not_the_stored_one() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-unsaved");
    let rows = notes.fill(&page, &["saved"]);

    notes.pasted(&id(&rows[0]), "saved and typed", 15, "!", "auto");

    assert_eq!(notes.outline_of(&page), vs(&["1:saved and typed!"]));
}

// `auto`では、`- `で始まる文字列がツリー形式として読まれ、字下げが階層になる。
#[test]
fn a_paste_in_auto_mode_reads_text_that_starts_with_a_dash_as_tree_markdown() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-tree");
    let rows = notes.fill(&page, &["host"]);

    notes.pasted(&id(&rows[0]), "host", 4, "- A\n  - B\n- C", "auto");

    assert_eq!(notes.outline_of(&page), vs(&["1:hostA", "2:B", "1:C"]));
}

// `raw`では、文字列は変換も分割もされずに、そのまま項目の本文に入る。
#[test]
fn a_raw_paste_puts_the_text_into_the_body_as_it_is() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-raw");
    let rows = notes.fill(&page, &["ab"]);

    let pasted = notes.pasted(&id(&rows[0]), "ab", 1, "- X\n\nY", "raw");

    assert_eq!(notes.outline_of(&page), vs(&["1:a- X\n\nYb"]));
    assert_eq!(
        pasted["cursor"],
        json!({ "itemId": id(&rows[0]), "position": 7 })
    );
}

// `flat`では、`- `で始まる文字列もツリー形式として読まれず、Flat Markdownとして分かれる。
#[test]
fn a_flat_paste_does_not_read_a_dash_as_tree_markdown() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-flat");
    let rows = notes.fill(&page, &["host"]);

    let flat = "- A\n- B\n\n<!-- @@ -->\nC";
    notes.pasted(&id(&rows[0]), "host", 4, flat, "flat");

    assert_eq!(notes.outline_of(&page), vs(&["1:host- A\n- B", "2:C"]));
}

// `auto`では、ブロックの中に貼った文字列は分割されない。コードブロックでも、複数行の引用でも同じである。
#[test]
fn a_paste_in_auto_mode_inside_a_block_is_not_split() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-fence");
    let rows = notes.fill(&page, &["x"]);

    for (body, cursor) in [("```\ncode\n```", 8), (">>>\nquote\n>>>", 9)] {
        let pasted = notes.pasted(&id(&rows[0]), body, cursor, "X\n\nY", "auto");

        let (before, after) = body.split_at(cursor);
        assert_eq!(
            field_of_each(&pasted, "items", "body"),
            vec![json!(format!("{before}X\n\nY{after}"))]
        );
        assert_eq!(
            pasted["cursor"],
            json!({ "itemId": id(&rows[0]), "position": cursor + 4 })
        );
    }
}

// 途中の項目が作れないと、貼り付けの全体が取り消される。貼り付け先の本文も変わらない。
#[test]
fn a_paste_that_cannot_create_one_of_its_items_changes_nothing() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-atomic");
    let rows = notes.fill(&page, &["ab"]);

    // The second piece is two levels below the first, which no item can be
    let flat = "X\n\n<!-- @@@ -->\nY";
    let pasted = notes.paste(&id(&rows[0]), "ab", 1, flat, "flat");

    assert!(matches!(pasted, Err(AppError::BadRequest(_))), "{pasted:?}");
    assert_eq!(notes.outline_of(&page), vs(&["1:ab"]));
}

// 空の文字列を貼り付けると、項目は増えず、要求が運んできた本文だけが保存される。
#[test]
fn pasting_nothing_saves_the_body_the_request_carries_and_adds_no_item() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-empty");
    let rows = notes.fill(&page, &["saved"]);

    let pasted = notes.pasted(&id(&rows[0]), "saved and typed", 5, "", "auto");

    assert_eq!(
        pasted["cursor"],
        json!({ "itemId": id(&rows[0]), "position": 5 })
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:saved and typed"]));
}

// 項目をいくつ作る貼り付けでも、ページの変更は1回だけクライアントに知らされる。
#[test]
fn the_clients_hear_of_the_page_once_however_many_items_a_paste_creates() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-event");
    let rows = notes.fill(&page, &["ab"]);

    let told = notes.times_told(&[&page], || {
        notes.pasted(&id(&rows[0]), "ab", 1, "X\n\nY\n\nZ", "auto");
    });

    assert_eq!(told, [1]);
}

// トランスクルードされている項目に貼り付けると、その写しを見せているページの変更も知らされる。
#[test]
fn the_clients_hear_of_the_pages_that_transclude_the_item_pasted_into() {
    let notes = Core::new();
    let (_, source, page, _) = notes.transcluded_rows("items-paste-transcluder", &["ab"]);

    let told = notes.times_told(&[&page], || {
        notes.pasted(&id(&source[0]), "ab", 1, "X", "auto");
    });

    assert_ne!(told, [0]);
    assert_eq!(notes.items(&page)[0]["body"], json!("aXb"));
}

// トランスクルージョンへの貼り付けは、元の項目への貼り付けになる。入力が元の項目に保存されるのと同じである。
#[test]
fn a_paste_into_a_transclusion_goes_into_its_source_item() {
    let notes = Core::new();
    let (source_page, source, page, host) =
        notes.transcluded_rows("items-paste-into-copy", &["ab"]);

    let pasted = notes.pasted(&host, "ab", 1, "X", "auto");

    assert_eq!(
        field_of_each(&pasted, "items", "id"),
        vec![json!(id(&source[0]))]
    );
    assert_eq!(
        pasted["cursor"],
        json!({ "itemId": id(&source[0]), "position": 2 })
    );
    assert_eq!(notes.outline_of(&source_page), vs(&["1:aXb"]));
    // The copy the transclusion keeps follows its source
    assert_eq!(notes.outline_of(&page), vs(&["1:aXb"]));
}

// トランスクルージョンへの貼り付けが作る項目は、必ず元の項目のページに、元の項目の階層を基準に作られる。写している側のページには作られない。
#[test]
fn the_items_a_paste_into_a_transclusion_makes_go_on_the_source_page() {
    let notes = Core::new();
    let source_page = notes.new_doc_page("items-paste-copy-several-source");
    let source = notes.fill(&source_page, &["ab", "next"]);
    let page = notes.new_doc_page("items-paste-copy-several");
    let host = id(&notes.fill(&page, &["top", "  placeholder"])[1]);
    notes.transclude(&host, &id(&source[0]));

    notes.pasted(&host, "ab", 1, "X\n\nY", "auto");

    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:aX", "1:Yb", "1:next"])
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:top", "2:aX"]));
}

// トランスクルージョンへの貼り付けは、それを見せているページの変更としても知らされる。
#[test]
fn the_clients_hear_of_the_page_of_the_transclusion_pasted_into() {
    let notes = Core::new();
    let (_, _, page, host) = notes.transcluded_rows("items-paste-copy-told", &["ab"]);

    let told = notes.times_told(&[&page], || {
        notes.pasted(&host, "ab", 1, "X", "auto");
    });

    assert_ne!(told, [0]);
}

// もう無い項目への貼り付けは断られる。ほかの項目の操作と同じ答えである。
#[test]
fn a_paste_into_an_item_that_is_gone_is_refused() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-paste-gone");
    let rows = notes.fill(&page, &["ab", "other"]);
    notes.delete_item(&id(&rows[0]));

    let pasted = notes.paste(&id(&rows[0]), "ab", 1, "X", "auto");

    assert!(matches!(pasted, Err(AppError::BadRequest(_))), "{pasted:?}");
    assert_eq!(notes.outline_of(&page), vs(&["1:other"]));
}
