//! Sending an item with its children under the "# Stash" heading at the end of the page.

use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::{create_calls, item, move_calls, typed, update_calls, Harness};
use crate::model::TranscludedTrees;
use crate::testing::item_with;

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

// Stash 見出しが既にあれば、新しい見出しは作られず、項目はその直下へ移る。
#[test]
fn sending_to_the_stash_reuses_an_existing_stash_heading() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "todo"),
        item("stash", 1, "# Stash"),
        item("old", 2, ""),
    ]);
    h.send(Event::SendToStash {
        item_id: "a".into(),
    });
    assert_eq!(stash_calls(&h), [("a".to_string(), None)]);
    assert_eq!(h.outline(), ["stash:1=# Stash", "a:2=todo", "old:2="]);
}

// 入力した直後に Stash へ送ると、打ったばかりの文は要求に入って届く。保存を別には送らず、その自動保存は取り消される。画面には、サーバーが保存した本文が出る。
#[test]
fn text_typed_just_before_sending_to_the_stash_travels_in_the_request() {
    let mut h = Harness::with_items(vec![item("a", 1, "todo"), item("z", 1, "")]);
    typed(&mut h, "a", "todo typed");
    h.send(Event::SendToStash {
        item_id: "a".into(),
    });
    assert_eq!(
        stash_calls(&h),
        [("a".to_string(), Some("todo typed".to_string()))]
    );
    h.fire_timers();
    assert!(update_calls(&h.api).is_empty());
    assert_eq!(h.outline(), ["z:1=", "new1:1=# Stash", "a:2=todo typed"]);
}

// トランスクルージョンのルートに打って、まだ保存していない文も、Stash への要求に入って届く。答えのあとも、行は打った文を見せる（#734）。
#[test]
fn text_typed_into_a_transclusion_root_travels_in_the_stash_and_stays_on_its_row() {
    let mut h = Harness::with_items(vec![
        item_with("root", 1, |i| {
            i.transclude_item_id = Some("s".into());
            i.body = "shown".into();
        }),
        item("z", 1, ""),
    ]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, "shown")]);
    h.model.state.set_transcluded_trees(trees);
    typed(&mut h, "root", "shown typed");

    h.send(Event::SendToStash {
        item_id: "root".into(),
    });

    assert_eq!(
        stash_calls(&h),
        [("root".to_string(), Some("shown typed".to_string()))]
    );
    assert_eq!(h.items_ids(), ["z", "new1", "root"]);
    // The row shows the body of the source, whatever the server answers for the root
    let row = h.model.state.get_visible_item("root").unwrap();
    assert_eq!(row.body, "shown typed");
    h.fire_timers();
    assert!(update_calls(&h.api).is_empty());
}

// Stash への送付が断られると、画面は変わらず、要求に入れた文はもう一度自動保存にかけられる。
#[test]
fn after_a_refused_stash_nothing_changes_and_the_typed_text_is_auto_saved_again() {
    let mut h = Harness::with_items(vec![item("a", 1, "todo"), item("z", 1, "")]);
    typed(&mut h, "a", "todo typed");
    h.api.fail_next("StashItem", "offline");
    h.send(Event::SendToStash {
        item_id: "a".into(),
    });
    assert_eq!(h.outline(), ["a:1=todo typed", "z:1="]);
    h.fire_timers();
    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("todo typed".into()))]
    );
}

// 答えが届いたときに送った項目がもう画面に無ければ、何も表示しない。待っている間に別のページへ移った場合である。
#[test]
fn the_answer_to_a_stash_of_an_item_no_longer_shown_changes_nothing() {
    let mut h = Harness::with_items(vec![item("x", 1, "other page")]);
    h.send(Event::Stashed {
        stashed: Box::new(crate::model::Stashed {
            heading: item("new1", 1, "# Stash"),
            created: true,
            items: vec![item("a", 2, "todo")],
        }),
    });
    assert_eq!(h.outline(), ["x:1=other page"]);
}
