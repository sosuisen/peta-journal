//! Sending an item with its children under the "# Stash" heading at the end of the page.

use crate::app::Event;
use crate::harness::{create_calls, item, move_calls, Harness};

// Stash への送付で Stash 見出しがページ末尾に作られ、部分木がそのまま（タグなしで）その下へ移動される。
#[test]
fn sending_to_the_stash_creates_its_heading_at_the_end_and_moves_the_subtree_under_it() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "todo"),
        item("a1", 2, ""),
        item("z", 1, ""),
    ]);
    h.send(Event::SendToStash {
        item_id: "a".into(),
    });
    assert!(
        !h.api.called("UpdateItem"),
        "the item is moved as it is, with no tag"
    );
    assert_eq!(
        create_calls(&h.api),
        [("page1".into(), "after:z".into(), 1, "# Stash".into(), None)]
    );
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["a".to_string(), "a1".to_string()],
            "page1".to_string(),
            "after:new1".to_string()
        )]
    );
}

// Stash 見出しが既にあれば、Stash への送付で新しい見出しは作られずその直後へ移動される。
#[test]
fn sending_to_the_stash_reuses_an_existing_stash_heading() {
    let mut h = Harness::with_items(vec![item("a", 1, "todo"), item("stash", 1, "# Stash")]);
    h.send(Event::SendToStash {
        item_id: "a".into(),
    });
    assert!(!h.api.called("CreateItem"));
    assert_eq!(move_calls(&h.api)[0].2, "after:stash");
}
