//! The atlas page of a directory and the map button that switches to it.

use crate::app::{Event, PageState, Screen};
use crate::effects::{ApiOp, ApiOutput, AtlasChange, UiOp};
use crate::harness::Harness;
use crate::model::{Item, Page};
use crate::screens::atlas::{Bounds, Callout, GroupTitle};
use crate::screens::page::{page_title, ATLAS_NAME};

fn doc(dir_path: &str, name: &str) -> Page {
    Page {
        id: format!("{dir_path}{name}"),
        dir_path: dir_path.into(),
        name: name.into(),
        page_type: "doc".into(),
        ..Page::default()
    }
}

fn showing(page: Page) -> Harness {
    let mut h = Harness::new();
    h.model.screen = Screen::Page(PageState {
        page_name: page.canonical_path(),
        page: Some(page),
        ..PageState::default()
    });
    h
}

// インデックスページのマップボタンは、そのディレクトリのアトラスを開く。
#[test]
fn the_map_button_of_an_index_page_opens_the_atlas_of_its_directory() {
    let mut h = showing(doc("a/b/", "index"));
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b/index_atlas");
}

// ルートのインデックスページのマップボタンは、ルートのアトラスを開く。
#[test]
fn the_map_button_of_the_root_index_opens_the_root_atlas() {
    let mut h = showing(doc("", "index"));
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/index_atlas");
}

// アトラスページのマップボタンは、インデックスページに戻る。
#[test]
fn the_map_button_of_an_atlas_page_goes_back_to_the_index_page() {
    let mut h = showing(doc("a/b/", ATLAS_NAME));
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b/index");
}

// アトラスページのタイトルは、そのディレクトリ名から付けられる。ルートのアトラスでは "(root)/" と表される。
#[test]
fn an_atlas_page_is_titled_by_its_directory() {
    assert_eq!(page_title(&doc("a/b/", ATLAS_NAME)), "Atlas of a/b/");
    assert_eq!(page_title(&doc("", ATLAS_NAME)), "Atlas of (root)/");
}

fn index_with_atlas() -> Harness {
    let mut h = showing(doc("a/b/", "index"));
    h.api.answer_next(
        "GetChildPages",
        Ok(ApiOutput::Pages(vec![doc("a/b/", ATLAS_NAME)])),
    );
    h
}

// アトラスを持つインデックスページの降格では、まず確認が求められ、承諾後にアトラスが削除されてページ名が変更される。
#[test]
fn demoting_an_index_page_with_an_atlas_asks_first_then_deletes_the_atlas_and_renames() {
    let mut h = index_with_atlas();
    h.confirm = true;
    h.send(Event::DemoteIndex);

    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Confirm(_))));
    let deleted: Vec<_> = h
        .api
        .calls_named("DeletePage")
        .into_iter()
        .map(|op| match op {
            ApiOp::DeletePage { page_id } => page_id.clone(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(deleted, vec!["a/b/index_atlas"]);
    assert!(h.api.called("UpdatePage"));
}

// 確認に「いいえ」と答えると、アトラスもインデックスページもそのまま残される。
#[test]
fn a_no_to_the_question_keeps_the_atlas_and_the_index_page() {
    let mut h = index_with_atlas();
    h.confirm = false;
    h.send(Event::DemoteIndex);

    assert!(!h.api.called("DeletePage"));
    assert!(!h.api.called("UpdatePage"));
}

// アトラスを持たないインデックスページは、確認なしに名前が変更される。
#[test]
fn an_index_page_without_an_atlas_is_renamed_without_a_question() {
    let mut h = showing(doc("a/b/", "index"));
    h.send(Event::DemoteIndex);

    assert!(!h.ui.iter().any(|op| matches!(op, UiOp::Confirm(_))));
    assert!(!h.api.called("DeletePage"));
    assert!(h.api.called("UpdatePage"));
}

// ---- the cards on the canvas ----

const ATLAS_ID: &str = "a/b/index_atlas";

/// An item of the atlas page with the given properties
fn atlas_item(id: &str, level: i32, body: &str, props: &[(&str, &str)]) -> Item {
    let mut item = crate::testing::plain_item(id, level, body);
    item.page_id = ATLAS_ID.into();
    for (key, value) in props {
        item.properties.insert((*key).into(), (*value).into());
    }
    item
}

/// The item of the card of the page `a/b/<name>`, `i-<name>`, placed by the bounds if any
fn card_item(name: &str, level: i32, bounds: Option<&str>) -> Item {
    let page_id = format!("a/b/{name}");
    let mut props = vec![("atlas_page", page_id.as_str())];
    if let Some(b) = bounds {
        props.push(("atlas_bounds", b));
    }
    atlas_item(
        &format!("i-{name}"),
        level,
        &format!("[[{page_id}]]"),
        &props,
    )
}

fn child(name: &str) -> Page {
    doc("a/b/", name)
}

/// The atlas of `a/b/` showing the pages and the items of its outline
fn atlas_of(pages: Vec<Page>, items: Vec<Item>) -> Harness {
    let mut h = showing(doc("a/b/", ATLAS_NAME));
    h.model.sidebar.index_pages = pages;
    h.model.state.set_items(items);
    h
}

/// The atlas of `a/b/` with a page and its card at level 1 for each name, placed by the bounds
fn atlas_with(cards: &[(&str, Option<&str>)]) -> Harness {
    atlas_of(
        cards.iter().map(|(name, _)| child(name)).collect(),
        cards
            .iter()
            .map(|(name, bounds)| card_item(name, 1, *bounds))
            .collect(),
    )
}

fn upserts(h: &Harness) -> Vec<(String, String, String)> {
    h.api
        .calls_named("UpsertItemProperty")
        .into_iter()
        .map(|op| match op {
            ApiOp::UpsertItemProperty {
                item_id,
                key,
                value,
                ..
            } => (item_id.clone(), key.clone(), value.clone()),
            _ => unreachable!(),
        })
        .collect()
}

fn changes(h: &Harness) -> Vec<AtlasChange> {
    h.api
        .calls_named("ChangeAtlas")
        .into_iter()
        .map(|op| match op {
            ApiOp::ChangeAtlas { atlas_id, change } => {
                assert_eq!(atlas_id, ATLAS_ID);
                change.clone()
            }
            _ => unreachable!(),
        })
        .collect()
}

fn bounds_of(h: &Harness, page_id: &str) -> Option<Bounds> {
    h.view()
        .atlas_cards
        .into_iter()
        .find(|c| c.page_id == page_id)
        .and_then(|c| c.bounds)
}

fn place(h: &mut Harness, card_id: &str, x: f64, y: f64) {
    h.send(Event::PlaceAtlasCard {
        card_id: card_id.into(),
        x,
        y,
    });
}

fn scale(h: &mut Harness, card_id: &str, scale: f64) {
    h.send(Event::ScaleAtlasCard {
        card_id: card_id.into(),
        scale,
    });
}

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

// カードの位置は、そのページを指すアイテムから読み取られる。同じページへのリンクを持つだけの他のアイテムの境界は使われない。
#[test]
fn the_place_of_a_card_is_read_from_the_item_of_its_page() {
    let h = atlas_of(
        vec![child("one"), child("two")],
        vec![
            card_item("two", 1, Some("1,2,0,40")),
            atlas_item("other", 1, "[[a/b/one]]", &[("atlas_bounds", "9,9,0,40")]),
        ],
    );

    assert_eq!(bounds_of(&h, "a/b/one"), None);
    assert_eq!(bounds_of(&h, "a/b/two").map(|b| b.x), Some(1.0));
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

// ページに対応するアイテムがまだないカードは配置されず、何も保存されない。
#[test]
fn a_card_whose_page_has_no_item_yet_is_not_placed() {
    let mut h = atlas_of(vec![child("one")], vec![]);
    place(&mut h, "a/b/one", 0.0, 0.0);

    assert!(upserts(&h).is_empty());
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

fn removed_properties(h: &Harness) -> Vec<(String, String)> {
    h.api
        .calls_named("DeleteItemProperty")
        .into_iter()
        .map(|op| match op {
            ApiOp::DeleteItemProperty { item_id, key } => (item_id.clone(), key.clone()),
            _ => unreachable!(),
        })
        .collect()
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

// アトラスの上へボタンは、一つ上のディレクトリのアトラスを開く。
#[test]
fn the_up_button_of_an_atlas_opens_the_atlas_of_the_directory_above() {
    let mut h = showing(doc("a/b/", ATLAS_NAME));
    h.send(Event::AtlasUp);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/index_atlas");
}

// 最上位ディレクトリの上へボタンは、ルートのアトラスを開く。
#[test]
fn the_up_button_of_a_top_directory_opens_the_root_atlas() {
    let mut h = showing(doc("a/", ATLAS_NAME));
    h.send(Event::AtlasUp);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/index_atlas");
}

// ルートのアトラスには上のディレクトリがなく、上へボタンでは遷移しない。
#[test]
fn the_root_atlas_has_no_directory_above() {
    let mut h = showing(doc("", ATLAS_NAME));
    h.send(Event::AtlasUp);

    assert!(h.navigations.is_empty());
}

// ---- the view a folder opens at ----

fn visit(h: &mut Harness, name: &str) {
    h.send(Event::Route {
        route: crate::app::Route::Page {
            name: name.into(),
            fragment: None,
        },
        today: "2031-01-05".into(),
    });
}

fn go_to(h: &mut Harness, path: &str) -> String {
    h.send(Event::GoToPage {
        path: path.into(),
        new_tab: false,
        new_window: false,
    });
    h.navigations.last().unwrap().url.clone()
}

// 最後にマップとして見たフォルダは、そのアトラスとして開かれる。他のフォルダはインデックスページのまま開かれる。
#[test]
fn a_folder_last_seen_as_a_map_opens_as_its_atlas() {
    let mut h = Harness::new();
    visit(&mut h, "a/b/index_atlas");

    assert_eq!(go_to(&mut h, "a/b/index"), "/pages/a/b/index_atlas");
    // Another folder keeps its index page
    assert_eq!(go_to(&mut h, "a/c/index"), "/pages/a/c/index");
}

// 最後にインデックスとして見たフォルダは、再びインデックスとして開かれる。
#[test]
fn a_folder_last_seen_as_its_index_opens_as_its_index_again() {
    let mut h = Harness::new();
    visit(&mut h, "a/b/index_atlas");
    visit(&mut h, "a/b/index");

    assert_eq!(go_to(&mut h, "a/b/index"), "/pages/a/b/index");
}

// ルートとタグフォルダも、最後に見た形で開かれる。
#[test]
fn the_root_and_the_tags_folder_open_as_they_were_last_seen() {
    let mut h = Harness::new();
    h.model.config.tags_dir = "tags/".into();
    visit(&mut h, "index_atlas");
    visit(&mut h, "tags/index_atlas");

    h.send(Event::NavigateToRoot);
    assert_eq!(h.navigations.last().unwrap().url, "/pages/index_atlas");
    h.send(Event::NavigateToTags);
    assert_eq!(h.navigations.last().unwrap().url, "/pages/tags/index_atlas");
}

// フォルダをマップとして見た後でも、マップボタンはインデックスページに戻る。
#[test]
fn the_map_button_still_goes_back_to_the_index_page() {
    let mut h = Harness::new();
    visit(&mut h, "a/b/index_atlas");
    h.model.screen = Screen::Page(PageState {
        page_name: "a/b/index_atlas".into(),
        page: Some(doc("a/b/", ATLAS_NAME)),
        ..PageState::default()
    });
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b/index");
}

// マップとして見たフォルダの一覧はシェルに保存され、復元されるとそのフォルダはアトラスとして開かれる。
#[test]
fn the_folders_seen_as_maps_are_kept_by_the_shell_and_restored() {
    let mut h = Harness::new();
    visit(&mut h, "a/b/index_atlas");
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::SaveAtlasDirs(dirs) if dirs == &vec!["a/b/".to_string()])));
    visit(&mut h, "a/b/index");
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::SaveAtlasDirs(dirs) if dirs.is_empty())));

    let mut h = Harness::new();
    h.send(Event::AtlasDirsRestored(vec!["a/b/".into()]));
    assert_eq!(go_to(&mut h, "a/b/index"), "/pages/a/b/index_atlas");
}

// ---- the groups ----

/// A group item `g`, its title the body, with the cards of one and two under it, and the card
/// of three alone
fn atlas_with_group(title: &str, side: Option<&str>) -> Harness {
    let mut props = vec![("atlas_type", "group")];
    if let Some(side) = side {
        props.push(("atlas_side", side));
    }
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

fn group(h: &mut Harness, card_ids: &[&str], target_id: &str) {
    h.send(Event::GroupAtlasCards {
        card_ids: card_ids.iter().map(|id| id.to_string()).collect(),
        target_id: target_id.into(),
    });
}

fn ungroup(h: &mut Harness, card_ids: &[&str]) {
    h.send(Event::UngroupAtlasCards {
        card_ids: card_ids.iter().map(|id| id.to_string()).collect(),
    });
}

// グループアイテムの下にあるカードは、そのグループに属する。
#[test]
fn a_card_under_a_group_item_belongs_to_the_group() {
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

// ---- the outline written by the server ----

fn page_changed(h: &mut Harness, page_id: &str, source: &str) {
    h.send(Event::Sse(crate::effects::SseEvent::Message(format!(
        r#"{{"type":"pagesUpdated","pageId":"{page_id}","source":"{source}"}}"#
    ))));
}

// 表示中のアトラスは、サーバーがそのアウトラインを書き換えると、アウトラインを取得し直す。
#[test]
fn the_shown_atlas_takes_its_outline_again_when_the_server_writes_it() {
    // No item is shown yet, as on an atlas whose directory had no pages
    let mut h = atlas_of(vec![], vec![]);
    h.api.answer_next(
        "GetPageById",
        Ok(ApiOutput::Page(crate::model::PageWithItems {
            page: doc("a/b/", ATLAS_NAME),
            items: vec![atlas_item(
                "l",
                1,
                "[[x/Target]]",
                &[("atlas_bounds", "1,2,0,40")],
            )],
            ..Default::default()
        })),
    );
    page_changed(&mut h, ATLAS_ID, "server");

    assert_eq!(h.view().atlas_links.len(), 1);
}

// アプリ内で行われた変更や他のページの変更では、アウトラインは取得されない。
#[test]
fn the_outline_is_not_fetched_for_a_change_made_in_the_app_or_on_another_page() {
    let mut h = atlas_with(&[("one", None)]);
    page_changed(&mut h, ATLAS_ID, "app");
    page_changed(&mut h, "a/other_atlas", "server");

    assert!(!h.api.called("GetPageById"));
}

// カードのアイコンは、そのページのタグから決まる。タグがなければ空である。
#[test]
fn a_card_draws_its_icon_from_the_tags_of_its_page() {
    use crate::screens::atlas::icon_tags;
    let mut page = doc("notes/", "post");
    assert!(icon_tags(&page, "tags/").is_empty());
    page.properties
        .insert("tags".into(), serde_json::json!(["cardbox", "card"]));
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
    tagged
        .properties
        .insert("tags".into(), serde_json::json!(["lang"]));
    assert_eq!(icon_tags(&tagged, "tags/"), ["lang"]);
    // The folder of the tags itself is no tag
    assert_eq!(icon_tags(&doc("tags/", "index"), "tags/"), ["tags"]);
}

// ---- the callouts ----

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

/// The parent of the callouts and the given callouts under it
fn atlas_with_callouts(callouts: Vec<Item>) -> Harness {
    let mut items = vec![atlas_item(
        "cp",
        1,
        "callout",
        &[("atlas_type", "callouts")],
    )];
    items.extend(callouts);
    atlas_of(vec![], items)
}

fn callout_item(id: &str, text: &str, bounds: &str) -> Item {
    atlas_item(
        id,
        2,
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

// 吹き出しは吹き出しの親アイテムの下にあるアイテムであり、位置・倍率・辺・テキストを持って表示される。
#[test]
fn a_callout_is_an_item_under_the_parent_of_the_callouts() {
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
        atlas_item("a", 2, "Hi", &[("atlas_type", "callout")]),
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

// 上へボタンには一つ上のディレクトリ名が表示される。最上位ディレクトリでは "(root)/" である。
#[test]
fn the_up_button_names_the_directory_above() {
    use crate::screens::atlas::parent_dir_label;
    assert_eq!(parent_dir_label("a/b/"), "a/");
    assert_eq!(parent_dir_label("a/b/c/"), "a/b/");
    assert_eq!(parent_dir_label("a/"), "(root)/");
}

// アトラスのカードは、そのページが公開済みかどうかを知っている。
#[test]
fn an_atlas_card_knows_whether_its_page_is_published() {
    let mut published = child("one");
    published.properties.insert(
        "published".into(),
        serde_json::json!("2026-10-01T00:00:00Z"),
    );
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

// ---- the link cards ----

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

// 本文がリンク一つだけで配置済みのアトラスのアイテムはリンクカードである。未配置のもの、リンク以外の文字を含むもの、リンクが複数あるもの、ページカードは除かれる。
#[test]
fn an_item_of_the_atlas_that_is_one_placed_link_is_a_link_card() {
    let h = atlas_of(
        vec![],
        vec![
            link_item("a", "[[x/Target]]", Some("10,20,0,40")),
            link_item("b", "[[x/Unplaced]]", None),
            link_item("c", "see [[x/Target]]", Some("0,0,0,40")),
            link_item("d", "[[x/One]] [[x/Two]]", Some("0,0,0,40")),
            card_item("one", 1, Some("0,0,0,40")),
        ],
    );

    let links = h.view().atlas_links;
    assert_eq!(links.len(), 1);
    assert_eq!(
        (links[0].item_id.as_str(), links[0].target.as_str()),
        ("a", "x/Target")
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
            atlas_item("g", 1, "Tools", &[("atlas_type", "group")]),
            card_item("one", 2, Some("0,0,0,40")),
            atlas_item("l", 2, "[[x/Target]]", &[("atlas_bounds", "9,0,0,40")]),
        ],
    );

    let link = &h.view().atlas_links[0];
    assert_eq!(link.group.as_deref(), Some("g"));
    assert_eq!(
        link.group_title,
        Some(GroupTitle {
            side: "top".into(),
            text: "Tools".into()
        })
    );
}

// 追加されたリンクは、丸めた位置とともにサーバーによって書き込まれ、返されたアウトラインが表示される。リンク先の括弧と空白は取り除かれる。
#[test]
fn an_added_link_is_written_by_the_server_with_its_place() {
    let mut h = atlas_of(vec![], vec![]);
    h.api.answer_next(
        "ChangeAtlas",
        Ok(ApiOutput::Items(vec![link_item(
            "new",
            "[[x/Target]]",
            Some("10,-21,0,40"),
        )])),
    );
    h.send(Event::AddAtlasLink {
        x: 10.4,
        y: -20.6,
        target: " [[x/Target]] ".into(),
    });

    assert_eq!(
        changes(&h),
        [AtlasChange::AddLink {
            target: "x/Target".into(),
            bounds: "10,-21,0,40".into()
        }]
    );
    let links = h.view().atlas_links;
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "x/Target");
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

    assert!(changes(&h).is_empty());
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
        changes(&h),
        [AtlasChange::Group {
            cards: vec!["l".into()],
            target: "a/b/one".into()
        }]
    );
}

// 取り除かれたリンクカードは、サーバーによって取り去られる。
#[test]
fn a_removed_link_card_is_taken_away_by_the_server() {
    let mut h = atlas_with_link();
    h.send(Event::RemoveAtlasLink {
        item_id: "l".into(),
    });

    assert_eq!(
        changes(&h),
        [AtlasChange::Remove {
            item_id: "l".into()
        }]
    );
}

// リンクとして取り除かれるのはリンクカードだけである。ページカードでは何も送られない。
#[test]
fn only_a_link_card_is_removed_as_a_link() {
    let mut h = atlas_with_link();
    h.send(Event::RemoveAtlasLink {
        item_id: "i-one".into(),
    });

    assert!(changes(&h).is_empty());
}

// リンクカードを編集すると、新しいページへのリンクになる。リンク先の括弧と空白は取り除かれる。
#[test]
fn editing_a_link_card_links_it_to_the_new_page() {
    let mut h = atlas_with_link();
    h.send(Event::SetAtlasLink {
        item_id: "l".into(),
        target: " [[y/Other]] ".into(),
    });

    assert_eq!(
        changes(&h),
        [AtlasChange::SetLink {
            item_id: "l".into(),
            target: "y/Other".into()
        }]
    );
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

    assert!(changes(&h).is_empty());
    assert_eq!(h.view().atlas_links[0].target, "x/Target");
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
