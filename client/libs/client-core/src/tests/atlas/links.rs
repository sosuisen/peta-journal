//! The link cards: a placed link to a page outside the folder.

use super::*;

fn link_item(id: &str, body: &str, bounds: Option<&str>) -> Item {
    match bounds {
        Some(b) => atlas_item(id, 1, body, &[("atlas_bounds", b)]),
        None => atlas_item(id, 1, body, &[]),
    }
}

/// An atlas whose items are a page card and the link `l` to `x/Target` at (10, 20)
fn atlas_with_link() -> Harness {
    atlas_of(
        vec![child("one")],
        vec![
            card_item("one", 1, Some("0,0,0,40")),
            link_item("l", "[[x/Target]]", Some("10,20,0,40")),
        ],
    )
}

// 1行目がリンク一つだけで配置済みのアトラスのアイテムのうち、リンク先がこのフォルダのページでないものはリンクカードである。未配置のものとページカードは除かれる。
#[test]
fn an_item_of_the_atlas_that_is_one_placed_link_out_of_the_folder_is_a_link_card() {
    let h = atlas_of(
        vec![child("one")],
        vec![
            link_item("a", "[[x/Target]]", Some("10,20,0,40")),
            link_item("b", "[[x/Unplaced]]", None),
            card_item("one", 1, Some("0,0,0,40")),
        ],
    );

    let links = h.view().atlas_links;
    assert_eq!(links.len(), 1);
    assert_eq!(
        (links[0].item_id.as_str(), links[0].target.as_deref()),
        ("a", Some("x/Target"))
    );
    assert_eq!(
        links[0].bounds,
        Bounds {
            x: 10.0,
            y: 20.0,
            width: 0.0,
            height: 40.0
        }
    );
}

// グループアイテムの下にあるリンクカードはそのグループに属し、グループのタイトルを持つ。
#[test]
fn a_link_card_under_a_group_item_belongs_to_the_group() {
    let h = atlas_of(
        vec![],
        vec![
            parent_item("g", 1, "Tools", &[]),
            card_item("one", 2, Some("0,0,0,40")),
            atlas_item("l", 2, "[[x/Target]]", &[("atlas_bounds", "9,0,0,40")]),
        ],
    );

    let link = &h.view().atlas_links[0];
    assert_eq!(group_of(&link.groups).as_deref(), Some("g"));
    assert_eq!(
        title_of(&link.groups),
        Some(GroupTitle {
            side: "top".into(),
            text: "Tools".into()
        })
    );
}

// リンクカードは、ページカードと同じように移動と拡大縮小ができる。
#[test]
fn a_link_card_moves_and_scales_as_a_card() {
    let mut h = atlas_with_link();
    place(&mut h, "l", 30.4, -5.6);
    scale(&mut h, "l", 0.2);

    assert_eq!(
        upserts(&h),
        [
            ("l".into(), "atlas_bounds".into(), "30,-6,0,40".into()),
            ("l".into(), "atlas_bounds".into(), "30,-6,0,20".into()),
        ]
    );
    assert_eq!(
        h.view().atlas_links[0].bounds,
        Bounds {
            x: 30.0,
            y: -6.0,
            width: 0.0,
            height: 20.0
        }
    );
}

// リンクの移動が拒否されると、リンクカードは元の位置に戻される。
#[test]
fn a_refused_move_of_a_link_puts_it_back() {
    let mut h = atlas_with_link();
    h.api.fail_next("UpsertItemProperty", "nope");
    place(&mut h, "l", 30.0, 0.0);

    assert_eq!(h.view().atlas_links[0].bounds.x, 10.0);
}

// リンクカードは、ページカードと同じようにグループに加わる。
#[test]
fn a_link_card_joins_a_group_as_a_card() {
    let mut h = atlas_with_link();
    group(&mut h, &["l"], "a/b/one");

    assert_eq!(
        batches(&h),
        [[
            made(
                ItemPosition::after("i-one"),
                "",
                &[("atlas_bounds", "5,10,0,40")]
            ),
            leveled_at("i-one", 2, "-5,-10,0,40"),
            leveled_at("l", 2, "5,10,0,40"),
            moved(&["i-one", "l"], NEW),
        ]]
    );
}

// 取り除かれたリンクカードは、その項目が消される。
#[test]
fn the_item_of_a_removed_link_card_is_deleted() {
    let mut h = atlas_with_link();
    h.send(Event::RemoveAtlasLink {
        item_id: "l".into(),
    });

    assert_eq!(batches(&h), [[deleted("l")]]);
}

// ノーマルカードも「Delete Card」で項目ごと消える。
#[test]
fn the_item_of_a_deleted_plain_card_is_deleted() {
    let mut h = atlas_of(vec![], vec![link_item("p", "plain text", Some("1,2,0,40"))]);
    h.send(Event::RemoveAtlasLink {
        item_id: "p".into(),
    });

    assert_eq!(batches(&h), [[deleted("p")]]);
}

// リンクカードとノーマルカードも「Remove from Atlas」で`atlas_bounds`だけを失い、項目は残って、座標のない項目（Not on Atlas）に並ぶ。
#[test]
fn a_link_card_or_a_plain_card_taken_off_the_atlas_keeps_its_item_and_waits() {
    let mut h = atlas_of(
        vec![],
        vec![
            link_item("l", "[[x/Target]]", Some("10,20,0,40")),
            link_item("p", "plain text", Some("1,2,0,40")),
        ],
    );
    h.send(Event::RemoveAtlasCard {
        page_id: "l".into(),
    });
    h.send(Event::RemoveAtlasCard {
        page_id: "p".into(),
    });

    assert_eq!(
        removed_properties(&h),
        [
            ("l".to_string(), "atlas_bounds".to_string()),
            ("p".to_string(), "atlas_bounds".to_string()),
        ]
    );
    assert!(batches(&h).is_empty());
    let waiting: Vec<String> = h
        .view()
        .atlas_unplaced
        .into_iter()
        .filter(|c| c.added)
        .map(|c| c.id)
        .collect();
    assert_eq!(waiting, ["l", "p"]);
}

// グループの最後のカードだったリンクカードが取り除かれると、親でなくなった項目は、同じ送信で辺のプロパティを失う。ほかにカードが残る項目、辺を持たない項目、吹き出しの親、隣のカードは、何も失わない。
#[test]
fn an_item_whose_last_card_is_removed_loses_its_side_in_the_same_batch() {
    let side = [("atlas_side", "left")];
    let link = |id: &str| atlas_item(id, 2, "[[x/Target]]", &[("atlas_bounds", "9,0,0,40")]);
    let removed = |id: &str| {
        let mut h = atlas_of(
            vec![],
            vec![
                parent_item("alone", 1, "Tools", &side),
                link("a"),
                parent_item("two", 1, "", &side),
                link("b"),
                link("b2"),
                parent_item("plain", 1, "", &[]),
                link("c"),
                atlas_item(
                    "callout",
                    1,
                    "Hi",
                    &[
                        ("atlas_type", "callout"),
                        ("atlas_side", "left"),
                        ("atlas_bounds", "0,0,0,40"),
                    ],
                ),
                link("d"),
                parent_item("kept", 1, "", &[]),
                // A side written by hand on a card, which is the parent of nothing
                atlas_item(
                    "e",
                    2,
                    "[[x/Target]]",
                    &[("atlas_bounds", "9,0,0,40"), side[0]],
                ),
                link("e2"),
            ],
        );
        h.send(Event::RemoveAtlasLink { item_id: id.into() });
        batches(&h).remove(0)
    };

    assert_eq!(removed("a"), [deleted("a"), unsided("alone")]);
    assert_eq!(removed("b"), [deleted("b")]);
    // An item with no side still gets a body when it has none
    assert_eq!(
        removed("c"),
        [deleted("c"), emptied("plain", Some("no title"), false)]
    );
    assert_eq!(removed("d"), [deleted("d")]);
    // The card above the removed one is beside it, not over it
    assert_eq!(removed("e2"), [deleted("e2")]);
}

// リンクとして取り除かれるのはリンクカードだけである。ページカードでは何も送られない。
#[test]
fn only_a_link_card_is_removed_as_a_link() {
    let mut h = atlas_with_link();
    h.send(Event::RemoveAtlasLink {
        item_id: "i-one".into(),
    });

    assert!(batches(&h).is_empty());
}

/// An atlas whose only item is the link `l` to `x/Target`, shown as Shown, with a second line
fn atlas_with_labelled_link() -> Harness {
    atlas_of(
        vec![],
        vec![link_item(
            "l",
            "[[x/Target|Shown]]\nmore",
            Some("10,20,0,40"),
        )],
    )
}

// ラベルのあるリンクカードのリンク先は、パスだけである。
#[test]
fn the_target_of_a_link_card_with_a_label_is_its_path_alone() {
    let h = atlas_with_labelled_link();

    assert_eq!(h.view().atlas_links[0].target.as_deref(), Some("x/Target"));
}
