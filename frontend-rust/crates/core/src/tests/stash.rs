//! Sending an item with its children under the "# Stash" heading at the end of the page.

use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::{create_calls, item, move_calls, Harness};

/// The stash requests sent: (item id, body).
fn stash_calls(h: &Harness) -> Vec<(String, Option<String>)> {
    h.api
        .calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::StashItem { item_id, body } => Some((item_id.clone(), body.clone())),
            _ => None,
        })
        .collect()
}

// Stash への送付は1つの要求で送られ、答えが来ると、ページ末尾に作られた Stash 見出しと、その下へ移った部分木（タグなし、見出しの子の階層）が表示される。見出しの作成や移動は別には送らない。
#[test]
fn sending_to_the_stash_sends_one_request_and_shows_the_heading_made_with_the_subtree_under_it() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "todo"),
        item("a1", 2, ""),
        item("z", 1, ""),
    ]);
    h.send(Event::SendToStash {
        item_id: "a".into(),
    });
    assert_eq!(stash_calls(&h), [("a".to_string(), None)]);
    assert!(
        !h.api.called("UpdateItem"),
        "the item goes as it is, with no tag"
    );
    assert!(create_calls(&h.api).is_empty());
    assert!(move_calls(&h.api).is_empty());
    assert_eq!(h.outline(), ["z:1=", "new1:1=# Stash", "a:2=todo", "a1:3="]);
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
