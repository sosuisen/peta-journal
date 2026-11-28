//! The cards of the pages on the canvas: placing, moving, scaling and removing them.

use super::*;

// 境界を持たないページのカードは未配置のまま待機し、境界を持つページのカードはキャンバス上に置かれる。
#[test]
fn a_page_without_bounds_waits_unplaced_and_a_page_with_them_lies_on_the_canvas() {
    let h = atlas_with(&[("one", None), ("two", Some("-80,-28,160,56"))]);

    let cards = h.view().atlas_cards;
    assert_eq!(
        cards.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
        vec!["one", "two"]
    );
    assert_eq!(cards[0].bounds, None);
    assert_eq!(
        cards[1].bounds,
        Some(Bounds {
            x: -80.0,
            y: -28.0,
            width: 160.0,
            height: 56.0
        })
    );
}

// カードのページは、本文のwikilinkのパスから引かれる。リンク先がこのフォルダのページならページのカードであり、リンクカードにはならない。
#[test]
fn the_page_of_a_card_is_found_from_the_path_of_its_link() {
    let h = atlas_of(
        vec![child("one"), child("two")],
        vec![atlas_item(
            "any",
            1,
            "[[a/b/two]]",
            &[("atlas_bounds", "1,2,0,40")],
        )],
    );

    assert_eq!(bounds_of(&h, "a/b/two").map(|b| b.x), Some(1.0));
    assert!(h.view().atlas_links.is_empty());
}

// 子のフォルダへのカードは、そのフォルダのindexページへのリンクであり、フォルダのカードになる。
#[test]
fn a_card_that_links_to_the_index_of_a_child_folder_is_the_card_of_the_folder() {
    let h = atlas_of(
        vec![doc("a/b/c/", "index")],
        vec![atlas_item(
            "f",
            1,
            "[[a/b/c/index]]",
            &[("atlas_bounds", "1,2,0,40")],
        )],
    );

    let cards = h.view().atlas_cards;
    assert_eq!(
        (cards[0].is_dir, cards[0].bounds.map(|b| b.x)),
        (true, Some(1.0))
    );
    assert!(h.view().atlas_links.is_empty());
}

// 最初の1行がwikilinkだけの項目は、2行目より後に何があっても、そのページのカードである。
#[test]
fn an_item_whose_first_line_is_a_link_is_a_card_whatever_follows() {
    let h = atlas_of(
        vec![child("one")],
        vec![atlas_item(
            "any",
            1,
            "[[a/b/one]]\nnotes about it",
            &[("atlas_bounds", "1,2,0,40")],
        )],
    );

    assert_eq!(bounds_of(&h, "a/b/one").map(|b| b.x), Some(1.0));
}

// ラベルのあるwikilinkのカードは、ページをパスから引き、ラベルを出す。ラベルのないカードと、カードのないページは、ページの名前を出す。
#[test]
fn a_card_shows_the_label_of_its_link_and_finds_its_page_by_the_path() {
    let h = atlas_of(
        vec![child("one"), child("three"), child("two")],
        vec![
            atlas_item(
                "any",
                1,
                "[[a/b/one|My one]]",
                &[("atlas_bounds", "1,2,0,40")],
            ),
            card_item("two", 1, None),
        ],
    );

    let cards: Vec<_> = h
        .view()
        .atlas_cards
        .into_iter()
        .map(|c| (c.page_id, c.label, c.bounds.map(|b| b.x)))
        .collect();
    assert_eq!(
        cards,
        [
            ("a/b/one".to_string(), "My one".to_string(), Some(1.0)),
            ("a/b/three".to_string(), "three".to_string(), None),
            ("a/b/two".to_string(), "two".to_string(), None),
        ]
    );
}

// このフォルダにないページへのカードは、リンクカードである。パスがこのフォルダでも、ページがなければリンクカードである。
#[test]
fn a_card_that_links_to_no_page_of_the_folder_is_a_link_card() {
    let h = atlas_of(
        vec![child("one")],
        vec![
            atlas_item("out", 1, "[[x/one]]", &[("atlas_bounds", "1,2,0,40")]),
            atlas_item("gone", 1, "[[a/b/gone]]", &[("atlas_bounds", "3,4,0,40")]),
        ],
    );

    let links: Vec<_> = h
        .view()
        .atlas_links
        .into_iter()
        .map(|l| l.item_id)
        .collect();
    assert_eq!(links, ["out", "gone"]);
    assert_eq!(bounds_of(&h, "a/b/one"), None);
}

// 同じページへのカードが2つあるときは、先のものがページのカードで、後のものはリンクカードである。
#[test]
fn the_first_of_two_cards_of_a_page_is_its_card_and_the_second_a_link_card() {
    let h = atlas_of(
        vec![child("one")],
        vec![
            card_item("one", 1, Some("1,2,0,40")),
            atlas_item("again", 1, "[[a/b/one]]", &[("atlas_bounds", "9,9,0,40")]),
        ],
    );

    assert_eq!(bounds_of(&h, "a/b/one").map(|b| b.x), Some(1.0));
    let links: Vec<_> = h
        .view()
        .atlas_links
        .into_iter()
        .map(|l| l.item_id)
        .collect();
    assert_eq!(links, ["again"]);
}

// カードのないページは、未追加の項目として、未配置の一覧に並ぶ。子のフォルダも並ぶ。座標のないカードのページは、1度だけ並ぶ。
#[test]
fn a_page_with_no_card_waits_in_the_list_with_the_pages_whose_card_has_no_place() {
    let h = atlas_of(
        vec![doc("a/b/c/", "index"), child("one"), child("two")],
        vec![card_item("two", 1, None)],
    );

    let unplaced: Vec<_> = h
        .view()
        .atlas_cards
        .into_iter()
        .map(|c| (c.label, c.bounds))
        .collect();
    assert_eq!(
        unplaced,
        [
            ("c/".to_string(), None),
            ("one".to_string(), None),
            ("two".to_string(), None)
        ]
    );
}

// フォルダのカードはそのフォルダのアトラスを開き、ページのカードはそのページを開く。
#[test]
fn a_folder_card_opens_the_atlas_of_the_folder_and_a_page_card_the_page() {
    let h = atlas_of(vec![doc("a/b/c/", "index"), child("one")], vec![]);

    let cards = h.view().atlas_cards;
    assert_eq!(
        cards.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
        vec!["a/b/c/index_atlas", "a/b/one"]
    );
}

// カードを持つのはアトラスページだけである。インデックスページではカードは空である。
#[test]
fn only_an_atlas_page_has_cards() {
    let mut h = showing(doc("a/b/", "index"));
    h.model.sidebar.index_pages = vec![child("one")];

    assert!(h.view().atlas_cards.is_empty());
}

// ドロップされたカードは、ラベルに任せた幅0と倍率1の高さ40をとり、指定した点を中心に置かれて保存される。座標は整数に丸められる。
#[test]
fn a_dropped_card_takes_the_label_width_and_scale_1_with_its_center_at_the_given_point() {
    let mut h = atlas_with(&[("one", None)]);
    place(&mut h, "a/b/one", 10.4, -20.0);

    let expected = Bounds {
        x: 10.0,
        y: -20.0,
        width: 0.0,
        height: 40.0,
    };
    assert_eq!(bounds_of(&h, "a/b/one"), Some(expected));
    assert_eq!(
        upserts(&h),
        vec![("i-one".into(), "atlas_bounds".into(), "10,-20,0,40".into())]
    );
}

// 一覧からドロップしたカードは、つかんだ点がポインターの下に残るように置かれる。中心は、つかんだ点から一覧のカードの幅と高さの割合だけずれる。
#[test]
fn a_card_dropped_from_the_list_keeps_the_grabbed_spot_under_the_pointer() {
    let mut h = atlas_with(&[("one", None)]);
    h.send(Event::DropAtlasCard {
        card_id: "a/b/one".into(),
        x: 100.0,
        y: 50.0,
        grab_x: 0.25,
        grab_y: 1.0,
        target_id: None,
        parent_id: None,
    });

    // 120 wide and 40 high: the center is 30 right of a spot a quarter in, and 20 above the bottom
    assert_eq!(
        batches(&h),
        [[
            leveled_at("i-one", 1, "130,30,0,40"),
            moved_last(&["i-one"])
        ]]
    );
}

// 未追加の項目をアトラスに置くと、そのページへのwikilinkの項目が、位置とともに作られ、読み直したアウトラインが表示される。
#[test]
fn a_page_with_no_card_dropped_on_the_atlas_gets_an_item_that_links_to_it() {
    let mut h = atlas_of(vec![child("one")], vec![]);
    answer_outline(&mut h, vec![card_item("one", 1, Some("10,-20,0,40"))]);
    place(&mut h, "a/b/one", 10.4, -20.0);

    assert_eq!(
        batches(&h),
        [[made(
            ItemPosition::Last,
            "[[a/b/one]]",
            &[("atlas_bounds", "10,-20,0,40")]
        )]]
    );
    assert!(upserts(&h).is_empty());
    assert_eq!(bounds_of(&h, "a/b/one").map(|b| b.x), Some(10.0));
}

// 未追加のフォルダをアトラスに置くと、そのフォルダのindexページへのwikilinkの項目が作られる。
#[test]
fn a_folder_with_no_card_dropped_on_the_atlas_gets_an_item_that_links_to_its_index() {
    let mut h = atlas_of(vec![doc("a/b/c/", "index")], vec![]);
    place(&mut h, "a/b/c/index", 0.0, 0.0);

    assert_eq!(
        batches(&h),
        [[made(
            ItemPosition::Last,
            "[[a/b/c/index]]",
            &[("atlas_bounds", "0,0,0,40")]
        )]]
    );
}

// 一覧からドロップした未追加の項目も、つかんだ点がポインターの下に残るように置かれる。
#[test]
fn a_page_with_no_card_dropped_from_the_list_keeps_the_grabbed_spot_under_the_pointer() {
    let mut h = atlas_of(vec![child("one")], vec![]);
    h.send(Event::DropAtlasCard {
        card_id: "a/b/one".into(),
        x: 100.0,
        y: 50.0,
        grab_x: 0.25,
        grab_y: 1.0,
        target_id: None,
        parent_id: None,
    });

    assert_eq!(
        batches(&h),
        [[made(
            ItemPosition::Last,
            "[[a/b/one]]",
            &[("atlas_bounds", "130,30,0,40")]
        )]]
    );
}

// このフォルダのページでもカードでもないものは置かれず、何も送られない。
#[test]
fn what_is_neither_a_page_of_the_folder_nor_a_card_is_not_placed() {
    let mut h = atlas_of(vec![child("one")], vec![]);
    place(&mut h, "a/b/none", 0.0, 0.0);

    assert!(upserts(&h).is_empty());
    assert!(batches(&h).is_empty());
}

// 項目を作れなかったときは、アラートが表示され、ページは未追加のまま残る。
#[test]
fn a_refused_item_leaves_the_page_in_the_list() {
    let mut h = atlas_of(vec![child("one")], vec![]);
    h.api.fail_next("ApplyBatch", "refused");
    place(&mut h, "a/b/one", 0.0, 0.0);

    assert_eq!(bounds_of(&h, "a/b/one"), None);
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
    assert!(!h.api.called("GetPageById"));
}

// 配置済みのカードは、移動してもその大きさを保つ。
#[test]
fn a_placed_card_keeps_its_size_when_it_moves() {
    let mut h = atlas_with(&[("one", Some("0,0,200,100"))]);
    place(&mut h, "a/b/one", 1.0, 0.0);

    assert_eq!(upserts(&h)[0].2, "1,0,200,100");
}

// 拡大縮小されたカードは位置を保ち、倍率は高さとして保存される。
#[test]
fn a_scaled_card_keeps_its_place_and_stores_its_scale_as_its_height() {
    let mut h = atlas_with(&[("one", Some("10,-20,0,40"))]);
    scale(&mut h, "a/b/one", 1.5);

    // A width of 0 stays 0: the label still sets it
    assert_eq!(upserts(&h)[0].2, "10,-20,0,60");
    assert_eq!(Bounds::parse("10,-20,0,60").unwrap().scale(), 1.5);
}

// 拡大縮小されたカードの幅は0になり、ラベルに委ねられる。
#[test]
fn a_scaled_card_leaves_its_width_to_the_label() {
    let mut h = atlas_with(&[("one", Some("0,0,200,80"))]);
    scale(&mut h, "a/b/one", 1.0);

    assert_eq!(upserts(&h)[0].2, "0,0,0,40");
}

// カードは半分より小さくは縮まず、拡大には上限がない。
#[test]
fn a_card_shrinks_no_smaller_than_half_and_grows_without_limit() {
    let mut h = atlas_with(&[("one", Some("0,0,0,40")), ("two", Some("0,0,0,40"))]);
    scale(&mut h, "a/b/one", 0.1);
    scale(&mut h, "a/b/two", 10.0);

    let saved: Vec<String> = upserts(&h).into_iter().map(|u| u.2).collect();
    assert_eq!(saved, ["0,0,0,20", "0,0,0,400"]);
}

// 未配置のカードや倍率の変わらないカードは拡大縮小されず、何も保存されない。
#[test]
fn an_unplaced_or_unchanged_card_is_not_scaled() {
    let mut h = atlas_with(&[("one", None), ("two", Some("0,0,0,40"))]);
    scale(&mut h, "a/b/one", 2.0);
    scale(&mut h, "a/b/two", 1.0);

    assert!(upserts(&h).is_empty());
}

// 配置の保存が拒否されると、カードは未配置の状態に戻され、アラートが表示される。
#[test]
fn a_refused_place_puts_the_card_back() {
    let mut h = atlas_with(&[("one", None)]);
    h.api.fail_next("UpsertItemProperty", "refused");
    place(&mut h, "a/b/one", 0.0, 0.0);

    assert_eq!(bounds_of(&h, "a/b/one"), None);
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
}

// 取り除かれたカードは境界のプロパティを失い、再び未配置のまま待機する。
#[test]
fn a_removed_card_loses_its_bounds_and_waits_unplaced_again() {
    let mut h = atlas_with(&[("one", Some("0,0,120,40"))]);
    h.send(Event::RemoveAtlasCard {
        page_id: "a/b/one".into(),
    });

    assert_eq!(bounds_of(&h, "a/b/one"), None);
    assert_eq!(
        removed_properties(&h),
        vec![("i-one".to_string(), "atlas_bounds".to_string())]
    );
}

// 取り除きが拒否されると、カードはキャンバス上の元の位置に戻され、アラートが表示される。
#[test]
fn a_refused_remove_puts_the_card_back_on_the_canvas() {
    let mut h = atlas_with(&[("one", Some("0,0,120,40"))]);
    h.api.fail_next("DeleteItemProperty", "refused");
    h.send(Event::RemoveAtlasCard {
        page_id: "a/b/one".into(),
    });

    assert_eq!(
        bounds_of(&h, "a/b/one"),
        Some(Bounds {
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 40.0
        })
    );
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
}

// カードのアイコンは、そのページのタグから決まる。タグがなければ空である。
#[test]
fn a_card_draws_its_icon_from_the_tags_of_its_page() {
    use crate::screens::atlas::icon_tags;
    let mut page = doc("notes/", "post");
    assert!(icon_tags(&page, "tags/").is_empty());
    page.properties
        .insert("tags".into(), ["cardbox", "card"].into());
    assert_eq!(icon_tags(&page, "tags/"), ["cardbox", "card"]);
}

// タグページのアイコンは、タグディレクトリからの相対パスで表した自身のタグから決まる。
#[test]
fn a_tag_page_draws_its_icon_from_its_own_tag() {
    use crate::screens::atlas::icon_tags;
    assert_eq!(icon_tags(&doc("tags/", "CG"), "tags/"), ["CG"]);
    assert_eq!(
        icon_tags(&doc("tags/DailyReport/", "WPF"), "tags/"),
        ["DailyReport/WPF"]
    );
}

// フォルダのアイコンは、タグページのフォルダならそのタグから、それ以外ならインデックスページのタグか、なければフォルダ名から決まる。タグフォルダ自体はタグではない。
#[test]
fn a_folder_draws_its_icon_from_its_tag_its_tags_or_its_name() {
    use crate::screens::atlas::icon_tags;
    // A folder of tag pages is a tag too
    assert_eq!(
        icon_tags(&doc("tags/DailyReport/", "index"), "tags/"),
        ["DailyReport"]
    );
    assert_eq!(icon_tags(&doc("tags/a/b/", "index"), "tags/"), ["a/b"]);
    // Any other folder: the tags of its index page, or else its name
    assert_eq!(icon_tags(&doc("notes/rust/", "index"), "tags/"), ["rust"]);
    let mut tagged = doc("notes/rust/", "index");
    tagged.properties.insert("tags".into(), ["lang"].into());
    assert_eq!(icon_tags(&tagged, "tags/"), ["lang"]);
    // The folder of the tags itself is no tag
    assert_eq!(icon_tags(&doc("tags/", "index"), "tags/"), ["tags"]);
}

// アトラスのカードは、そのページが公開済みかどうかを知っている。
#[test]
fn an_atlas_card_knows_whether_its_page_is_published() {
    let mut published = child("one");
    published
        .properties
        .insert("published".into(), "2026-10-01T00:00:00Z".into());
    let h = atlas_of(
        vec![published, child("two")],
        vec![
            card_item("one", 1, Some("0,0,0,40")),
            card_item("two", 1, Some("0,0,0,40")),
        ],
    );

    let cards: Vec<_> = h
        .view()
        .atlas_cards
        .into_iter()
        .map(|c| (c.page_id, c.published))
        .collect();
    assert_eq!(
        cards,
        [
            ("a/b/one".to_string(), true),
            ("a/b/two".to_string(), false)
        ]
    );
}

// 座標のどちらかが数でない点には、カードもリンクも吹き出しも置かれず、吹き出しも動かない。
#[test]
fn nothing_is_placed_at_a_point_that_is_no_number() {
    let mut h = atlas_of(
        vec![child("one")],
        vec![
            card_item("one", 1, Some("0,0,0,40")),
            atlas_item(
                "c",
                1,
                "Hello",
                &[("atlas_type", "callout"), ("atlas_bounds", "10,20,0,40")],
            ),
        ],
    );
    for (x, y) in [(f64::NAN, 0.0), (0.0, f64::INFINITY)] {
        place(&mut h, "a/b/one", x, y);
        h.send(Event::AddAtlasCallout {
            x,
            y,
            side: "top".into(),
            text: "Hi".into(),
        });
        h.send(Event::MoveAtlasCallout {
            key: "c".into(),
            x,
            y,
        });
    }

    assert!(upserts(&h).is_empty());
    assert!(batches(&h).is_empty());
}
