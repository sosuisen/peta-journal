//! A body that ends inside a block that is not closed is not saved (ADR 050).

use journal_core::error::AppError;
use journal_core::journal::CreateItemInput;
use serde_json::json;

use crate::common::items::{id, input, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

/// What the journal answers when a body ends inside a block that is not closed.
const UNCLOSED: &str =
    "Markdownのブロック要素は、開始記号と閉じる記号が揃わない限り、保存できません。";

// 閉じていないブロックを持つ本文での更新は断られ、本文はそのまま残る。断るときの文は決まっている。
#[test]
fn an_update_to_a_body_with_an_unclosed_block_is_refused() {
    let notes = Core::new();
    let page = notes.new_doc_page("unclosed-update");
    let rows = notes.fill(&page, &["a"]);

    let updated = notes.core.journal.update_item(
        &id(&rows[0]),
        input(json!({ "body": "```\nabc" })),
        &by_the_client(),
    );

    assert!(
        matches!(&updated, Err(AppError::BadRequest(message)) if message == UNCLOSED),
        "{updated:?}"
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:a"]));
}

// 閉じていないブロックを持つ本文での作成は断られ、項目はできない。
#[test]
fn an_item_with_an_unclosed_block_is_not_created() {
    let notes = Core::new();
    let page = notes.new_doc_page("unclosed-create");
    notes.fill(&page, &["a"]);

    let created = notes.core.journal.create_item(
        input::<CreateItemInput>(
            json!({ "pageId": page, "position": "last", "level": 1, "body": ">>>\nabc" }),
        ),
        &by_the_client(),
    );

    assert!(
        matches!(created, Err(AppError::BadRequest(_))),
        "{created:?}"
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:a"]));
}

// バッチの本文の1つでも閉じていなければ、バッチ全体が断られる。閉じた本文の更新も書かれない。
#[test]
fn a_batch_with_one_unclosed_body_is_refused_as_a_whole() {
    let notes = Core::new();
    let page = notes.new_doc_page("unclosed-batch");
    let rows = notes.fill(&page, &["a", "b"]);

    let batch = notes.batch(json!([
        { "op": "update", "id": id(&rows[0]), "body": "closed" },
        { "op": "update", "id": id(&rows[1]), "body": "<!--\nabc" },
    ]));

    assert!(matches!(batch, Err(AppError::BadRequest(_))), "{batch:?}");
    assert_eq!(notes.outline_of(&page), vs(&["1:a", "1:b"]));
}
