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
        .map(|c| (c.page_id, c.group))
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

// カードの上にドロップされたカードはサーバーによってグループ化され、サーバーが返したアウトラインが表示に反映される。ドロップ先自身はグループ化の対象から除かれる。
#[test]
fn cards_dropped_on_a_card_are_grouped_by_the_server_which_answers_with_the_outline() {
    let mut h = atlas_with(&[("one", Some("0,0,0,40")), ("two", Some("9,0,0,40"))]);
    h.api.answer_next(
        "ChangeAtlas",
        Ok(ApiOutput::Items(vec![
            atlas_item("g", 1, "", &[("atlas_type", "group")]),
            card_item("one", 2, Some("0,0,0,40")),
            card_item("two", 2, Some("9,0,0,40")),
        ])),
    );
    group(&mut h, &["a/b/one", "a/b/two"], "a/b/two");

    assert_eq!(
        changes(&h),
        [AtlasChange::Group {
            cards: vec!["a/b/one".into()],
            target: "a/b/two".into()
        }]
    );
    let groups: Vec<_> = h.view().atlas_cards.into_iter().map(|c| c.group).collect();
    assert_eq!(groups, [Some("g".to_string()), Some("g".to_string())]);
}

// 自分自身や存在しないカードの上にドロップされたカードは、グループを作らない。
#[test]
fn a_card_dropped_on_itself_or_on_no_card_makes_no_group() {
    let mut h = atlas_with(&[("one", Some("0,0,0,40"))]);
    group(&mut h, &["a/b/one"], "a/b/one");
    group(&mut h, &["a/b/one"], "a/b/none");

    assert!(changes(&h).is_empty());
}

// 配置とグループ化が同時に行われたカードは、表示されていた位置を保つ。サーバーが位置の保存前にグループ化を返すためである。
#[test]
fn a_card_placed_and_grouped_at_once_keeps_the_place_shown_for_it() {
    let mut h = atlas_with(&[("one", None), ("two", Some("9,0,0,40"))]);
    // The server grouped the card before its place was saved
    h.api.answer_next(
        "ChangeAtlas",
        Ok(ApiOutput::Items(vec![
            atlas_item("g", 1, "", &[("atlas_type", "group")]),
            card_item("one", 2, None),
            card_item("two", 2, Some("9,0,0,40")),
        ])),
    );
    place(&mut h, "a/b/one", 5.0, 5.0);
    group(&mut h, &["a/b/one"], "a/b/two");

    assert_eq!(bounds_of(&h, "a/b/one").map(|b| b.x), Some(5.0));
}

// グループ化が拒否されると、アウトラインは元のまま保たれ、アラートが表示される。
#[test]
fn a_refused_group_leaves_the_outline_as_it_was() {
    let mut h = atlas_with_group("", None);
    h.api.fail_next("ChangeAtlas", "refused");
    group(&mut h, &["a/b/three"], "a/b/one");

    assert_eq!(bounds_of(&h, "a/b/three").map(|b| b.x), Some(200.0));
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
}

// グループの外へドラッグされたカードは、サーバーへのグループ解除として送られる。
#[test]
fn a_card_dragged_out_of_its_group_is_ungrouped_by_the_server() {
    let mut h = atlas_with_group("", None);
    ungroup(&mut h, &["a/b/two"]);

    assert_eq!(
        changes(&h),
        [AtlasChange::Ungroup {
            cards: vec!["a/b/two".into()]
        }]
    );
}

// どのグループにも属さないカードのグループ解除では、何も送られない。
#[test]
fn a_card_in_no_group_leaves_nothing() {
    let mut h = atlas_with_group("", None);
    ungroup(&mut h, &["a/b/three"]);

    assert!(changes(&h).is_empty());
}

// グループに属するカードはそのグループの色を持ち、属さないカードは色を持たない。
#[test]
fn a_card_in_a_group_has_the_color_of_its_group() {
    let h = atlas_with_group("", None);
    let color = Some(crate::screens::atlas::group_color("g"));

    let colors: Vec<_> = h
        .view()
        .atlas_cards
        .into_iter()
        .map(|c| (c.page_id, c.group_color))
        .collect();
    assert_eq!(
        colors,
        [
            ("a/b/one".to_string(), color.clone()),
            ("a/b/three".to_string(), None),
            ("a/b/two".to_string(), color),
        ]
    );
}

fn titled(h: &mut Harness, group_id: &str, side: &str, title: &str) {
    h.send(Event::SetAtlasGroupTitle {
        group_id: group_id.into(),
        side: side.into(),
        title: title.into(),
    });
}

// グループは外形の一辺に沿ったタイトルをとる。タイトルの前後の空白は取り除かれて送られる。
#[test]
fn a_group_takes_a_title_along_a_side_of_its_outline() {
    let mut h = atlas_with_group("", None);
    titled(&mut h, "g", "left", "  Tools  ");

    assert_eq!(
        changes(&h),
        [AtlasChange::SetGroupTitle {
            item_id: "g".into(),
            side: "left".into(),
            title: "Tools".into()
        }]
    );
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
        .map(|c| (c.page_id, c.group_title))
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

// 空のタイトルは、タイトルを取り去るために送られる。
#[test]
fn an_empty_title_is_sent_to_take_the_title_away() {
    let mut h = atlas_with_group("Tools", Some("left"));
    titled(&mut h, "g", "left", " ");

    assert_eq!(
        changes(&h),
        [AtlasChange::SetGroupTitle {
            item_id: "g".into(),
            side: "left".into(),
            title: "".into()
        }]
    );
}

// 存在しないグループやグループでないアイテムへのタイトル、変更のないタイトルは送られず、不明な辺は上辺に直されて送られる。
#[test]
fn a_title_of_no_group_an_unknown_side_or_no_change_is_not_sent_as_given() {
    let mut h = atlas_with_group("Tools", None);
    titled(&mut h, "none", "top", "Tools");
    titled(&mut h, "i-one", "top", "Tools");
    titled(&mut h, "g", "top", "Tools");
    titled(&mut h, "g", "middle", "Tools");
    titled(&mut h, "g", "middle", "Other");

    assert_eq!(
        changes(&h),
        [AtlasChange::SetGroupTitle {
            item_id: "g".into(),
            side: "top".into(),
            title: "Other".into()
        }]
    );
}
