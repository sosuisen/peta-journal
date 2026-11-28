//! A link to `today` on the atlas: today's page can always be opened, so the link has a target
//! on any atlas, also on the root atlas, where `today` reads as a page of the folder.

use super::*;
use crate::app::Route;
use crate::keys::OpenIn;
use crate::screens::atlas::{CardTone, GroupLook};
use crate::view::{LoadState, UnplacedCard};

/// An item of the root atlas page
fn root_item(id: &str, level: i32, body: &str, bounds: Option<&str>) -> Item {
    let props: Vec<(&str, &str)> = bounds.map(|b| ("atlas_bounds", b)).into_iter().collect();
    let mut item = atlas_item(id, level, body, &props);
    item.page_id = doc("", ATLAS_NAME).id;
    item
}

/// The root atlas with the items of its outline, after the list of its pages is in
fn root_atlas_of(items: Vec<Item>) -> Harness {
    let mut h = showing(doc("", ATLAS_NAME));
    h.model.sidebar.index_pages = vec![doc("", "one")];
    h.model.sidebar.index_state = LoadState::Loaded;
    h.model.state.set_items(items);
    h
}

// ルートのアトラスで、todayへのカードは、オレンジのリンク先のあるカードである。同じフォルダにないほかのページへのカードは、灰色である。
#[test]
fn a_card_to_today_on_the_root_atlas_is_a_card_with_a_target() {
    let h = root_atlas_of(vec![
        root_item("t", 1, "[[today]]", Some("0,0,0,40")),
        root_item("g", 1, "[[gone]]", Some("9,0,0,40")),
    ]);

    let cards: Vec<(Option<String>, CardTone)> = h
        .view()
        .atlas_links
        .into_iter()
        .map(|l| (l.target, l.tone))
        .collect();
    assert_eq!(
        cards,
        [
            (Some("today".into()), CardTone::Link),
            (Some("gone".into()), CardTone::Missing),
        ]
    );
}

// todayへのリンクの親は、オレンジで、今日のページを開くパスを持つ。
#[test]
fn a_parent_that_links_to_today_opens_today() {
    let h = root_atlas_of(vec![
        root_item("pa", 1, "[[today]]", Some("0,0,0,40")),
        root_item("n", 2, "note", Some("0,0,0,40")),
    ]);

    assert_eq!(
        h.view().atlas_links[0].groups[0].look,
        GroupLook::Page {
            path: Some("today".into()),
            tone: CardTone::Link,
            icon_tags: vec![],
        }
    );
}

// 座標のないtodayへのカードは、座標のない項目として待ち、今日のページを開くパスを持つ。todayは未追加のページには出ない。
#[test]
fn a_card_to_today_without_a_place_waits_with_the_path_of_today() {
    let h = root_atlas_of(vec![
        root_item("one", 1, "[[one]]", Some("0,0,0,40")),
        root_item("t", 1, "[[today]]", None),
    ]);

    assert_eq!(
        h.view().atlas_unplaced,
        [UnplacedCard {
            id: "t".into(),
            label: "today".into(),
            path: Some("today".into()),
            is_dir: false,
            added: true,
        }]
    );
}

// todayのパスを開くと、今日のページのURLへ移る。
#[test]
fn going_to_the_path_of_today_opens_the_page_of_today() {
    let mut h = root_atlas_of(vec![]);

    h.send(Event::GoToPage {
        path: "today".into(),
        open_in: OpenIn::Here,
    });

    let url = &h.navigations.last().unwrap().url;
    assert_eq!(
        Route::parse_url(url),
        Some(Route::Page {
            name: "today".into(),
            fragment: None,
        })
    );
}
