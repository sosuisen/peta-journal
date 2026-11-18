//! Sending an item to the stash of its page (ADR 011).

use journal_core::error::AppError;
use journal_core::journal::StashAnswer;
use serde_json::json;

use crate::common::items::{answer, id, input, vs};
use crate::common::pages::by_the_client;
use crate::common::Core;

impl Core {
    /// Sends an item to the stash of its page, with the body its editor has now when it is
    /// not saved yet.
    fn stash(&self, item_id: &str, body: Option<&str>) -> Result<StashAnswer, AppError> {
        let mut request = json!({});
        if let Some(body) = body {
            request["body"] = json!(body);
        }
        self.core
            .journal
            .stash(item_id, input(request), &by_the_client())
    }
}

// スタッシュへ送ると、ページ末尾に`# Stash`見出しが作られ、項目は子ごとその直下へ、見出しの子の階層で動く。要求の本文は項目に保存される。1つの要求である。
#[test]
fn sending_to_the_stash_makes_its_heading_at_the_end_and_moves_the_subtree_under_it() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-stash");
    let rows = notes.fill(&page, &["a", "  a1", "z"]);

    let stashed = answer(notes.stash(&id(&rows[0]), Some("a typed")).unwrap());

    assert_eq!(stashed["created"], json!(true));
    assert_eq!(stashed["heading"]["body"], json!("# Stash"));
    let bodies: Vec<_> = stashed["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["body"].clone())
        .collect();
    assert_eq!(bodies, vec![json!("a typed"), json!("a1")]);
    assert_eq!(
        notes.outline_of(&page),
        vs(&["1:z", "1:# Stash", "2:a typed", "3:a1"])
    );
}

// `# Stash`見出しが既にあれば、新しい見出しは作られず、項目はその直下へ動く。末尾に空白のある見出しも見つかる。
#[test]
fn sending_to_the_stash_reuses_the_heading_the_page_has() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-stash-reuse");
    let rows = notes.fill(&page, &["a", "# Stash ", "  old"]);

    let stashed = notes.stash(&id(&rows[0]), None).unwrap();

    assert!(!stashed.created);
    assert_eq!(stashed.heading.id, id(&rows[1]));
    assert_eq!(notes.outline_of(&page), vs(&["1:# Stash ", "2:a", "2:old"]));
}

// 見出しそのものはスタッシュへ送れない。断られた要求は、本文も保存しない。
#[test]
fn the_stash_heading_itself_is_not_sent_to_the_stash() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-stash-heading");
    let rows = notes.fill(&page, &["a", "# Stash"]);

    let stashed = notes.stash(&id(&rows[1]), Some("# Stash typed"));

    assert!(
        matches!(stashed, Err(AppError::BadRequest(_))),
        "{stashed:?}"
    );
    assert_eq!(notes.outline_of(&page), vs(&["1:a", "1:# Stash"]));
}

// スタッシュへ送ると、そのページの変更がクライアントに1回知らされる。見出しを作ったときも1回である。
#[test]
fn the_clients_hear_once_of_the_page_when_an_item_goes_to_the_stash() {
    let notes = Core::new();
    let page = notes.new_doc_page("items-stash-event");
    let rows = notes.fill(&page, &["a", "z"]);

    let told = notes.times_told(&[&page], || {
        notes.stash(&id(&rows[0]), None).unwrap();
    });

    assert_eq!(told, [1]);
}
