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

// 追加されたリンクは、丸めた位置を持つwikilinkの項目として作られ、読み直したアウトラインが表示される。リンク先の括弧と空白は取り除かれる。
#[test]
fn an_added_link_is_made_as_an_item_with_its_place() {
    let mut h = atlas_of(vec![], vec![]);
    answer_outline(
        &mut h,
        vec![link_item("new", "[[x/Target]]", Some("10,-21,0,40"))],
    );
    h.send(Event::AddAtlasLink {
        x: 10.4,
        y: -20.6,
        target: " [[x/Target]] ".into(),
    });

    assert_eq!(
        batches(&h),
        [[made(
            ItemPosition::Last,
            "[[x/Target]]",
            &[("atlas_bounds", "10,-21,0,40")]
        )]]
    );
    let links = h.view().atlas_links;
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target.as_deref(), Some("x/Target"));
}

// 空のリンクは追加されない。
#[test]
fn an_empty_link_is_not_added() {
    let mut h = atlas_of(vec![], vec![]);
    h.send(Event::AddAtlasLink {
        x: 0.0,
        y: 0.0,
        target: " [[ ]] ".into(),
    });

    assert!(batches(&h).is_empty());
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
            made(ItemPosition::after("i-one"), "", &[]),
            leveled("i-one", 2),
            leveled("l", 2),
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
    assert_eq!(removed("c"), [deleted("c")]);
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

// リンクカードを編集すると、新しいページへのリンクになる。リンク先の括弧と空白は取り除かれる。
#[test]
fn editing_a_link_card_links_it_to_the_new_page() {
    let mut h = atlas_with_link();
    h.send(Event::SetAtlasLink {
        item_id: "l".into(),
        target: " [[y/Other]] ".into(),
    });

    assert_eq!(batches(&h), [[written("l", "[[y/Other]]")]]);
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

// リンクカードの宛先を変えても、ラベルと2行目より後は保たれる。同じパスへの編集では何も変わらない。
#[test]
fn a_link_card_linked_to_another_page_keeps_its_label_and_its_later_lines() {
    let mut h = atlas_with_labelled_link();
    for target in ["x/Target", "y/Other"] {
        h.send(Event::SetAtlasLink {
            item_id: "l".into(),
            target: target.into(),
        });
    }

    assert_eq!(batches(&h), [[written("l", "[[y/Other|Shown]]\nmore")]]);
}

// 空のリンク先や同じページへの編集、ページカードへの編集では何も変わらない。
#[test]
fn an_edit_of_a_link_to_nothing_or_to_the_same_page_changes_nothing() {
    let mut h = atlas_with_link();
    h.send(Event::SetAtlasLink {
        item_id: "l".into(),
        target: "  ".into(),
    });
    h.send(Event::SetAtlasLink {
        item_id: "l".into(),
        target: "x/Target".into(),
    });
    h.send(Event::SetAtlasLink {
        item_id: "i-one".into(),
        target: "y/Other".into(),
    });

    assert!(batches(&h).is_empty());
    assert_eq!(h.view().atlas_links[0].target.as_deref(), Some("x/Target"));
}

// リンクのダイアログは、入力が一息置いた後にページ名の候補を示す。検索語が空になると候補は空になる。
#[test]
fn the_dialog_of_a_link_offers_page_names_after_a_pause() {
    let mut h = atlas_with(&[]);
    let found = |title: &str| crate::model::SearchResult {
        page_title: title.into(),
        ..Default::default()
    };
    h.api.answer_next(
        "SearchPageNames",
        Ok(ApiOutput::SearchResults(vec![
            found("x/Target"),
            found("x/Tea"),
        ])),
    );
    h.send(Event::SearchAtlasLinks {
        query: "x/T".into(),
    });
    assert!(h.view().atlas_link_suggestions.is_empty());

    h.fire_timers();
    assert_eq!(h.view().atlas_link_suggestions, ["x/Target", "x/Tea"]);

    h.send(Event::SearchAtlasLinks { query: "".into() });
    assert!(h.view().atlas_link_suggestions.is_empty());
}
