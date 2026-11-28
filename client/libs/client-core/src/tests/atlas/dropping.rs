//! Dropping what waits beside the canvas onto it (ADR 053). The item always becomes an item
//! of the top at the end of the page, placed at the point of the drop. With Ctrl held on a
//! card or on a group, the shell then sends what it sends for a card of the canvas let go
//! there with Ctrl, so the item joins as such a card does.

use super::*;

/// The ids of the items that wait for a place
fn no_place(h: &Harness) -> Vec<String> {
    let waiting = h.view().atlas_unplaced.into_iter();
    waiting.filter(|c| c.added).map(|c| c.id).collect()
}

/// The ids, the levels and the bounds of the items of the atlas page as they are shown
fn outline(h: &Harness) -> Vec<(String, i32, Option<String>)> {
    let items = h.model.state.items();
    items
        .iter()
        .map(|item| {
            let bounds = item.properties.get("atlas_bounds");
            let bounds = bounds.and_then(|value| value.as_text().map(str::to_string));
            (item.id.clone(), item.level, bounds)
        })
        .collect()
}

fn shown(id: &str, level: i32, bounds: Option<&str>) -> (String, i32, Option<String>) {
    (id.into(), level, bounds.map(str::to_string))
}

// ドロップした項目は、Markdownのどこにあっても、1回の送信で、ページの末尾のレベル1の項目になり、ドロップした点を`atlas_bounds`として持つ。その点に、すぐに描かれる。読めない`atlas_bounds`を持っていた項目も同じである。
#[test]
fn a_dropped_item_goes_to_the_end_of_the_page_at_the_top_placed_at_the_point() {
    let mut h = atlas_of(
        vec![],
        vec![
            atlas_item(
                "call",
                1,
                "Hi",
                &[("atlas_type", "callout"), ("atlas_bounds", "1,2")],
            ),
            atlas_item("text", 1, "plain", &[("atlas_bounds", "5,5,0,40")]),
        ],
    );
    h.api
        .fail_next("GetPageById", "not read again in this test");
    drop_on(&mut h, "call", 100.4, 50.0);

    assert!(upserts(&h).is_empty());
    assert_eq!(
        batches(&h),
        [[leveled_at("call", 1, "100,50,0,40"), moved_last(&["call"])]]
    );
    assert_eq!(
        outline(&h),
        [
            shown("text", 1, Some("5,5,0,40")),
            shown("call", 1, Some("100,50,0,40")),
        ]
    );
    let callouts = h.view().atlas_callouts;
    assert_eq!((callouts[0].x, callouts[0].y), (100.0, 50.0));
    assert!(no_place(&h).is_empty());
}

// 親の下の項目も、ドロップすると、親から出て、ページの末尾のレベル1に入る。親が描かれているかどうかによらない。その下の項目は、子のまま一緒に動く。最後の子が出て親でなくなった項目は、同じ送信で辺を失う。
#[test]
fn a_dropped_item_under_a_parent_leaves_it_with_what_is_under_it() {
    let atlas = || {
        let mut h = atlas_of(
            vec![],
            vec![
                atlas_item("parent", 1, "Parent", &[("atlas_side", "left")]),
                atlas_item("under", 2, "Under", &[]),
                atlas_item("deep", 3, "deep", &[("atlas_bounds", "5,5,0,60")]),
                parent_item("placed", 1, "Placed", &[("atlas_side", "left")]),
                atlas_item("shown", 2, "shown", &[("atlas_bounds", "1,1,0,40")]),
                atlas_item("not", 2, "not", &[]),
            ],
        );
        h.api
            .fail_next("GetPageById", "not read again in this test");
        h
    };

    // From under a parent that is not drawn
    let mut h = atlas();
    drop_on(&mut h, "under", 10.0, 20.0);
    assert_eq!(
        batches(&h),
        [[
            leveled_at("under", 1, "10,20,0,40"),
            leveled("deep", 2),
            moved_last(&["under", "deep"]),
            unsided("parent"),
        ]]
    );
    assert_eq!(
        outline(&h)[3..],
        [
            shown("not", 2, None),
            shown("under", 1, Some("10,20,0,40")),
            shown("deep", 2, Some("5,5,0,60")),
        ]
    );
    assert_eq!(no_place(&h), ["parent", "not"]);

    // An item that has bounds keeps its size
    let mut h = atlas();
    drop_on(&mut h, "deep", 10.0, 20.0);
    assert_eq!(
        batches(&h),
        [[leveled_at("deep", 1, "10,20,0,60"), moved_last(&["deep"])]]
    );

    // From under a parent that is drawn, which keeps another item
    let mut h = atlas();
    drop_on(&mut h, "not", 10.0, 20.0);
    assert_eq!(
        batches(&h),
        [[leveled_at("not", 1, "10,20,0,40"), moved_last(&["not"])]]
    );
}

// 座標のない親をドロップすると、親がページの末尾に移り、`atlas_bounds`を得る。子は、レベルも`atlas_bounds`も変わらずに一緒に動き、座標を持つ子が、親からの相対の位置に出る。
#[test]
fn a_dropped_parent_without_a_place_takes_what_is_under_it_along() {
    let mut h = atlas_of(
        vec![child("one")],
        vec![
            atlas_item("g", 1, "Tools", &[]),
            card_item("one", 2, Some("10,20,0,40")),
            atlas_item("not", 2, "not", &[]),
            atlas_item("text", 1, "plain", &[("atlas_bounds", "5,5,0,40")]),
        ],
    );
    h.api
        .fail_next("GetPageById", "not read again in this test");
    drop_on(&mut h, "g", 100.0, 50.0);

    assert_eq!(
        batches(&h),
        [[
            leveled_at("g", 1, "100,50,0,40"),
            leveled("i-one", 2),
            leveled("not", 2),
            moved_last(&["g", "i-one", "not"])
        ]]
    );
    let card = h.view().atlas_cards.remove(0);
    assert_eq!(card.bounds.map(|b| (b.x, b.y)), Some((10.0, 20.0)));
    assert_eq!((card.groups[0].x, card.groups[0].y), (100.0, 50.0));
    assert_eq!(no_place(&h), ["not"]);
}

/// The group `g` at 100,200 with the cards of one and two under it, one above the other, the
/// card of three at the top, and a note that waits for a place under an item that is not
/// drawn.
fn atlas_with_a_group_and_a_waiting_note() -> Harness {
    let mut h = atlas_of(
        vec![child("one"), child("three"), child("two")],
        vec![
            parent_item("g", 1, "", &[("atlas_bounds", "100,200,0,40")]),
            card_item("one", 2, Some("0,0,0,40")),
            card_item("two", 2, Some("0,100,0,40")),
            card_item("three", 1, Some("300,300,0,40")),
            atlas_item("hidden", 1, "hidden", &[]),
            atlas_item("note", 2, "note", &[("atlas_bounds", "1,1,0,40")]),
        ],
    );
    h.api
        .fail_next("GetPageById", "not read again in this test");
    h
}

// Ctrlを押してグループの上にドロップした項目は、キャンバスのカードをCtrlでそこに放したときと同じに、そのグループの親の子になる。`atlas_bounds`は親からの相対の座標で、兄弟の中の入り先は、その座標のYで決まる。
#[test]
fn an_item_dropped_with_ctrl_on_a_group_goes_under_its_parent_as_a_card_of_the_canvas_does() {
    let mut h = atlas_with_a_group_and_a_waiting_note();
    drop_on(&mut h, "note", 130.0, 250.0);
    h.send(Event::MoveAtlasCards {
        card_ids: vec!["note".into()],
        parent_id: "g".into(),
    });

    assert_eq!(
        batches(&h),
        [
            vec![leveled_at("note", 1, "130,250,0,40"), moved_last(&["note"])],
            vec![
                leveled_at("note", 2, "30,50,0,40"),
                moved(&["note"], "i-one")
            ],
        ]
    );
}

// Ctrlを押して、グループにないカードの上にドロップした項目は、キャンバスのカードと同じに、そのカードと2つで新しいグループを作る。新しい親は、2つの座標の重心にある。
#[test]
fn an_item_dropped_with_ctrl_on_a_lone_card_makes_a_new_group_of_the_two() {
    let mut h = atlas_with_a_group_and_a_waiting_note();
    drop_on(&mut h, "note", 320.0, 340.0);
    group(&mut h, &["note"], "a/b/three");

    assert_eq!(
        batches(&h)[1],
        [
            made(
                ItemPosition::after("i-three"),
                "",
                &[("atlas_bounds", "310,320,0,40")]
            ),
            leveled_at("i-three", 2, "-10,-20,0,40"),
            leveled_at("note", 2, "10,20,0,40"),
            moved(&["i-three", "note"], NEW),
        ]
    );
}

/// The group `g` at 100,200 with the cards of one and two under it, and the card of three at
/// the top. The page of four has no card.
fn atlas_with_a_page_not_added() -> Vec<Item> {
    vec![
        parent_item("g", 1, "", &[("atlas_bounds", "100,200,0,40")]),
        card_item("one", 2, Some("0,0,0,40")),
        card_item("two", 2, Some("0,100,0,40")),
        card_item("three", 1, Some("300,300,0,40")),
    ]
}

/// The atlas of [`atlas_with_a_page_not_added`] after the page of four was dropped at 130,250
/// with Ctrl, while the server is slow: the shell has sent what follows the drop, but the
/// card is not made yet. `made` is the outline the server answers with once it is.
fn page_dropped_with_ctrl(follows: Event, made: Vec<Item>) -> Harness {
    let pages = ["four", "one", "three", "two"].map(child).to_vec();
    let mut h = atlas_of(pages, atlas_with_a_page_not_added());
    answer_outline(&mut h, made);
    h.hold("ApplyBatch");
    drop_on(&mut h, "a/b/four", 130.0, 250.0);
    h.send(follows);
    h
}

// 未追加のページをCtrlを押してグループやカードの上にドロップすると、ページの末尾にカードができたあとで、キャンバスのカードと同じにグループに入る。シェルは2つの操作を続けて送るので、カードができる前に届いた操作は、カードができるまで待つ。
#[test]
fn a_page_with_no_card_dropped_with_ctrl_joins_once_its_card_is_made() {
    let made = || {
        let mut items = atlas_with_a_page_not_added();
        items.push(card_item("four", 1, Some("130,250,0,40")));
        items
    };
    let onto_the_group = Event::MoveAtlasCards {
        card_ids: vec!["a/b/four".into()],
        parent_id: "g".into(),
    };
    let onto_the_card = Event::GroupAtlasCards {
        card_ids: vec!["a/b/four".into()],
        target_id: "a/b/three".into(),
    };

    let mut h = page_dropped_with_ctrl(onto_the_group, made());
    assert!(batches(&h).is_empty());
    h.release_held();
    assert_eq!(
        batches(&h)[1],
        [
            leveled_at("i-four", 2, "30,50,0,40"),
            moved(&["i-four"], "i-one")
        ]
    );

    let mut h = page_dropped_with_ctrl(onto_the_card, made());
    h.release_held();
    assert_eq!(
        batches(&h)[1],
        [
            made_group(ItemPosition::after("i-three"), "215,275,0,40"),
            leveled_at("i-four", 2, "-85,-25,0,40"),
            leveled_at("i-three", 2, "85,25,0,40"),
            moved(&["i-four", "i-three"], NEW),
        ]
    );
}

// カードができなかったときは、待っていた操作は捨てられる。あとでそのページのカードができても、グループには入らない。
#[test]
fn what_waits_for_a_card_that_is_not_made_is_given_up() {
    let onto_the_group = Event::MoveAtlasCards {
        card_ids: vec!["a/b/four".into()],
        parent_id: "g".into(),
    };
    // The outline comes back without the card
    let mut h = page_dropped_with_ctrl(onto_the_group, atlas_with_a_page_not_added());
    h.release_held();
    let mut later = atlas_with_a_page_not_added();
    later.push(card_item("four", 1, Some("130,250,0,40")));
    h.send(Event::AtlasChanged {
        atlas_id: ATLAS_ID.into(),
        items: later,
    });

    assert_eq!(batches(&h).len(), 1);
    assert_eq!(bounds_of(&h, "a/b/four").map(|b| b.x), Some(130.0));
}

// ドロップが拒否されると、アラートが表示され、項目は元の場所と元の`atlas_bounds`に戻る。
#[test]
fn a_refused_drop_puts_the_item_back_where_it_was() {
    let items = || {
        vec![
            atlas_item("hidden", 1, "hidden", &[]),
            atlas_item("note", 2, "note", &[("atlas_bounds", "1,1,0,40")]),
            atlas_item("call", 1, "Hi", &[("atlas_type", "callout")]),
            atlas_item("text", 1, "plain", &[("atlas_bounds", "5,5,0,40")]),
        ]
    };
    let before = [
        shown("hidden", 1, None),
        shown("note", 2, Some("1,1,0,40")),
        shown("call", 1, None),
        shown("text", 1, Some("5,5,0,40")),
    ];

    for dropped in ["note", "call"] {
        let mut h = atlas_of(vec![], items());
        h.api.fail_next("ApplyBatch", "refused");
        drop_on(&mut h, dropped, 10.0, 20.0);

        assert_eq!(outline(&h), before, "{dropped}");
        assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
        assert!(!h.api.called("GetPageById"));
    }
}

// アトラスでないページ、数でない点、ページでもカードでもないものは、何も送られない。
#[test]
fn nothing_is_dropped_on_no_atlas_at_no_point_or_for_no_item() {
    let mut h = atlas_of(vec![], vec![atlas_item("note", 1, "note", &[])]);
    drop_on(&mut h, "note", f64::NAN, 0.0);
    drop_on(&mut h, "note", 0.0, f64::INFINITY);
    drop_on(&mut h, "none", 0.0, 0.0);

    assert!(batches(&h).is_empty());
    assert_eq!(outline(&h), [shown("note", 1, None)]);
}
