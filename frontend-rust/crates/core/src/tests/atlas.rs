//! The atlas page of a directory and the map button that switches to it.

use crate::app::{Event, PageState, Screen};
use crate::effects::{ApiOp, ApiOutput, UiOp};
use crate::harness::Harness;
use crate::model::Page;
use crate::screens::atlas::Bounds;
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
    h.api.answer_next("GetChildPages", Ok(ApiOutput::Pages(vec![doc("a/b/", ATLAS_NAME)])));
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

fn child(name: &str, bounds: Option<&str>) -> Page {
    let mut p = doc("a/b/", name);
    if let Some(b) = bounds {
        p.properties.insert("atlas_bounds".into(), b.into());
    }
    p
}

fn atlas_with(children: Vec<Page>) -> Harness {
    let mut h = showing(doc("a/b/", ATLAS_NAME));
    h.model.sidebar.index_pages = children;
    h
}

fn upserts(h: &Harness) -> Vec<(String, String, String)> {
    h.api
        .calls_named("UpsertPageProperty")
        .into_iter()
        .map(|op| match op {
            ApiOp::UpsertPageProperty { page_id, key, value, .. } => {
                (page_id.clone(), key.clone(), value.clone())
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

#[test]
fn a_page_without_bounds_waits_unplaced_and_a_page_with_them_lies_on_the_canvas() {
    let h = atlas_with(vec![child("one", None), child("two", Some("-80,-28,160,56"))]);

    let cards = h.view().atlas_cards;
    assert_eq!(cards.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(), vec!["one", "two"]);
    assert_eq!(cards[0].bounds, None);
    assert_eq!(cards[1].bounds, Some(Bounds { x: -80.0, y: -28.0, width: 160.0, height: 56.0 }));
}

#[test]
fn only_an_atlas_page_has_cards() {
    let mut h = showing(doc("a/b/", "index"));
    h.model.sidebar.index_pages = vec![child("one", None)];

    assert!(h.view().atlas_cards.is_empty());
}

#[test]
fn bounds_that_are_not_four_numbers_leave_the_card_unplaced() {
    assert_eq!(Bounds::parse("1,2,3"), None);
    assert_eq!(Bounds::parse("1,2,x,4"), None);
    assert_eq!(Bounds::parse(" 1, 2.5,3 ,4 "), Some(Bounds { x: 1.0, y: 2.5, width: 3.0, height: 4.0 }));
}

#[test]
fn a_dropped_card_takes_the_default_size_with_its_center_at_the_given_point() {
    let mut h = atlas_with(vec![child("one", None)]);
    h.send(Event::PlaceAtlasCard { page_id: "a/b/one".into(), x: 10.4, y: -20.0 });

    let expected = Bounds { x: 10.0, y: -20.0, width: 120.0, height: 40.0 };
    assert_eq!(bounds_of(&h, "a/b/one"), Some(expected));
    assert_eq!(
        upserts(&h),
        vec![("a/b/one".into(), "atlas_bounds".into(), "10,-20,120,40".into())]
    );
}

#[test]
fn a_placed_card_keeps_its_size_when_it_moves() {
    let mut h = atlas_with(vec![child("one", Some("0,0,200,100"))]);
    h.send(Event::PlaceAtlasCard { page_id: "a/b/one".into(), x: 0.0, y: 0.0 });

    assert_eq!(upserts(&h)[0].2, "0,0,200,100");
}

#[test]
fn a_refused_place_puts_the_card_back() {
    let mut h = atlas_with(vec![child("one", None)]);
    h.api.fail_next("UpsertPageProperty", "refused");
    h.send(Event::PlaceAtlasCard { page_id: "a/b/one".into(), x: 0.0, y: 0.0 });

    assert_eq!(bounds_of(&h, "a/b/one"), None);
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
}
