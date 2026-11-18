//! Splitting an item in two at the cursor, as Enter in an editor does, also when the item is
//! a transclusion or an item a transclusion shows.

use journal_core::error::AppError;
use journal_core::journal::SplitAnswer;
use serde_json::{json, Value};

use crate::common::items::{answer, id, input, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Splits an item the way Enter in an editor does: with the body and the cursor it has
    /// now
    fn split(&self, item_id: &str, body: &str, cursor: usize) -> Result<SplitAnswer, AppError> {
        self.split_with(item_id, json!({ "body": body, "cursor": cursor }))
    }

    /// Splits an item of another page that the transclusion `root_id` shows below itself
    fn split_shown_by(
        &self,
        root_id: &str,
        item_id: &str,
        body: &str,
        cursor: usize,
    ) -> Result<SplitAnswer, AppError> {
        self.split_with(
            item_id,
            json!({ "body": body, "cursor": cursor, "rootId": root_id }),
        )
    }

    fn split_with(&self, item_id: &str, request: Value) -> Result<SplitAnswer, AppError> {
        self.core
            .journal
            .split(item_id, input(request), &by_the_client())
    }

    /// Splits as `split` does, and gives the answer as the JSON a client gets
    fn splits(&self, item_id: &str, body: &str, cursor: usize) -> Value {
        answer(self.split(item_id, body, cursor).unwrap())
    }

    /// Splits as `split_shown_by` does, and gives the answer as the JSON a client gets
    fn splits_shown_by(&self, root_id: &str, item_id: &str, body: &str, cursor: usize) -> Value {
        answer(self.split_shown_by(root_id, item_id, body, cursor).unwrap())
    }

    /// Collapses an item, as the toggle beside it does
    fn collapse(&self, item_id: &str) {
        self.update_item(item_id, json!({ "collapsed": true }));
    }
}

fn is_refused(split: &Result<SplitAnswer, AppError>) -> bool {
    matches!(split, Err(AppError::BadRequest(_)))
}

// 途中での分割では、カーソルより前の文が元の項目に残り、後ろの文が同じ階層の新しい項目として直後に入る。
#[test]
fn a_split_in_the_middle_leaves_the_text_before_the_cursor_and_puts_the_rest_in_a_new_item_below() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-middle");
    let rows = notes.fill(&page, &["parent", "  alpha beta", "  next"]);

    let split = notes.splits(&id(&rows[1]), "alpha beta", 6);

    assert_eq!(split["item"]["id"], json!(id(&rows[1])));
    assert_eq!(split["item"]["body"], json!("alpha "));
    assert_eq!(split["created"]["body"], json!("beta"));
    assert_eq!(split["created"]["level"], json!(2));
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:parent", "2:alpha ", "2:beta", "2:next"])
    );
}

// 行頭での分割では、空の項目が同じ階層で直前に入り、元の項目は本文を保つ。
#[test]
fn a_split_at_the_start_puts_an_empty_item_above_and_the_item_keeps_its_body() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-start");
    let rows = notes.fill(&page, &["parent", "  alpha", "  next"]);

    let split = notes.splits(&id(&rows[1]), "alpha", 0);

    assert_eq!(split["item"]["body"], json!("alpha"));
    assert_eq!(split["created"]["body"], json!(""));
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:parent", "2:", "2:alpha", "2:next"])
    );
}

// 開いている親を分割すると、新しい項目はその最初の子になる。親の直後に同じ階層で置くと、子を奪ってしまうためである。
#[test]
fn a_split_of_an_open_parent_puts_the_new_item_in_as_its_first_child() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-parent");
    let rows = notes.fill(&page, &["parent", "  child", "next"]);

    notes.splits(&id(&rows[0]), "parent", 6);

    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:parent", "2:", "2:child", "1:next"])
    );
}

// 折りたたまれた親を分割すると、新しい項目は隠れている子孫の後ろに、親と同じ階層で入る。
#[test]
fn a_split_of_a_collapsed_parent_puts_the_new_item_after_its_hidden_descendants() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-collapsed");
    let rows = notes.fill(&page, &["parent", "  child", "    grandchild", "next"]);
    notes.collapse(&id(&rows[0]));

    notes.splits(&id(&rows[0]), "parent", 3);

    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:par", "2:child", "3:grandchild", "1:ent", "1:next"])
    );
}

// トランスクルードされている項目を分割すると、その写しを見せているページの変更も知らされる。
#[test]
fn the_clients_hear_of_the_pages_that_transclude_the_item_that_was_split() {
    let notes = Core::new();
    let (_, source, page, _) = notes.transcluded_rows("items-split-transcluder", &["ab"]);

    let told = notes.times_told(&[&page], || {
        notes.splits(&id(&source[0]), "ab", 1);
    });

    assert_ne!(told, [0]);
    assert_eq!(notes.items(&page)[0]["body"], json!("a"));
}

// 子を見せていないトランスクルージョンを途中で分割すると、後ろの文は元のページで元の項目の直後に入り、このページからは見えなくなる。元のページの文は、見せているページへ移らない。
#[test]
fn the_rest_of_a_split_transclusion_goes_below_its_source_item_on_the_source_page() {
    let notes = Core::new();
    let (source_page, source, page, own) = notes.transclusion(
        "items-split-copy",
        &["top", "  alpha beta", "  next"],
        1,
        &["copy", "below"],
        0,
    );

    let split = notes.splits(&id(&own[0]), "alpha beta", 6);

    assert_eq!(split["item"]["id"], json!(id(&source[1])));
    assert_eq!(split["created"]["pageId"], json!(source_page));
    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha ", "2:beta", "2:next"])
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:alpha ", "1:below"]));
}

// 折りたたまれたトランスクルージョンを途中で分割すると、後ろの文は元のページで、元の項目の子孫の後ろに元の項目と同じ階層で入る。直後に置くと、元の項目の子を奪ってしまうためである。
#[test]
fn the_rest_of_a_collapsed_transclusion_goes_after_the_descendants_of_its_source_item() {
    let notes = Core::new();
    let (source_page, _, page, own) = notes.transclusion(
        "items-split-copy-collapsed",
        &["top", "  alpha beta", "    child", "  next"],
        1,
        &["copy"],
        0,
    );
    notes.collapse(&id(&own[0]));

    notes.splits(&id(&own[0]), "alpha beta", 6);

    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha ", "3:child", "2:beta", "2:next"])
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:alpha "]));
}

// 自前の子を隠しているトランスクルージョンを途中で分割しても、元の項目に子がなければ、後ろの文は元のページで元の項目の直後に入る。このページの項目のidは、元のページでの位置を指せない。
#[test]
fn the_rest_of_a_collapsed_transclusion_with_own_children_goes_below_its_source_item() {
    let notes = Core::new();
    let (source_page, _, page, own) = notes.transclusion(
        "items-split-copy-collapsed-own",
        &["top", "  alpha beta", "  next"],
        1,
        &["copy", "  own child"],
        0,
    );
    notes.collapse(&id(&own[0]));

    notes.splits(&id(&own[0]), "alpha beta", 6);

    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha ", "2:beta", "2:next"])
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:alpha ", "2:own child"]));
}

// 開いているトランスクルージョンを途中で分割すると、元の項目が元のページで折りたたまれていても、後ろの文は元の項目の最初の子になる。子を見せるかどうかは、トランスクルージョンの側で決まる。
#[test]
fn the_rest_of_an_open_transclusion_is_the_first_child_of_its_source_item() {
    let notes = Core::new();
    let (source_page, source, _, own) = notes.transclusion(
        "items-split-copy-open",
        &["top", "  alpha beta", "    child", "  next"],
        1,
        &["copy"],
        0,
    );
    notes.collapse(&id(&source[1]));

    notes.splits(&id(&own[0]), "alpha beta", 6);

    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha ", "3:beta", "3:child", "2:next"])
    );
}

// 自前の子を見せているトランスクルージョンを途中で分割すると、元の項目に子がなくても、後ろの文は元の項目の最初の子になる。このページでは、自前の子の上に現れる。
#[test]
fn the_rest_of_a_transclusion_with_own_children_is_the_first_child_of_its_source_item() {
    let notes = Core::new();
    let (source_page, _, _, own) = notes.transclusion(
        "items-split-copy-own",
        &["top", "  alpha beta", "  next"],
        1,
        &["copy", "  own child"],
        0,
    );

    notes.splits(&id(&own[0]), "alpha beta", 6);

    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha ", "3:beta", "2:next"])
    );
}

// トランスクルージョンを行頭で分割すると、空の項目がこのページでその直前に入り、要求が運んできた本文は元の項目に保存される。空の項目は元のページの文を持たないので、このページに置ける。
#[test]
fn a_split_at_the_start_of_a_transclusion_puts_an_empty_item_above_it_on_its_page() {
    let notes = Core::new();
    let (source_page, source, page, own) = notes.transclusion(
        "items-split-copy-start",
        &["top", "  alpha"],
        1,
        &["above", "  copy"],
        1,
    );

    let split = notes.splits(&id(&own[1]), "alpha typed", 0);

    assert_eq!(split["item"]["id"], json!(id(&source[1])));
    assert_eq!(split["created"]["pageId"], json!(page));
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:above", "2:", "2:alpha typed"])
    );
    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha typed"])
    );
}

// 子を見せていないトランスクルージョンを行末で分割すると、空の項目がこのページでその直後に入り、要求が運んできた本文は元の項目に保存される。元のページに作ると、このページには現れない。
#[test]
fn a_split_at_the_end_of_a_transclusion_that_shows_no_children_puts_an_empty_item_below_it_on_its_page(
) {
    let notes = Core::new();
    let (source_page, source, page, own) = notes.transclusion(
        "items-split-copy-end",
        &["top", "  alpha", "  next"],
        1,
        &["above", "  copy", "below"],
        1,
    );

    let split = notes.splits(&id(&own[1]), "alpha typed", 11);

    assert_eq!(split["item"]["id"], json!(id(&source[1])));
    assert_eq!(split["created"]["pageId"], json!(page));
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:above", "2:alpha typed", "2:", "1:below"])
    );
    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha typed", "2:next"])
    );
}

// 折りたたまれたトランスクルージョンを行末で分割すると、元の項目に子があっても、空の項目はこのページで、隠れている自前の子孫の後ろに入る。
#[test]
fn a_split_at_the_end_of_a_collapsed_transclusion_puts_the_empty_item_after_its_own_hidden_descendants(
) {
    let notes = Core::new();
    let (source_page, _, page, own) = notes.transclusion(
        "items-split-copy-end-collapsed",
        &["top", "  alpha", "    child"],
        1,
        &["copy", "  own child", "below"],
        0,
    );
    notes.collapse(&id(&own[0]));

    notes.splits(&id(&own[0]), "alpha", 5);

    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:alpha", "2:own child", "1:", "1:below"])
    );
    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha", "3:child"])
    );
}

// 自前の子を見せているトランスクルージョンを行末で分割すると、元の項目に子がなければ、空の項目はこのページで自前の子の先頭に入る。
#[test]
fn a_split_at_the_end_of_a_transclusion_with_own_children_puts_the_empty_item_in_as_their_first() {
    let notes = Core::new();
    let (source_page, _, page, own) = notes.transclusion(
        "items-split-copy-end-own",
        &["top", "  alpha"],
        1,
        &["copy", "  own child"],
        0,
    );

    notes.splits(&id(&own[0]), "alpha", 5);

    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:alpha", "2:", "2:own child"])
    );
    assert_eq!(notes.outline_of(&source_page), vs(&["1:top", "2:alpha"]));
}

// 元の項目の子を見せているトランスクルージョンを行末で分割すると、空の項目は元のページで元の項目の最初の子になる。このページでは、トランスクルージョンのすぐ下に現れる。
#[test]
fn a_split_at_the_end_of_a_transclusion_that_shows_children_puts_the_empty_item_in_as_the_first_child_of_its_source(
) {
    let notes = Core::new();
    let (source_page, _, page, own) = notes.transclusion(
        "items-split-copy-end-children",
        &["top", "  alpha", "    child"],
        1,
        &["copy"],
        0,
    );

    let split = notes.splits(&id(&own[0]), "alpha", 5);

    assert_eq!(split["created"]["pageId"], json!(source_page));
    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha", "3:", "3:child"])
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:alpha"]));
}

// トランスクルージョンが見せている最後の項目を行末で分割すると、空の項目はトランスクルージョンのページでその下に入り、要求が運んできた本文は分割した項目に保存される。
#[test]
fn a_split_at_the_end_of_the_last_item_a_transclusion_shows_puts_an_empty_item_below_the_transclusion(
) {
    let notes = Core::new();
    let (source_page, source, page, own) = notes.transclusion(
        "items-split-shown-last",
        &["top", "  alpha", "    child a", "    child b", "  next"],
        1,
        &["above", "copy", "below"],
        1,
    );

    let split = notes.splits_shown_by(&id(&own[1]), &id(&source[3]), "child b typed", 13);

    assert_eq!(split["item"]["id"], json!(id(&source[3])));
    assert_eq!(split["created"]["pageId"], json!(page));
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:above", "1:alpha", "1:", "1:below"])
    );
    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha", "3:child a", "3:child b typed", "2:next"])
    );
}

// 見せている最後の項目が折りたたまれていて、その下に隠れた子があっても、行末での分割では空の項目がトランスクルージョンの下に入る。隠れた項目は、見せている項目に数えない。
#[test]
fn a_collapsed_item_is_the_last_a_transclusion_shows_though_it_hides_children() {
    let notes = Core::new();
    let (source_page, source, page, own) = notes.transclusion(
        "items-split-shown-last-collapsed",
        &["top", "  alpha", "    child", "      hidden", "  next"],
        1,
        &["copy"],
        0,
    );
    notes.collapse(&id(&source[2]));

    notes.splits_shown_by(&id(&own[0]), &id(&source[2]), "child", 5);

    assert_eq!(notes.outline_of(&page), vs(&["1:alpha", "1:"]));
    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha", "3:child", "4:hidden", "2:next"])
    );
}

// 見せている項目のうち最後でないものを行末で分割すると、空の項目は元のページで、ふつうの分割と同じ場所に入る。折りたたまれた項目の後ろにも、見せている項目が続くことがある。
#[test]
fn a_split_at_the_end_of_an_item_that_is_not_the_last_shown_stays_on_the_source_page() {
    let notes = Core::new();
    let (source_page, source, page, own) = notes.transclusion(
        "items-split-shown-not-last",
        &[
            "top",
            "  alpha",
            "    child a",
            "      hidden",
            "    child b",
        ],
        1,
        &["copy"],
        0,
    );
    notes.collapse(&id(&source[2]));

    let split = notes.splits_shown_by(&id(&own[0]), &id(&source[2]), "child a", 7);

    assert_eq!(split["created"]["pageId"], json!(source_page));
    assert_eq!(
        notes.outline_of(&source_page),
        vs(&[
            "1:top",
            "2:alpha",
            "3:child a",
            "4:hidden",
            "3:",
            "3:child b"
        ])
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:alpha"]));
}

// 見せている最後の項目でも、途中での分割では、後ろの文は元のページでその直後に入る。元のページの文は、見せているページへ移らない。
#[test]
fn the_rest_of_the_last_item_a_transclusion_shows_stays_on_the_source_page() {
    let notes = Core::new();
    let (source_page, source, page, own) = notes.transclusion(
        "items-split-shown-middle",
        &["top", "  alpha", "    child beta"],
        1,
        &["copy"],
        0,
    );

    notes.splits_shown_by(&id(&own[0]), &id(&source[2]), "child beta", 6);

    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha", "3:child ", "3:beta"])
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:alpha"]));
}

// 名指しされたトランスクルージョンがその項目を見せていなければ、分割は断られ、何も変わらない。
#[test]
fn a_split_is_refused_when_the_transclusion_named_does_not_show_the_item() {
    let notes = Core::new();
    let (source_page, source, page, own) = notes.transclusion(
        "items-split-shown-not",
        &["top", "  alpha", "    child", "  outside"],
        1,
        &["copy"],
        0,
    );

    let split = notes.split_shown_by(&id(&own[0]), &id(&source[3]), "outside", 7);

    assert!(is_refused(&split), "{split:?}");
    assert_eq!(
        notes.outline_of(&source_page),
        vs(&["1:top", "2:alpha", "3:child", "2:outside"])
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:alpha"]));
}

// トランスクルージョンでない項目を、見せているトランスクルージョンとして名指しした分割は断られる。
#[test]
fn a_split_is_refused_when_the_item_named_as_the_transclusion_is_not_one() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-shown-plain");
    let rows = notes.fill(&page, &["plain", "  child"]);

    let split = notes.split_shown_by(&id(&rows[0]), &id(&rows[1]), "child", 5);

    assert!(is_refused(&split), "{split:?}");
    assert_eq!(notes.outline_of(&page), vs(&["1:plain", "2:child"]));
}

// トランスクルージョンを途中で分割すると、元のページとトランスクルージョンのページの変更が、1回ずつクライアントに知らされる。
#[test]
fn the_clients_hear_once_of_each_page_when_a_transclusion_is_split() {
    let notes = Core::new();
    let (source_page, _, page, own) =
        notes.transclusion("items-split-copy-event", &["alpha beta"], 0, &["copy"], 0);

    let told = notes.times_told(&[&source_page, &page], || {
        notes.splits(&id(&own[0]), "alpha beta", 6);
    });

    assert_eq!(told, [1, 1]);
}

// 見せている最後の項目を行末で分割すると、空の項目が入るトランスクルージョンのページの変更も知らされる。
#[test]
fn the_clients_hear_of_the_page_of_the_transclusion_when_the_empty_item_goes_below_it() {
    let notes = Core::new();
    let (source_page, source, page, own) = notes.transclusion(
        "items-split-shown-event",
        &["alpha", "  child"],
        0,
        &["copy"],
        0,
    );

    let told = notes.times_told(&[&source_page, &page], || {
        notes.splits_shown_by(&id(&own[0]), &id(&source[1]), "child", 5);
    });

    assert_eq!(told, [1, 1]);
}

// 行末での分割では、空の項目が直後に入る。
#[test]
fn a_split_at_the_end_puts_an_empty_item_below() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-end");
    let rows = notes.fill(&page, &["alpha", "next"]);

    let split = notes.splits(&id(&rows[0]), "alpha", 5);

    assert_eq!(split["created"]["body"], json!(""));
    assert_eq!(notes.outline_of(&page), vs(&["1:alpha", "1:", "1:next"]));
}

// カーソルが本文の長さを超えていても、行末での分割になる。
#[test]
fn a_split_with_the_cursor_past_the_end_is_a_split_at_the_end() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-past");
    let rows = notes.fill(&page, &["alpha"]);

    notes.splits(&id(&rows[0]), "alpha", 99);

    assert_eq!(notes.outline_of(&page), vs(&["1:alpha", "1:"]));
}

// ページの最初の項目を行頭で分割すると、空の項目がページの先頭に入る。
#[test]
fn a_split_at_the_start_of_the_first_item_puts_the_empty_item_first_on_the_page() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-first");
    let rows = notes.fill(&page, &["alpha", "next"]);

    notes.splits(&id(&rows[0]), "alpha", 0);

    assert_eq!(notes.outline_of(&page), vs(&["1:", "1:alpha", "1:next"]));
}

// 空の項目の分割では、空の項目が下に入る。上に入れる理由になる本文がないためである。
#[test]
fn a_split_of_an_empty_item_puts_the_empty_item_below() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-empty");
    let rows = notes.fill(&page, &["", "next"]);

    let split = notes.splits(&id(&rows[0]), "", 0);

    assert_eq!(split["item"]["id"], json!(id(&rows[0])));
    assert_eq!(
        notes.idxs_of(&page)[1],
        split["created"]["idx"].as_str().unwrap()
    );
}

// 分割するのは、保存されている本文ではなく、要求が運んできた本文である。まだ保存されていない入力が入っている。
#[test]
fn the_split_cuts_the_body_the_request_carries_not_the_stored_one() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-unsaved");
    let rows = notes.fill(&page, &["saved"]);

    notes.splits(&id(&rows[0]), "saved and typed", 6);

    assert_eq!(notes.outline_of(&page), vs(&["1:saved ", "1:and typed"]));
}

// 行頭での分割でも、要求が運んできた本文が元の項目に保存される。
#[test]
fn a_split_at_the_start_saves_the_body_the_request_carries() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-start-unsaved");
    let rows = notes.fill(&page, &["saved"]);

    notes.splits(&id(&rows[0]), "saved and typed", 0);

    assert_eq!(notes.outline_of(&page), vs(&["1:", "1:saved and typed"]));
}

// コードブロックの中で分けると、元の項目に残る本文は閉じ、新しい項目の本文には開始行が写される。RESTやMCPからの分割である（ADR 050）。
#[test]
fn a_split_inside_a_code_block_closes_and_opens_it_again() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-fence");
    let rows = notes.fill(&page, &["saved"]);

    let split = notes.splits(&id(&rows[0]), "```js\ncode\n```", 8);

    assert_eq!(split["item"]["body"], json!("```js\nco\n```"));
    assert_eq!(split["created"]["body"], json!("```js\nde\n```"));
}

// 分割の要求の本文が閉じていないブロックで終われば、どこで分けても断られる。分けた2つは閉じていても、届いた本文が閉じていない。
#[test]
fn a_split_of_a_body_with_an_unclosed_block_is_refused() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-unclosed");
    let rows = notes.fill(&page, &["saved"]);

    for cursor in [0, 5] {
        let split = notes.split(&id(&rows[0]), "```\nabc", cursor);

        assert!(is_refused(&split), "{cursor}: {split:?}");
    }
    assert_eq!(notes.outline_of(&page), vs(&["1:saved"]));
}

// 分割は項目を2つ書くが、ページの変更は1回だけクライアントに知らされる。
#[test]
fn the_clients_hear_of_the_page_once_for_a_split() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-event");
    let rows = notes.fill(&page, &["ab"]);

    let told = notes.times_told(&[&page], || {
        notes.splits(&id(&rows[0]), "ab", 1);
    });

    assert_eq!(told, [1]);
}

// もう無い項目の分割は断られる。ほかの項目の操作と同じ答えである。
#[test]
fn a_split_of_an_item_that_is_gone_is_refused() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-split-gone");
    let rows = notes.fill(&page, &["ab", "other"]);
    notes.delete_item(&id(&rows[0]));

    let split = notes.split(&id(&rows[0]), "ab", 1);

    assert!(is_refused(&split), "{split:?}");
    assert_eq!(notes.outline_of(&page), vs(&["1:other"]));
}
