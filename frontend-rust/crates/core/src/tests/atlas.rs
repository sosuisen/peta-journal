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

#[test]
fn the_map_button_of_an_index_page_opens_the_atlas_of_its_directory() {
    let mut h = showing(doc("a/b/", "index"));
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b/index_atlas");
}

#[test]
fn the_map_button_of_the_root_index_opens_the_root_atlas() {
    let mut h = showing(doc("", "index"));
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/index_atlas");
}

#[test]
fn the_map_button_of_an_atlas_page_goes_back_to_the_index_page() {
    let mut h = showing(doc("a/b/", ATLAS_NAME));
    h.send(Event::ToggleAtlas);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/b/index");
}

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

#[test]
fn a_no_to_the_question_keeps_the_atlas_and_the_index_page() {
    let mut h = index_with_atlas();
    h.confirm = false;
    h.send(Event::DemoteIndex);

    assert!(!h.api.called("DeletePage"));
    assert!(!h.api.called("UpdatePage"));
}

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

#[test]
fn a_folder_card_opens_the_atlas_of_the_folder_and_a_page_card_the_page() {
    let h = atlas_of(vec![doc("a/b/c/", "index"), child("one")], vec![]);

    let cards = h.view().atlas_cards;
    assert_eq!(
        cards.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
        vec!["a/b/c/index_atlas", "a/b/one"]
    );
}

#[test]
fn only_an_atlas_page_has_cards() {
    let mut h = showing(doc("a/b/", "index"));
    h.model.sidebar.index_pages = vec![child("one")];

    assert!(h.view().atlas_cards.is_empty());
}

#[test]
fn bounds_that_are_not_four_numbers_leave_the_card_unplaced() {
    assert_eq!(Bounds::parse("1,2,3"), None);
    assert_eq!(Bounds::parse("1,2,x,4"), None);
    assert_eq!(
        Bounds::parse(" 1, 2.5,3 ,4 "),
        Some(Bounds {
            x: 1.0,
            y: 2.5,
            width: 3.0,
            height: 4.0
        })
    );
}

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

#[test]
fn a_card_whose_page_has_no_item_yet_is_not_placed() {
    let mut h = atlas_of(vec![child("one")], vec![]);
    place(&mut h, "a/b/one", 0.0, 0.0);

    assert!(upserts(&h).is_empty());
}

#[test]
fn a_placed_card_keeps_its_size_when_it_moves() {
    let mut h = atlas_with(&[("one", Some("0,0,200,100"))]);
    place(&mut h, "a/b/one", 1.0, 0.0);

    assert_eq!(upserts(&h)[0].2, "1,0,200,100");
}

#[test]
fn a_scaled_card_keeps_its_place_and_stores_its_scale_as_its_height() {
    let mut h = atlas_with(&[("one", Some("10,-20,0,40"))]);
    scale(&mut h, "a/b/one", 1.5);

    // A width of 0 stays 0: the label still sets it
    assert_eq!(upserts(&h)[0].2, "10,-20,0,60");
    assert_eq!(Bounds::parse("10,-20,0,60").unwrap().scale(), 1.5);
}

#[test]
fn a_scaled_card_leaves_its_width_to_the_label() {
    let mut h = atlas_with(&[("one", Some("0,0,200,80"))]);
    scale(&mut h, "a/b/one", 1.0);

    assert_eq!(upserts(&h)[0].2, "0,0,0,40");
}

#[test]
fn a_card_shrinks_no_smaller_than_half_and_grows_without_limit() {
    let mut h = atlas_with(&[("one", Some("0,0,0,40")), ("two", Some("0,0,0,40"))]);
    scale(&mut h, "a/b/one", 0.1);
    scale(&mut h, "a/b/two", 10.0);

    let saved: Vec<String> = upserts(&h).into_iter().map(|u| u.2).collect();
    assert_eq!(saved, ["0,0,0,20", "0,0,0,400"]);
}

#[test]
fn an_unplaced_or_unchanged_card_is_not_scaled() {
    let mut h = atlas_with(&[("one", None), ("two", Some("0,0,0,40"))]);
    scale(&mut h, "a/b/one", 2.0);
    scale(&mut h, "a/b/two", 1.0);

    assert!(upserts(&h).is_empty());
}

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

#[test]
fn the_up_button_of_an_atlas_opens_the_atlas_of_the_directory_above() {
    let mut h = showing(doc("a/b/", ATLAS_NAME));
    h.send(Event::AtlasUp);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/a/index_atlas");
}

#[test]
fn the_up_button_of_a_top_directory_opens_the_root_atlas() {
    let mut h = showing(doc("a/", ATLAS_NAME));
    h.send(Event::AtlasUp);

    assert_eq!(h.navigations.last().unwrap().url, "/pages/index_atlas");
}

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

#[test]
fn a_folder_last_seen_as_a_map_opens_as_its_atlas() {
    let mut h = Harness::new();
    visit(&mut h, "a/b/index_atlas");

    assert_eq!(go_to(&mut h, "a/b/index"), "/pages/a/b/index_atlas");
    // Another folder keeps its index page
    assert_eq!(go_to(&mut h, "a/c/index"), "/pages/a/c/index");
}

#[test]
fn a_folder_last_seen_as_its_index_opens_as_its_index_again() {
    let mut h = Harness::new();
    visit(&mut h, "a/b/index_atlas");
    visit(&mut h, "a/b/index");

    assert_eq!(go_to(&mut h, "a/b/index"), "/pages/a/b/index");
}

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

#[test]
fn a_card_dropped_on_itself_or_on_no_card_makes_no_group() {
    let mut h = atlas_with(&[("one", Some("0,0,0,40"))]);
    group(&mut h, &["a/b/one"], "a/b/one");
    group(&mut h, &["a/b/one"], "a/b/none");

    assert!(changes(&h).is_empty());
}

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

#[test]
fn a_refused_group_leaves_the_outline_as_it_was() {
    let mut h = atlas_with_group("", None);
    h.api.fail_next("ChangeAtlas", "refused");
    group(&mut h, &["a/b/three"], "a/b/one");

    assert_eq!(bounds_of(&h, "a/b/three").map(|b| b.x), Some(200.0));
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
}

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

#[test]
fn a_card_in_no_group_leaves_nothing() {
    let mut h = atlas_with_group("", None);
    ungroup(&mut h, &["a/b/three"]);

    assert!(changes(&h).is_empty());
}

#[test]
fn the_id_of_a_group_picks_one_of_the_six_colors() {
    use crate::screens::atlas::group_color;
    let six = [
        "#F9A2A2", "#B5B5FF", "#94D094", "#E1A6E1", "#5FD3D3", "#C3C372",
    ];
    for id in [
        "g",
        "2026-09-29-10-00-00-AAAAAAA",
        "2026-09-29-10-00-01-AAAAAAB",
    ] {
        assert!(six.contains(&group_color(id).as_str()), "{id}");
        // The same group keeps its color
        assert_eq!(group_color(id), group_color(id));
    }
}

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

#[test]
fn the_outline_is_not_fetched_for_a_change_made_in_the_app_or_on_another_page() {
    let mut h = atlas_with(&[("one", None)]);
    page_changed(&mut h, ATLAS_ID, "app");
    page_changed(&mut h, "a/other_atlas", "server");

    assert!(!h.api.called("GetPageById"));
}

#[test]
fn a_card_draws_its_icon_from_the_tags_of_its_page() {
    use crate::screens::atlas::icon_tags;
    let mut page = doc("notes/", "post");
    assert!(icon_tags(&page, "tags/").is_empty());
    page.properties
        .insert("tags".into(), serde_json::json!(["cardbox", "card"]));
    assert_eq!(icon_tags(&page, "tags/"), ["cardbox", "card"]);
}

#[test]
fn a_tag_page_draws_its_icon_from_its_own_tag() {
    use crate::screens::atlas::icon_tags;
    assert_eq!(icon_tags(&doc("tags/", "CG"), "tags/"), ["CG"]);
    assert_eq!(
        icon_tags(&doc("tags/DailyReport/", "WPF"), "tags/"),
        ["DailyReport/WPF"]
    );
}

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

#[test]
fn an_icon_is_an_svg_of_the_size_asked() {
    use crate::screens::atlas::tag_icon_svg;
    let tags: Vec<String> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|t| t.to_string())
        .collect();
    let svg = tag_icon_svg(&tags, 24);
    assert!(svg.starts_with("<svg"), "{svg}");
    assert!(svg.contains(r#"width="24""#), "{svg}");
    // The same tags draw the same icon
    assert_eq!(svg, tag_icon_svg(&tags, 24));
}

#[test]
fn a_blob_is_a_light_blue_svg_drawn_from_the_tag() {
    use crate::screens::atlas::tag_blob_svg;
    let svg = tag_blob_svg("DailyReport", 100, false);
    assert!(svg.starts_with("<svg"), "{svg}");
    assert!(svg.contains(r##"fill="#CFE3FF""##), "{svg}");
    // The same tag draws the same blob, and another tag another one
    assert_eq!(svg, tag_blob_svg("DailyReport", 100, false));
    assert_ne!(svg, tag_blob_svg("JavaFX", 100, false));
}

#[test]
fn the_blob_of_a_link_card_is_light_orange() {
    use crate::screens::atlas::{link_blob_svg, tag_blob_svg};
    let svg = link_blob_svg("x/Target", 100);
    assert!(svg.contains(r##"fill="#FFD8A8""##), "{svg}");
    assert!(!svg.contains(r##"fill="#CFE3FF""##), "{svg}");
    // The same shape as the blob of a card of that name
    assert_eq!(
        svg.replace("#FFD8A8", "#CFE3FF"),
        tag_blob_svg("x/Target", 100, false)
    );
}

#[test]
fn a_blob_fills_its_svg_to_the_edges() {
    use crate::screens::atlas::tag_blob_svg;
    let svg = tag_blob_svg("DailyReport", 100, false);
    // The view box is cut to the blob and stretched to the size of the SVG
    assert!(svg.contains(r#"preserveAspectRatio="none""#), "{svg}");
    assert!(!svg.contains(r#"viewBox="-50 -50 100 100""#), "{svg}");
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

#[test]
fn a_callout_is_an_item_under_the_parent_of_the_callouts() {
    let h = atlas_with_callout();

    let shown = Callout {
        scale: 1.5,
        ..callout("c", 10.0, 20.0, "top", "Hello")
    };
    assert_eq!(h.view().atlas_callouts, [shown]);
}

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

#[test]
fn the_character_of_a_callout_is_a_face_of_its_text() {
    let svg = crate::screens::atlas::callout_face_svg("Hello", 64);

    assert!(svg.starts_with("<svg"), "{svg}");
    assert!(svg.contains("<path"), "{svg}");
    assert!(svg.contains(r#"stroke-width="3.5""#), "{svg}");
    assert!(!svg.contains(r#"stroke-width="5""#), "{svg}");
}

#[test]
fn the_up_button_names_the_directory_above() {
    use crate::screens::atlas::parent_dir_label;
    assert_eq!(parent_dir_label("a/b/"), "a/");
    assert_eq!(parent_dir_label("a/b/c/"), "a/b/");
    assert_eq!(parent_dir_label("a/"), "(root)/");
}

#[test]
fn the_blob_of_a_published_page_has_a_line_around_it() {
    use crate::screens::atlas::tag_blob_svg;
    let plain = tag_blob_svg("DailyReport", 100, false);
    let outlined = tag_blob_svg("DailyReport", 100, true);

    assert!(!plain.contains("stroke="), "{plain}");
    assert!(outlined.contains(r##"<path d="##), "{outlined}");
    assert!(
        outlined.contains(r#"vector-effect="non-scaling-stroke""#),
        "{outlined}"
    );
    assert!(
        outlined.contains(r#"preserveAspectRatio="none""#),
        "{outlined}"
    );
}

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

#[test]
fn a_link_card_is_named_by_the_last_part_of_its_target() {
    use crate::screens::atlas::link_label;
    assert_eq!(
        link_label("sosuisen.net/article/Picturesque Tags"),
        "Picturesque Tags"
    );
    assert_eq!(link_label("Top"), "Top");
    assert_eq!(link_label("a/b/index"), "b/");
    assert_eq!(link_label("index"), "(root)/");
}

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

#[test]
fn a_refused_move_of_a_link_puts_it_back() {
    let mut h = atlas_with_link();
    h.api.fail_next("UpsertItemProperty", "nope");
    place(&mut h, "l", 30.0, 0.0);

    assert_eq!(h.view().atlas_links[0].bounds.x, 10.0);
}

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

#[test]
fn only_a_link_card_is_removed_as_a_link() {
    let mut h = atlas_with_link();
    h.send(Event::RemoveAtlasLink {
        item_id: "i-one".into(),
    });

    assert!(changes(&h).is_empty());
}

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
