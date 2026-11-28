//! Dropping what waits beside the canvas onto it (ADR 053). The item always becomes an item
//! of the top at the end of the page, placed at the point of the drop. With Ctrl held on a
//! card it becomes a sibling of the card, and with Ctrl on a group, or Shift and Ctrl on a
//! card, a child of that item: one batch each, as for what is dragged on the canvas.

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
/// card of three at the top, and under an item that is not drawn a note with an item under
/// it, and a callout. The page of four has no card.
fn atlas_with_a_group_and_what_waits() -> Harness {
    atlas_of(
        vec![child("four"), child("one"), child("three"), child("two")],
        vec![
            parent_item("g", 1, "", &[("atlas_bounds", "100,200,0,40")]),
            card_item("one", 2, Some("0,0,0,40")),
            card_item("two", 2, Some("0,100,0,40")),
            card_item("three", 1, Some("300,300,0,40")),
            atlas_item("hidden", 1, "hidden", &[]),
            atlas_item("note", 2, "note", &[("atlas_bounds", "1,1,0,40")]),
            atlas_item("sub", 3, "sub", &[]),
            atlas_item("call", 2, "Hi", &[("atlas_type", "callout")]),
        ],
    )
}

/// What is sent for an item of [`atlas_with_a_group_and_what_waits`] dropped with its center
/// at the point, with Ctrl on the card `target` or to go under the item `parent`
fn dropped_onto(
    card: &str,
    (x, y): (f64, f64),
    target: Option<&str>,
    parent: Option<&str>,
) -> Vec<Vec<BatchOpInput>> {
    let mut h = atlas_with_a_group_and_what_waits();
    h.send(Event::DropAtlasCard {
        card_id: card.into(),
        x,
        y,
        grab_x: 0.5,
        grab_y: 0.5,
        target_id: target.map(str::to_string),
        parent_id: parent.map(str::to_string),
    });
    assert!(upserts(&h).is_empty());
    batches(&h)
}

// Ctrlを押してグループの上にドロップした項目は、1回の送信で、そのグループの親の子になる。`atlas_bounds`は親からの相対の座標で、兄弟の中の入り先は、その座標のYで決まる。下の項目は、子のまま一緒に動く。コールアウトも同じに入る。
#[test]
fn an_item_dropped_with_ctrl_on_a_group_goes_under_its_parent_in_one_batch() {
    assert_eq!(
        dropped_onto("note", (130.0, 250.0), None, Some("g")),
        [[
            leveled_at("note", 2, "30,50,0,40"),
            leveled("sub", 3),
            moved(&["note", "sub"], "i-one"),
        ]]
    );
    assert_eq!(
        dropped_onto("call", (130.0, 350.0), None, Some("g")),
        [[
            leveled_at("call", 2, "30,150,0,40"),
            moved(&["call"], "i-two")
        ]]
    );
}

// Ctrlを押して、グループにないカードの上にドロップした項目は、1回の送信で、そのカードと2つで新しいグループを作る。新しい親は、本文が空の項目で、2つの座標の重心にある。グループのカードの上では、そのグループに入る。
#[test]
fn an_item_dropped_with_ctrl_on_a_card_becomes_its_sibling_in_one_batch() {
    assert_eq!(
        dropped_onto("note", (320.0, 340.0), Some("a/b/three"), None),
        [[
            made_group(ItemPosition::after("i-three"), "310,320,0,40"),
            leveled_at("i-three", 2, "-10,-20,0,40"),
            leveled_at("note", 2, "10,20,0,40"),
            leveled("sub", 3),
            moved(&["i-three", "note", "sub"], NEW),
        ]]
    );
    assert_eq!(
        dropped_onto("note", (130.0, 250.0), Some("a/b/one"), None),
        [[
            leveled_at("note", 2, "30,50,0,40"),
            leveled("sub", 3),
            moved(&["note", "sub"], "i-one"),
        ]]
    );
}

// ShiftとCtrlを押してカードの上にドロップした項目は、1回の送信で、そのカードの子になる。カードは親になり、項目の`atlas_bounds`はカードからの相対の座標になる。
#[test]
fn an_item_dropped_with_shift_and_ctrl_on_a_card_goes_under_the_card_in_one_batch() {
    assert_eq!(
        dropped_onto("note", (320.0, 340.0), None, Some("a/b/three")),
        [[
            leveled_at("note", 2, "20,40,0,40"),
            leveled("sub", 3),
            moved(&["note", "sub"], "i-three"),
        ]]
    );
}

// 未追加のページをCtrlやShift+Ctrlでドロップしても、1回の送信である。ページの末尾にそのページへのwikilinkの項目を作り、同じ送信で、グループやカードの子にする。
#[test]
fn a_page_with_no_card_dropped_with_a_key_gets_its_item_and_joins_in_one_batch() {
    let made_card = || {
        made(
            ItemPosition::Last,
            "[[a/b/four]]",
            &[("atlas_bounds", "130,250,0,40")],
        )
    };

    assert_eq!(
        dropped_onto("a/b/four", (130.0, 250.0), None, Some("g")),
        [[
            made_card(),
            leveled_at(NEW, 2, "30,50,0,40"),
            moved(&[NEW], "i-one"),
        ]]
    );
    assert_eq!(
        dropped_onto("a/b/four", (130.0, 250.0), Some("a/b/three"), None),
        [[
            made_card(),
            made_group(ItemPosition::after("i-three"), "215,275,0,40"),
            leveled_at(NEW, 2, "-85,-25,0,40"),
            leveled_at("i-three", 2, "85,25,0,40"),
            moved(&[NEW, "i-three"], NEW),
        ]]
    );
    assert_eq!(
        dropped_onto("a/b/four", (130.0, 250.0), None, Some("a/b/three")),
        [[
            made_card(),
            leveled_at(NEW, 2, "-170,-50,0,40"),
            moved(&[NEW], "i-three"),
        ]]
    );
    // Without a key, or on nothing that takes it: only the item
    assert_eq!(
        dropped_onto("a/b/four", (130.0, 250.0), None, None),
        [[made_card()]]
    );
    assert_eq!(
        dropped_onto("a/b/four", (130.0, 250.0), None, Some("none")),
        [[made_card()]]
    );
}

// 1回の送信で作る項目（ページのカードと、新しいグループの親）は、それぞれ新しいidを持つ。同じ送信の続きの操作は、そのidで項目を名指しする。今ある項目のidは変わらない。
#[test]
fn the_items_a_batch_makes_get_ids_of_their_own_that_the_rest_of_the_batch_names() {
    let mut h = atlas_with_a_group_and_what_waits();
    h.send(Event::DropAtlasCard {
        card_id: "a/b/four".into(),
        x: 130.0,
        y: 250.0,
        grab_x: 0.5,
        grab_y: 0.5,
        target_id: Some("a/b/three".into()),
        parent_id: None,
    });

    let ops = crate::harness::batch_calls(&h.api).remove(0);
    let [BatchOpInput::Create { id: card, .. }, BatchOpInput::Create {
        id: group,
        position,
        ..
    }, BatchOpInput::Update { id: first, .. }, BatchOpInput::Update { id: second, .. }, BatchOpInput::Move {
        ids,
        position: behind,
        ..
    }] = &ops[..]
    else {
        panic!("a card, a group, two levels and a move: {ops:?}");
    };
    assert_ne!(card, group);
    for made in [card, group] {
        assert!(hmtid::Hmtid::parse(made).is_ok(), "{made:?}");
    }
    assert_eq!(position, &ItemPosition::after("i-three"));
    assert_eq!(
        [first.as_str(), second.as_str()],
        [card.as_str(), "i-three"]
    );
    assert_eq!(ids, &[card.clone(), "i-three".to_string()]);
    assert_eq!(behind, &ItemPosition::after(group));
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::LocalNow)));
}

// 項目を作らない送信では、idを作らないので、シェルに時刻を尋ねない。
#[test]
fn a_batch_that_makes_no_item_asks_the_shell_for_no_time() {
    let mut h = atlas_with_a_group_and_what_waits();
    h.send(Event::DropAtlasCard {
        card_id: "note".into(),
        x: 130.0,
        y: 250.0,
        grab_x: 0.5,
        grab_y: 0.5,
        target_id: None,
        parent_id: Some("g".into()),
    });

    assert_eq!(batches(&h).len(), 1);
    assert!(!h.ui.iter().any(|op| matches!(op, UiOp::LocalNow)));
}

// キーを押していても、名指しされた相手が項目を受け取れなければ、Ctrlなしと同じに、ページの末尾のトップレベルに入る。相手が、ない項目、描かれていないカード、ドロップした項目自身、その下の項目のときである。
#[test]
fn a_drop_on_what_cannot_take_the_item_goes_to_the_top() {
    let to_the_top = [vec![
        leveled_at("note", 1, "10,20,0,40"),
        leveled("sub", 2),
        moved_last(&["note", "sub"]),
    ]];
    let at = (10.0, 20.0);

    assert_eq!(dropped_onto("note", at, Some("none"), None), to_the_top);
    assert_eq!(dropped_onto("note", at, None, Some("none")), to_the_top);
    assert_eq!(dropped_onto("note", at, Some("note"), None), to_the_top);
    assert_eq!(dropped_onto("note", at, None, Some("note")), to_the_top);
    assert_eq!(dropped_onto("note", at, None, Some("sub")), to_the_top);
    // A callout takes nothing, and is no card to become a sibling of
    assert_eq!(dropped_onto("note", at, None, Some("call")), to_the_top);
    assert_eq!(dropped_onto("note", at, Some("call"), None), to_the_top);
}

// 親のただ1つの子を、Ctrlを押してその親の上にドロップすると、親の子のまま、ドロップした場所に置かれる。親は子を失わないので、何も失わない。別の親へ移れば、元の親は、同じ送信で辺を失い、空の本文には`no title`が書かれる。
#[test]
fn a_parent_keeps_its_only_item_dropped_on_it_and_is_tidied_when_the_item_goes() {
    let atlas = || {
        atlas_of(
            vec![child("one")],
            vec![
                parent_item("p", 1, "", &[("atlas_side", "left")]),
                atlas_item("only", 2, "only", &[]),
                card_item("one", 1, Some("300,300,0,40")),
            ],
        )
    };
    let dropped = |at: (f64, f64), parent: &str| {
        let mut h = atlas();
        h.send(Event::DropAtlasCard {
            card_id: "only".into(),
            x: at.0,
            y: at.1,
            grab_x: 0.5,
            grab_y: 0.5,
            target_id: None,
            parent_id: Some(parent.into()),
        });
        batches(&h)
    };

    assert_eq!(
        dropped((10.0, 20.0), "p"),
        [[leveled_at("only", 2, "10,20,0,40"), moved(&["only"], "p")]]
    );
    assert_eq!(
        dropped((310.0, 320.0), "a/b/one"),
        [[
            leveled_at("only", 2, "10,20,0,40"),
            moved(&["only"], "i-one"),
            emptied("p", Some("no title"), true),
        ]]
    );
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
