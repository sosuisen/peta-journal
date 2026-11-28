//! What waits beside the canvas: the pages that no card links to, and the items without a place.

use super::*;
use crate::app::Route;
use crate::model::DocPage;
use crate::view::UnplacedCard;

/// The atlas of `a/b/` that also knows the index page of the folder itself
fn atlas_with_index(pages: Vec<Page>, items: Vec<Item>) -> Harness {
    let mut h = atlas_of(pages, items);
    h.model.sidebar.index_page = Some(doc("a/b/", "index"));
    h
}

/// The ids of the pages that wait to be added
fn not_added(h: &Harness) -> Vec<String> {
    let waiting = h.view().atlas_unplaced.into_iter();
    waiting.filter(|c| !c.added).map(|c| c.id).collect()
}

/// The items that wait for a place
fn no_place(h: &Harness) -> Vec<UnplacedCard> {
    let waiting = h.view().atlas_unplaced.into_iter();
    waiting.filter(|c| c.added).collect()
}

fn waiting(id: &str, label: &str, path: Option<&str>, is_dir: bool) -> UnplacedCard {
    UnplacedCard {
        id: id.into(),
        label: label.into(),
        path: path.map(str::to_string),
        is_dir,
        added: true,
    }
}

// アトラスを開くと、コアはフォルダ自身のindexのページも受け取り、ほかのページより先に、未追加の項目に出す。
#[test]
fn the_index_of_the_folder_itself_waits_to_be_added_before_the_other_pages() {
    let mut h = Harness::new();
    let mut atlas = doc("a/b/", ATLAS_NAME);
    atlas.parent_id = "a/b/index".into();
    h.api.answer_next(
        "GetOrCreateDoc",
        Ok(ApiOutput::DocPage(DocPage {
            page: atlas,
            items: vec![],
            created: false,
        })),
    );
    h.api.answer_next(
        "GetPageById",
        Ok(ApiOutput::Page(PageWithItems {
            page: doc("a/b/", "index"),
            ..Default::default()
        })),
    );
    h.api.answer_next(
        "GetChildPages",
        Ok(ApiOutput::Pages(vec![
            child("one"),
            doc("a/b/", ATLAS_NAME),
        ])),
    );
    h.send(Event::Route {
        route: Route::Page {
            name: "a/b/index_atlas".into(),
            fragment: None,
        },
        today: "2031-01-05".into(),
    });

    assert_eq!(not_added(&h), ["a/b/index", "a/b/one"]);
}

// フォルダのindexを開いたときも、コアはそのindexのページを持つ。
#[test]
fn the_index_of_the_folder_is_kept_when_the_index_itself_is_open() {
    let mut h = Harness::new();
    h.api.answer_next(
        "GetOrCreateDoc",
        Ok(ApiOutput::DocPage(DocPage {
            page: doc("a/b/", "index"),
            items: vec![],
            created: false,
        })),
    );
    h.send(Event::Route {
        route: Route::Page {
            name: "a/b/index".into(),
            fragment: None,
        },
        today: "2031-01-05".into(),
    });

    let kept = h.model.sidebar.index_page.as_ref().map(|p| p.id.as_str());
    assert_eq!(kept, Some("a/b/index"));
}

// 未追加の項目は、どのカードからもリンクされていないページである。フォルダが先で、あとは一覧の並びである。
#[test]
fn the_pages_that_no_card_links_to_wait_to_be_added() {
    let h = atlas_of(
        vec![child("two"), child("one"), doc("a/b/c/", "index")],
        vec![card_item("two", 1, Some("1,2,0,40"))],
    );

    assert_eq!(not_added(&h), ["a/b/c/index", "a/b/one"]);
    let folder = h.view().atlas_unplaced.remove(0);
    assert_eq!(
        (folder.label.as_str(), folder.path.as_deref(), folder.is_dir),
        ("c/", Some("a/b/c/index_atlas"), true)
    );
}

// 座標のない項目がリンクしているページは、未追加の項目に出ない。同じページが2つのグループに出ないためである。
#[test]
fn a_page_that_an_item_without_a_place_links_to_does_not_wait_to_be_added() {
    let h = atlas_with(&[("one", None)]);

    assert!(not_added(&h).is_empty());
    assert_eq!(
        no_place(&h),
        [waiting("a/b/one", "one", Some("a/b/one"), false)]
    );
}

// 親の項目だけからリンクされているページも、リンクされている。未追加の項目に出ない。
#[test]
fn a_page_that_only_a_parent_links_to_does_not_wait_to_be_added() {
    let h = atlas_of(
        vec![child("one"), child("two")],
        vec![
            card_item("one", 1, None),
            card_item("two", 2, Some("1,2,0,40")),
        ],
    );

    assert!(not_added(&h).is_empty());
}

// コールアウトの本文のリンクは、カードのリンクではない。そのページは未追加の項目に出る。
#[test]
fn a_page_that_only_a_callout_names_waits_to_be_added() {
    let h = atlas_of(
        vec![child("one")],
        vec![atlas_item(
            "c",
            1,
            "[[a/b/one]]",
            &[("atlas_type", "callout"), ("atlas_bounds", "1,2,0,40")],
        )],
    );

    assert_eq!(not_added(&h), ["a/b/one"]);
}

// フォルダ自身のindexへのカードがあれば、そのindexは未追加の項目に出ず、ページのカードとして置かれる。
#[test]
fn the_index_of_the_folder_itself_has_a_card_as_any_page_of_the_folder() {
    let h = atlas_with_index(
        vec![],
        vec![atlas_item(
            "i",
            1,
            "[[a/b/index]]",
            &[("atlas_bounds", "1,2,0,40")],
        )],
    );

    assert!(not_added(&h).is_empty());
    assert_eq!(bounds_of(&h, "a/b/index").map(|b| b.x), Some(1.0));
    assert!(h.view().atlas_links.is_empty());
}

// 座標のない項目は、アトラスのページにあって置かれていないカードとコールアウトである。項目の順に並ぶ。
#[test]
fn the_cards_and_callouts_without_a_place_wait_in_the_order_of_their_items() {
    let h = atlas_of(
        vec![child("one"), doc("a/b/c/", "index")],
        vec![
            atlas_item("out", 1, "[[x/far|Far]]\nmore", &[]),
            atlas_item("gone", 1, "[[a/b/gone]]", &[]),
            atlas_item("text", 1, "plain **text**\nmore", &[]),
            card_item("one", 1, None),
            atlas_item("call", 1, " Hi \nthere", &[("atlas_type", "callout")]),
            atlas_item("f", 1, "[[a/b/c/index|Folder]]", &[]),
            atlas_item("placed", 1, "placed", &[("atlas_bounds", "1,2,0,40")]),
        ],
    );
    let mut h = h;
    h.model.sidebar.index_state = crate::view::LoadState::Loaded;

    assert_eq!(
        no_place(&h),
        [
            waiting("out", "Far", Some("x/far"), false),
            waiting("gone", "gone", None, false),
            waiting("text", "plain text", None, false),
            waiting("a/b/one", "one", Some("a/b/one"), false),
            waiting("call", "Hi", None, false),
            waiting("a/b/c/index", "Folder", Some("a/b/c/index_atlas"), true),
        ]
    );
}

// `atlas_bounds`のない親は描かれず、座標のない項目に出る。その下の項目もすべて、自分の`atlas_bounds`の有無によらず、座標のない項目に出て、アトラスには描かれない。座標のある親の下では、座標のない項目だけが出る。
#[test]
fn a_parent_without_bounds_waits_for_a_place_with_everything_under_it() {
    let placed = [("atlas_bounds", "1,2,0,40")];
    let h = atlas_of(
        vec![child("one")],
        vec![
            atlas_item("g", 1, "Tools", &[]),
            atlas_item("in", 2, "[[x/far|Far]]", &placed),
            card_item("one", 3, Some("1,2,0,40")),
            atlas_item("put", 2, "put", &placed),
            atlas_item(
                "call",
                2,
                "[[x/far]]",
                &[("atlas_type", "callout"), ("atlas_bounds", "1,2,0,40")],
            ),
            atlas_item("p", 1, "Placed", &placed),
            atlas_item("shown", 2, "shown", &placed),
            atlas_item("not", 2, "not", &[]),
        ],
    );

    assert_eq!(
        no_place(&h),
        [
            waiting("g", "Tools", None, false),
            waiting("in", "Far", Some("x/far"), false),
            waiting("a/b/one", "one", Some("a/b/one"), false),
            waiting("put", "put", None, false),
            // The text of a callout is no link, whatever it looks like
            waiting("call", "[[x/far]]", None, false),
            waiting("not", "not", None, false),
        ]
    );
    let view = h.view();
    assert_eq!(bounds_of(&h, "a/b/one"), None);
    let links: Vec<&str> = view
        .atlas_links
        .iter()
        .map(|l| l.item_id.as_str())
        .collect();
    assert_eq!(links, ["shown"]);
    assert!(view.atlas_callouts.is_empty());
}

// アトラスでないページでは、何も待たない。
#[test]
fn nothing_waits_on_a_page_that_is_no_atlas() {
    let mut h = showing(doc("a/b/", "index"));
    h.model.sidebar.index_pages = vec![child("one")];

    assert!(h.view().atlas_unplaced.is_empty());
}

// 座標のないコールアウトを一覧からアトラスに落とすと、その項目に位置が付き、コールアウトとして描かれる。
#[test]
fn a_callout_without_a_place_dropped_on_the_atlas_gets_its_place() {
    let mut h = atlas_of(
        vec![],
        vec![atlas_item("call", 1, "Hi", &[("atlas_type", "callout")])],
    );
    h.send(Event::DropAtlasCard {
        card_id: "call".into(),
        x: 100.0,
        y: 50.0,
        grab_x: 0.5,
        grab_y: 0.5,
    });

    assert_eq!(
        upserts(&h),
        vec![("call".into(), "atlas_bounds".into(), "100,50,0,40".into())]
    );
    assert!(batches(&h).is_empty());
    let callouts = h.view().atlas_callouts;
    assert_eq!((callouts[0].x, callouts[0].y), (100.0, 50.0));
    assert!(no_place(&h).is_empty());
}

// 座標のない親を一覧からアトラスに落とすと、親に`atlas_bounds`が付き、座標を持つ子が、親からの相対の位置に一緒に出る。子の`atlas_bounds`は変わらない。
#[test]
fn a_parent_without_a_place_dropped_on_the_atlas_gets_its_place_and_shows_what_is_under_it() {
    let mut h = atlas_of(
        vec![child("one")],
        vec![
            atlas_item("g", 1, "Tools", &[]),
            card_item("one", 2, Some("10,20,0,40")),
            atlas_item("not", 2, "not", &[]),
        ],
    );
    h.send(Event::DropAtlasCard {
        card_id: "g".into(),
        x: 100.0,
        y: 50.0,
        grab_x: 0.5,
        grab_y: 0.5,
    });

    assert_eq!(
        upserts(&h),
        vec![("g".into(), "atlas_bounds".into(), "100,50,0,40".into())]
    );
    assert!(batches(&h).is_empty());
    let card = h.view().atlas_cards.remove(0);
    assert_eq!(card.bounds.map(|b| (b.x, b.y)), Some((10.0, 20.0)));
    assert_eq!((card.groups[0].x, card.groups[0].y), (100.0, 50.0));
    let waiting: Vec<String> = no_place(&h).into_iter().map(|c| c.id).collect();
    assert_eq!(waiting, ["not"]);
}

// 親が描かれていない項目を一覧からアトラスに落とすと、1回の送信で、トップレベルのカードになる。いちばん外の親の木の末尾の次に、レベル1で入り、落とした点を`atlas_bounds`として持つ。下の項目は一緒に動く。最後の子が出た親は、同じ送信で辺を失う。
#[test]
fn an_item_under_a_parent_that_is_not_drawn_dropped_on_the_atlas_becomes_a_card_of_the_top() {
    let mut h = atlas_of(
        vec![],
        vec![
            atlas_item("parent", 1, "Parent", &[("atlas_side", "left")]),
            atlas_item("under", 2, "Under", &[]),
            atlas_item("deep", 3, "deep", &[("atlas_bounds", "5,5,0,40")]),
            atlas_item("last", 1, "last", &[("atlas_bounds", "9,9,0,40")]),
        ],
    );
    place(&mut h, "under", 10.4, 20.0);

    assert!(upserts(&h).is_empty());
    assert_eq!(
        batches(&h),
        [[
            leveled_at("under", 1, "10,20,0,40"),
            leveled("deep", 2),
            moved(&["under", "deep"], "parent"),
            unsided("parent"),
        ]]
    );
}

// 描かれている親の下の、描かれていない親の下の項目も、落とすとトップレベルに出る。いちばん外の親の木の末尾の次に入り、自分の大きさを保つ。親が描かれている項目は、親の下のまま、相対の座標で置かれる。
#[test]
fn an_item_comes_out_to_the_top_from_under_any_parent_that_is_not_drawn() {
    let mut h = atlas_of(
        vec![],
        vec![
            parent_item("outer", 1, "", &[("atlas_bounds", "100,100,0,40")]),
            atlas_item("inner", 2, "Inner", &[]),
            atlas_item("card", 3, "card", &[("atlas_bounds", "1,2,0,60")]),
            atlas_item("other", 3, "other", &[]),
            atlas_item("kept", 2, "kept", &[]),
        ],
    );
    place(&mut h, "kept", 130.0, 150.0);
    place(&mut h, "card", 7.0, 8.0);

    assert_eq!(
        batches(&h),
        [[leveled_at("card", 1, "7,8,0,60"), moved(&["card"], "kept")]]
    );
    assert_eq!(
        upserts(&h),
        vec![("kept".into(), "atlas_bounds".into(), "30,50,0,40".into())]
    );
}

// 座標のないリンクカードとノーマルカードを一覧からアトラスに落とすと、その項目に位置が付く。新しい項目は作られない。
#[test]
fn a_link_card_and_a_plain_card_without_a_place_dropped_on_the_atlas_get_their_places() {
    let mut h = atlas_of(
        vec![],
        vec![
            atlas_item("out", 1, "[[x/far]]", &[]),
            atlas_item("text", 1, "plain", &[]),
        ],
    );
    place(&mut h, "out", 1.0, 2.0);
    place(&mut h, "text", 3.0, 4.0);

    assert_eq!(
        upserts(&h),
        vec![
            ("out".into(), "atlas_bounds".into(), "1,2,0,40".into()),
            ("text".into(), "atlas_bounds".into(), "3,4,0,40".into()),
        ]
    );
    assert!(batches(&h).is_empty());
    assert!(no_place(&h).is_empty());
}

// フォルダ自身のindexのカードのパスは、アトラスではなく、indexのページである。子のフォルダのカードのパスは、そのアトラスである。
#[test]
fn the_card_of_the_index_of_the_folder_itself_names_the_index_page() {
    let h = atlas_with_index(vec![doc("a/b/c/", "index")], vec![]);

    let paths: Vec<Option<String>> = h
        .view()
        .atlas_unplaced
        .into_iter()
        .map(|c| c.path)
        .collect();
    assert_eq!(
        paths,
        [
            Some("a/b/index".to_string()),
            Some("a/b/c/index_atlas".to_string())
        ]
    );
}
