//! The callouts written on the canvas.

use super::*;

fn callout(key: &str, x: f64, y: f64, side: &str, text: &str) -> Callout {
    Callout {
        key: key.into(),
        x,
        y,
        scale: 1.0,
        side: side.into(),
        text: text.into(),
    }
}

/// An atlas whose items are the given callouts
fn atlas_with_callouts(callouts: Vec<Item>) -> Harness {
    atlas_of(vec![], callouts)
}

fn callout_item(id: &str, text: &str, bounds: &str) -> Item {
    atlas_item(
        id,
        1,
        text,
        &[
            ("atlas_type", "callout"),
            ("atlas_bounds", bounds),
            ("atlas_side", "top"),
        ],
    )
}

/// An atlas with the callout `c` of the text Hello at (10, 20) and scale 1.5, its bubble above
fn atlas_with_callout() -> Harness {
    atlas_with_callouts(vec![callout_item("c", "Hello", "10,20,0,60")])
}

// 吹き出しは種類がcalloutのアイテムであり、レベル1にあっても、位置・倍率・辺・テキストを持って表示される。
#[test]
fn a_callout_is_an_item_of_the_callout_type_at_level_1() {
    let h = atlas_with_callout();

    let shown = Callout {
        scale: 1.5,
        ..callout("c", 10.0, 20.0, "top", "Hello")
    };
    assert_eq!(h.view().atlas_callouts, [shown]);
}

// 境界がない吹き出しや高さが0の吹き出しは、表示から除かれる。辺のない吹き出しは上に表示される。
#[test]
fn a_callout_without_a_place_is_left_out() {
    let mut no_side = callout_item("b", "Hi", "1,2,0,40");
    no_side.properties.remove("atlas_side");
    let h = atlas_with_callouts(vec![
        atlas_item("a", 1, "Hi", &[("atlas_type", "callout")]),
        no_side,
        callout_item("z", "Hi", "1,2,0,0"),
    ]);

    // A callout without a side has its bubble above
    assert_eq!(
        h.view().atlas_callouts,
        [callout("b", 1.0, 2.0, "top", "Hi")]
    );
}

// 吹き出しは、丸めた座標と辺、前後の空白を除いたテキストでサーバーによって追加され、返されたアウトラインが表示される。
#[test]
fn a_callout_is_added_by_the_server_at_the_point_with_its_side_and_text() {
    let mut h = atlas_with_callouts(vec![]);
    h.api.answer_next(
        "ChangeAtlas",
        Ok(ApiOutput::Items(vec![
            atlas_item("cp", 1, "callout", &[("atlas_type", "callouts")]),
            callout_item("new", "Hi: there", "10,-21,0,40"),
        ])),
    );
    h.send(Event::AddAtlasCallout {
        x: 10.4,
        y: -20.6,
        side: "left".into(),
        text: " Hi: there ".into(),
    });

    assert_eq!(
        changes(&h),
        [AtlasChange::AddCallout {
            bounds: "10,-21,0,40".into(),
            side: "left".into(),
            text: "Hi: there".into()
        }]
    );
    assert_eq!(h.view().atlas_callouts.len(), 1);
}

// テキストが空の吹き出しは追加されない。
#[test]
fn an_empty_callout_is_not_added() {
    let mut h = atlas_with_callouts(vec![]);
    h.send(Event::AddAtlasCallout {
        x: 0.0,
        y: 0.0,
        side: "top".into(),
        text: "  ".into(),
    });

    assert!(changes(&h).is_empty());
}

// 編集された吹き出しは、辺とテキストとともに送られる。変更のない編集は送られない。
#[test]
fn an_edited_callout_is_sent_with_its_side_and_text() {
    let mut h = atlas_with_callout();
    h.send(Event::SetAtlasCallout {
        key: "c".into(),
        side: "bottom".into(),
        text: "Bye".into(),
    });
    // No change
    h.send(Event::SetAtlasCallout {
        key: "c".into(),
        side: "top".into(),
        text: " Hello ".into(),
    });

    assert_eq!(
        changes(&h),
        [AtlasChange::SetCallout {
            item_id: "c".into(),
            side: "bottom".into(),
            text: "Bye".into()
        }]
    );
}

// 空のテキストは、吹き出しを取り去るために送られる。
#[test]
fn an_empty_text_is_sent_to_take_the_callout_away() {
    let mut h = atlas_with_callout();
    h.send(Event::SetAtlasCallout {
        key: "c".into(),
        side: "top".into(),
        text: "".into(),
    });

    assert_eq!(
        changes(&h),
        [AtlasChange::SetCallout {
            item_id: "c".into(),
            side: "top".into(),
            text: "".into()
        }]
    );
}

// 移動した吹き出しは倍率を保つ。座標は整数に丸められて保存される。
#[test]
fn a_moved_callout_keeps_its_scale() {
    let mut h = atlas_with_callout();
    h.send(Event::MoveAtlasCallout {
        key: "c".into(),
        x: -30.4,
        y: 40.6,
    });

    assert_eq!(
        upserts(&h),
        [("c".into(), "atlas_bounds".into(), "-30,41,0,60".into())]
    );
    assert_eq!(h.view().atlas_callouts[0].x, -30.0);
}

// 吹き出しは半分より小さくは縮まず、拡大には上限がない。
#[test]
fn a_callout_shrinks_no_smaller_than_half_and_grows_without_limit() {
    let mut h = atlas_with_callout();
    h.send(Event::ScaleAtlasCallout {
        key: "c".into(),
        scale: 0.2,
    });
    h.send(Event::ScaleAtlasCallout {
        key: "c".into(),
        scale: 2.504,
    });

    let values: Vec<String> = upserts(&h).into_iter().map(|(_, _, value)| value).collect();
    assert_eq!(values, ["10,20,0,20", "10,20,0,100"]);
}

// 丸めると変わらない移動や拡大縮小、存在しない吹き出しの移動は保存されない。
#[test]
fn a_callout_that_does_not_change_is_not_saved() {
    let mut h = atlas_with_callout();
    h.send(Event::MoveAtlasCallout {
        key: "c".into(),
        x: 10.2,
        y: 19.8,
    });
    h.send(Event::ScaleAtlasCallout {
        key: "c".into(),
        scale: 1.501,
    });
    h.send(Event::MoveAtlasCallout {
        key: "none".into(),
        x: 0.0,
        y: 0.0,
    });

    assert!(upserts(&h).is_empty());
}

// 吹き出しの移動が拒否されると、吹き出しは元の位置に戻される。
#[test]
fn a_refused_move_of_a_callout_puts_it_back() {
    let mut h = atlas_with_callout();
    h.api.fail_next("UpsertItemProperty", "refused");
    h.send(Event::MoveAtlasCallout {
        key: "c".into(),
        x: 0.0,
        y: 0.0,
    });

    assert_eq!(h.view().atlas_callouts[0].x, 10.0);
}
