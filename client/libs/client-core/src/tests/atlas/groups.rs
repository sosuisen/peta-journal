//! Grouping cards by dropping one on another, and the title of a group.

use super::*;

/// A group item `g`, its title the body, with the cards of one and two under it, and the card
/// of three alone. Nothing but the cards under it makes the item a group.
fn atlas_with_group(title: &str, side: Option<&str>) -> Harness {
    let props: Vec<(&str, &str)> = side.map(|s| ("atlas_side", s)).into_iter().collect();
    atlas_of(
        vec![child("one"), child("three"), child("two")],
        vec![
            atlas_item("g", 1, title, &props),
            card_item("one", 2, Some("0,0,0,40")),
            card_item("two", 2, Some("100,0,0,40")),
            card_item("three", 1, Some("200,0,0,40")),
        ],
    )
}

fn ungroup(h: &mut Harness, card_ids: &[&str]) {
    h.send(Event::UngroupAtlasCards {
        card_ids: card_ids.iter().map(|id| id.to_string()).collect(),
    });
}

// 子を持つレベル1のアイテムはグループであり、その下にあるカードは、そのグループに属する。
#[test]
fn a_card_under_an_item_of_level_1_belongs_to_its_group() {
    let h = atlas_with_group("", None);

    let groups: Vec<_> = h
        .view()
        .atlas_cards
        .into_iter()
        .map(|c| (c.page_id, group_of(&c.groups)))
        .collect();
    assert_eq!(
        groups,
        [
            ("a/b/one".to_string(), Some("g".to_string())),
            ("a/b/three".to_string(), None),
            ("a/b/two".to_string(), Some("g".to_string())),
        ]
    );
}

// どのグループにもないカードの上にカードをドロップすると、1回の送信で、ドロップ先の場所に空の項目が作られ、両方のカードがその下のレベル2に移される。読み直したアウトラインが表示に反映される。
#[test]
fn cards_dropped_on_a_card_go_under_a_new_item_in_one_batch() {
    let mut h = atlas_with(&[("one", Some("9,0,0,40")), ("two", Some("0,0,0,40"))]);
    answer_outline(
        &mut h,
        vec![
            atlas_item("g", 1, "", &[]),
            card_item("two", 2, Some("0,0,0,40")),
            card_item("one", 2, Some("9,0,0,40")),
        ],
    );
    // The target itself is not among the cards that join it
    group(&mut h, &["a/b/one", "a/b/two"], "a/b/two");

    assert_eq!(
        batches(&h),
        [[
            made(ItemPosition::after("i-two"), "", &[]),
            leveled("i-two", 2),
            leveled("i-one", 2),
            moved(&["i-two", "i-one"], NEW),
        ]]
    );
    let groups: Vec<_> = h
        .view()
        .atlas_cards
        .into_iter()
        .map(|c| group_of(&c.groups))
        .collect();
    assert_eq!(groups, [Some("g".to_string()), Some("g".to_string())]);
}

// グループのカードの上にドロップされたカードは、下に兄弟がなければ、そのグループの木の末尾に、レベル2で移される。
#[test]
fn a_card_dropped_on_a_card_of_a_group_goes_to_its_end_with_no_card_below_it() {
    let mut h = atlas_with_group("", None);
    group(&mut h, &["a/b/three"], "a/b/one");

    assert_eq!(
        batches(&h),
        [[leveled("i-three", 2), moved(&["i-three"], "i-two")]]
    );
}

// グループのカードの上にドロップされたカードは、自分より下にある最初の兄弟の前に入る。
#[test]
fn a_card_dropped_on_a_card_of_a_group_goes_in_before_the_first_card_below_it() {
    let mut h = atlas_of(
        vec![child("one"), child("three"), child("two")],
        vec![
            atlas_item("g", 1, "", &[]),
            card_item("one", 2, Some("0,0,0,40")),
            card_item("two", 2, Some("0,100,0,40")),
            card_item("three", 1, Some("0,50,0,40")),
        ],
    );
    group(&mut h, &["a/b/three"], "a/b/one");

    assert_eq!(
        batches(&h),
        [[leveled("i-three", 2), moved(&["i-three"], "i-one")]]
    );
}

// 新しいグループでは、ドロップ先のカードより上にあるカードが、その前に並ぶ。
#[test]
fn a_card_that_lies_above_the_card_it_is_dropped_on_comes_before_it_in_the_new_group() {
    let mut h = atlas_with(&[("one", Some("0,100,0,40")), ("two", Some("0,0,0,40"))]);
    group(&mut h, &["a/b/two"], "a/b/one");

    assert_eq!(
        batches(&h),
        [[
            made(ItemPosition::after("i-one"), "", &[]),
            leveled("i-two", 2),
            leveled("i-one", 2),
            moved(&["i-two", "i-one"], NEW),
        ]]
    );
}

// 同じグループのカードの上にドロップされたカードは、動かない。
#[test]
fn a_card_dropped_on_a_card_of_its_own_group_stays() {
    let mut h = atlas_with_group("", None);
    group(&mut h, &["a/b/two"], "a/b/one");

    assert!(batches(&h).is_empty());
}

// ほかのグループから移されたカードは、元のグループを残す。カードが1枚になったグループも解かれない。
#[test]
fn a_card_taken_to_another_group_leaves_its_old_group_as_it_is() {
    let mut h = atlas_of(
        vec![child("one"), child("three"), child("two")],
        vec![
            atlas_item("g", 1, "", &[]),
            card_item("one", 2, Some("0,0,0,40")),
            card_item("two", 2, Some("100,0,0,40")),
            atlas_item("h", 1, "", &[]),
            card_item("three", 2, Some("50,0,0,40")),
        ],
    );
    group(&mut h, &["a/b/two"], "a/b/three");

    assert_eq!(
        batches(&h),
        [[leveled("i-two", 2), moved(&["i-two"], "i-three")]]
    );
}

// 自分自身や存在しないカードの上にドロップされたカードは、グループを作らない。
#[test]
fn a_card_dropped_on_itself_or_on_no_card_makes_no_group() {
    let mut h = atlas_with(&[("one", Some("0,0,0,40"))]);
    group(&mut h, &["a/b/one"], "a/b/one");
    group(&mut h, &["a/b/one"], "a/b/none");

    assert!(batches(&h).is_empty());
}

// 配置とグループ化が同時に行われたカードは、表示されていた位置を保つ。サーバーが位置の保存前にアウトラインを返すことがあるためである。
#[test]
fn a_card_placed_and_grouped_at_once_keeps_the_place_shown_for_it() {
    let mut h = atlas_with(&[("one", None), ("two", Some("9,0,0,40"))]);
    // The outline was read before the place was saved
    answer_outline(
        &mut h,
        vec![
            atlas_item("g", 1, "", &[]),
            card_item("two", 2, Some("9,0,0,40")),
            card_item("one", 2, None),
        ],
    );
    place(&mut h, "a/b/one", 5.0, 5.0);
    group(&mut h, &["a/b/one"], "a/b/two");

    assert_eq!(bounds_of(&h, "a/b/one").map(|b| b.x), Some(5.0));
}

// グループ化が拒否されると、アウトラインは元のまま保たれ、アラートが表示される。
#[test]
fn a_refused_group_leaves_the_outline_as_it_was() {
    let mut h = atlas_with_group("", None);
    h.api.fail_next("ApplyBatch", "refused");
    group(&mut h, &["a/b/three"], "a/b/one");

    assert_eq!(bounds_of(&h, "a/b/three").map(|b| b.x), Some(200.0));
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
    assert!(!h.api.called("GetPageById"));
}

// グループの外へドラッグされたカードは、グループの木の末尾の次に、グループと同じレベルで移される。
#[test]
fn a_card_dragged_out_of_its_group_goes_behind_the_group_at_its_level() {
    let mut h = atlas_with_group("", None);
    ungroup(&mut h, &["a/b/one"]);

    assert_eq!(
        batches(&h),
        [[leveled("i-one", 1), moved(&["i-one"], "i-two")]]
    );
}

// グループのカードがすべて外へ出ても、グループだった項目は消されない。出たカードは、その項目の次に並ぶ。
#[test]
fn the_item_of_a_group_stays_when_all_its_cards_leave() {
    let mut h = atlas_with_group("Tools", None);
    ungroup(&mut h, &["a/b/two", "a/b/one"]);

    assert_eq!(
        batches(&h),
        [[
            leveled("i-one", 1),
            leveled("i-two", 1),
            moved(&["i-one", "i-two"], "g"),
        ]]
    );
}

// カードがすべて外へ出て親でなくなった項目は、同じ送信で辺のプロパティを失う。カードが残る項目は、辺を保つ。
#[test]
fn an_item_that_all_its_cards_leave_loses_its_side_in_the_same_batch() {
    let sent = |cards: &[&str]| {
        let mut h = atlas_with_group("Tools", Some("left"));
        ungroup(&mut h, cards);
        batches(&h).remove(0)
    };

    assert_eq!(
        sent(&["a/b/one"]),
        [leveled("i-one", 1), moved(&["i-one"], "i-two")]
    );
    assert_eq!(
        sent(&["a/b/two", "a/b/one"]),
        [
            leveled("i-one", 1),
            leveled("i-two", 1),
            moved(&["i-one", "i-two"], "g"),
            unsided("g"),
        ]
    );
}

// 最後のカードがほかのグループへ移って親でなくなった項目は、同じ送信で辺のプロパティを失う。
#[test]
fn an_item_whose_last_card_goes_to_another_group_loses_its_side_in_the_same_batch() {
    let sent = |card: &str, target: &str| {
        let mut h = atlas_of(
            vec![child("one"), child("three"), child("two")],
            vec![
                atlas_item("g", 1, "", &[("atlas_side", "left")]),
                card_item("one", 2, Some("300,0,0,40")),
                atlas_item("h", 1, "", &[("atlas_side", "right")]),
                card_item("three", 2, Some("200,0,0,40")),
                card_item("two", 1, Some("100,0,0,40")),
            ],
        );
        group(&mut h, &[card], target);
        batches(&h).remove(0)
    };

    // To a group that is there already
    assert_eq!(
        sent("a/b/one", "a/b/three"),
        [
            leveled("i-one", 2),
            moved(&["i-one"], "i-three"),
            unsided("g"),
        ]
    );
    // To a group that is made for it
    assert_eq!(
        sent("a/b/three", "a/b/two"),
        [
            made(ItemPosition::after("i-two"), "", &[]),
            leveled("i-two", 2),
            leveled("i-three", 2),
            moved(&["i-two", "i-three"], NEW),
            unsided("h"),
        ]
    );
}

// 2つのグループから同時に出たカードは、1回の送信で、それぞれのグループの次に移される。
#[test]
fn cards_that_leave_two_groups_at_once_go_behind_their_groups_in_one_batch() {
    let mut h = atlas_of(
        vec![child("one"), child("three"), child("two")],
        vec![
            atlas_item("g", 1, "", &[]),
            card_item("one", 2, Some("0,0,0,40")),
            card_item("two", 2, Some("100,0,0,40")),
            atlas_item("h", 1, "", &[]),
            card_item("three", 2, Some("200,0,0,40")),
        ],
    );
    ungroup(&mut h, &["a/b/three", "a/b/two"]);

    assert_eq!(
        batches(&h),
        [[
            leveled("i-two", 1),
            moved(&["i-two"], "i-one"),
            leveled("i-three", 1),
            moved(&["i-three"], "h"),
        ]]
    );
}

// どのグループにも属さないカードのグループ解除では、何も送られない。
#[test]
fn a_card_in_no_group_leaves_nothing() {
    let mut h = atlas_with_group("", None);
    ungroup(&mut h, &["a/b/three"]);

    assert!(batches(&h).is_empty());
}

fn titled(h: &mut Harness, group_id: &str, side: &str, title: &str) {
    h.send(Event::SetAtlasGroupTitle {
        group_id: group_id.into(),
        side: side.into(),
        title: title.into(),
    });
}

// グループのタイトルは、グループの項目の本文として書かれる。タイトルの前後の空白は取り除かれる。
#[test]
fn the_title_of_a_group_is_written_as_the_body_of_its_item() {
    let mut h = atlas_with_group("", None);
    titled(&mut h, "g", "top", "  Tools  ");

    assert_eq!(batches(&h), [[written("g", "Tools")]]);
}

// タイトルの沿う辺は、グループの項目のプロパティとして、項目の操作で保存される。
#[test]
fn the_side_of_the_title_of_a_group_is_saved_as_a_property_of_its_item() {
    let mut h = atlas_with_group("Tools", None);
    titled(&mut h, "g", "left", "Tools");

    assert_eq!(batches(&h), [[sided("g", None, "left")]]);
    assert!(upserts(&h).is_empty());
}

// タイトルと辺を同時に変えると、本文とプロパティが1回の送信で書き換えられ、そのあとアウトラインが読み直される。
#[test]
fn the_title_and_its_side_changed_at_once_are_saved_in_one_batch() {
    let mut h = atlas_with_group("", None);
    answer_outline(
        &mut h,
        vec![
            atlas_item("g", 1, "Tools", &[("atlas_side", "left")]),
            card_item("one", 2, Some("0,0,0,40")),
        ],
    );
    titled(&mut h, "g", "left", "Tools");

    assert_eq!(batches(&h), [[sided("g", Some("Tools"), "left")]]);
    assert!(upserts(&h).is_empty());
    assert_eq!(
        title_of(&h.view().atlas_cards[0].groups),
        Some(GroupTitle {
            side: "left".into(),
            text: "Tools".into()
        })
    );
}

// 本文がwikilinkだけのグループは、そのリンクのラベルをタイトルに持つ。同じタイトルのままなら、本文は書き換えられない。
#[test]
fn a_group_whose_body_is_a_link_keeps_the_link_while_its_title_stays() {
    let mut h = atlas_of(
        vec![child("one")],
        vec![
            atlas_item("g", 1, "[[x/Tools]]", &[]),
            card_item("one", 2, Some("0,0,0,40")),
        ],
    );
    titled(&mut h, "g", "left", "Tools");

    assert_eq!(batches(&h), [[sided("g", None, "left")]]);
}

// グループの各カードはそのタイトルを持ち、辺が指定されていなければタイトルは上辺に沿う。
#[test]
fn every_card_of_a_group_carries_its_title_which_lies_along_the_top_by_default() {
    let h = atlas_with_group("Tools", None);
    let title = Some(GroupTitle {
        side: "top".into(),
        text: "Tools".into(),
    });

    let titles: Vec<_> = h
        .view()
        .atlas_cards
        .into_iter()
        .map(|c| (c.page_id, title_of(&c.groups)))
        .collect();
    assert_eq!(
        titles,
        [
            ("a/b/one".to_string(), title.clone()),
            ("a/b/three".to_string(), None),
            ("a/b/two".to_string(), title)
        ]
    );
}

// 空のタイトルは、本文を空にして、タイトルを取り去る。
#[test]
fn an_empty_title_empties_the_body_to_take_the_title_away() {
    let mut h = atlas_with_group("Tools", Some("left"));
    titled(&mut h, "g", "left", " ");

    assert_eq!(batches(&h), [[written("g", "")]]);
}

// 存在しないグループやグループでないアイテムへのタイトル、変更のないタイトルは送られず、不明な辺は上辺として扱われる。
#[test]
fn a_title_of_no_group_an_unknown_side_or_no_change_is_not_sent_as_given() {
    let mut h = atlas_with_group("Tools", None);
    titled(&mut h, "none", "top", "Nobody");
    titled(&mut h, "i-one", "top", "A card");
    titled(&mut h, "g", "top", "Tools");
    titled(&mut h, "g", "middle", "Tools");
    titled(&mut h, "g", "middle", "Other");

    assert_eq!(batches(&h), [[written("g", "Other")]]);
}
