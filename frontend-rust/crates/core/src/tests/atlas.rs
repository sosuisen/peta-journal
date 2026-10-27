//! The atlas page of a directory and the map button that switches to it.

use crate::app::{Event, PageState, Screen};
use crate::effects::{ApiOp, ApiOutput, ApiResult, UiOp};
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
fn a_folder_card_opens_the_atlas_of_the_folder_and_a_page_card_the_page() {
    let h = atlas_with(vec![doc("a/b/c/", "index"), child("one", None)]);

    let cards = h.view().atlas_cards;
    assert_eq!(cards.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(), vec!["a/b/c/index_atlas", "a/b/one"]);
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

#[test]
fn a_removed_card_loses_its_bounds_and_waits_unplaced_again() {
    let mut h = atlas_with(vec![child("one", Some("0,0,120,40"))]);
    h.send(Event::RemoveAtlasCard { page_id: "a/b/one".into() });

    assert_eq!(bounds_of(&h, "a/b/one"), None);
    let deleted: Vec<_> = h
        .api
        .calls_named("DeletePageProperty")
        .into_iter()
        .map(|op| match op {
            ApiOp::DeletePageProperty { page_id, key } => (page_id.clone(), key.clone()),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(deleted, vec![("a/b/one".to_string(), "atlas_bounds".to_string())]);
}

#[test]
fn a_refused_remove_puts_the_card_back_on_the_canvas() {
    let mut h = atlas_with(vec![child("one", Some("0,0,120,40"))]);
    h.api.fail_next("DeletePageProperty", "refused");
    h.send(Event::RemoveAtlasCard { page_id: "a/b/one".into() });

    assert_eq!(bounds_of(&h, "a/b/one"), Some(Bounds { x: 0.0, y: 0.0, width: 120.0, height: 40.0 }));
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
        route: crate::app::Route::Page { name: name.into(), fragment: None },
        today: "2031-01-05".into(),
    });
}

fn go_to(h: &mut Harness, path: &str) -> String {
    h.send(Event::GoToPage { path: path.into(), new_tab: false, new_window: false });
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
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::SaveAtlasDirs(dirs) if dirs == &vec!["a/b/".to_string()])));
    visit(&mut h, "a/b/index");
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::SaveAtlasDirs(dirs) if dirs.is_empty())));

    let mut h = Harness::new();
    h.send(Event::AtlasDirsRestored(vec!["a/b/".into()]));
    assert_eq!(go_to(&mut h, "a/b/index"), "/pages/a/b/index_atlas");
}

fn atlas_with_groups(groups: &[(&str, &[&str])]) -> Harness {
    let mut atlas = doc("a/b/", ATLAS_NAME);
    for (key, ids) in groups {
        atlas.properties.insert((*key).into(), serde_json::json!(ids));
    }
    showing(atlas)
}

fn deletes(h: &Harness) -> Vec<String> {
    h.api
        .calls_named("DeletePageProperty")
        .into_iter()
        .map(|op| match op {
            ApiOp::DeletePageProperty { key, .. } => key.clone(),
            _ => unreachable!(),
        })
        .collect()
}

fn group(h: &mut Harness, page_ids: &[&str], target_id: &str) {
    h.send(Event::GroupAtlasCards {
        page_ids: page_ids.iter().map(|id| id.to_string()).collect(),
        target_id: target_id.into(),
    });
}

#[test]
fn a_card_dropped_on_a_card_without_a_group_makes_a_new_group_of_the_two() {
    let mut h = atlas_with_groups(&[]);
    group(&mut h, &["a/b/one"], "a/b/two");

    let saved = upserts(&h);
    assert_eq!(saved.len(), 1, "{saved:?}");
    let (page_id, key, value) = &saved[0];
    assert_eq!(page_id, "a/b/index_atlas");
    assert!(key.starts_with("atlas_group_") && key.len() == "atlas_group_".len() + 8, "{key}");
    assert_eq!(value, "a/b/two,a/b/one");
}

#[test]
fn a_card_dropped_on_a_grouped_card_joins_its_group() {
    let mut h = atlas_with_groups(&[("atlas_group_g", &["a/b/two", "a/b/three"])]);
    group(&mut h, &["a/b/one"], "a/b/three");

    assert_eq!(
        upserts(&h),
        [("a/b/index_atlas".into(), "atlas_group_g".into(), "a/b/two,a/b/three,a/b/one".into())]
    );
}

#[test]
fn a_card_leaves_its_old_group_which_goes_when_one_card_is_left() {
    let mut h = atlas_with_groups(&[
        ("atlas_group_old", &["a/b/one", "a/b/x"]),
        ("atlas_group_big", &["a/b/one", "a/b/y", "a/b/z"]),
        ("atlas_group_new", &["a/b/two", "a/b/three"]),
    ]);
    // Never in two groups at once in practice; both are left here to see each case
    group(&mut h, &["a/b/one"], "a/b/two");

    assert_eq!(deletes(&h), ["atlas_group_old"]);
    let saved = upserts(&h);
    assert!(saved.contains(&("a/b/index_atlas".into(), "atlas_group_big".into(), "a/b/y,a/b/z".into())), "{saved:?}");
    assert!(
        saved.contains(&("a/b/index_atlas".into(), "atlas_group_new".into(), "a/b/two,a/b/three,a/b/one".into())),
        "{saved:?}"
    );
}

#[test]
fn a_card_dropped_on_itself_makes_no_group() {
    let mut h = atlas_with_groups(&[]);
    group(&mut h, &["a/b/one"], "a/b/one");

    assert!(upserts(&h).is_empty());
}

#[test]
fn a_refused_group_puts_the_groups_back() {
    let mut h = atlas_with_groups(&[("atlas_group_g", &["a/b/two", "a/b/three"])]);
    h.api.fail_next("UpsertPageProperty", "refused");
    group(&mut h, &["a/b/one"], "a/b/two");

    let atlas = crate::screens::page::page_state(&h.model).and_then(|ps| ps.page.clone()).unwrap();
    assert_eq!(atlas.properties["atlas_group_g"], serde_json::json!(["a/b/two", "a/b/three"]));
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
}

#[test]
fn a_card_knows_the_group_it_belongs_to() {
    let mut h = atlas_with_groups(&[("atlas_group_g", &["a/b/one", "a/b/two"])]);
    h.model.sidebar.index_pages = vec![child("one", None), child("two", None), child("three", None)];

    let groups: Vec<_> = h.view().atlas_cards.into_iter().map(|c| (c.page_id, c.group)).collect();
    assert_eq!(
        groups,
        [
            ("a/b/one".to_string(), Some("atlas_group_g".to_string())),
            ("a/b/three".to_string(), None),
            ("a/b/two".to_string(), Some("atlas_group_g".to_string())),
        ]
    );
}

fn file_changed(h: &mut Harness, page_id: &str, source: &str) {
    h.send(Event::Sse(crate::effects::SseEvent::Message(format!(
        r#"{{"type":"pagesUpdated","pageId":"{page_id}","source":"{source}"}}"#
    ))));
}

fn served_atlas(groups: &[(&str, &[&str])]) -> ApiResult<ApiOutput> {
    let mut atlas = doc("a/b/", ATLAS_NAME);
    for (key, ids) in groups {
        atlas.properties.insert((*key).into(), serde_json::json!(ids));
    }
    Ok(ApiOutput::Page(crate::model::PageWithItems {
        page: atlas,
        ..Default::default()
    }))
}

#[test]
fn the_shown_atlas_takes_the_groups_of_its_file_changed_on_disk() {
    let mut h = atlas_with_groups(&[("atlas_group_g", &["a/b/one", "a/b/two"])]);
    h.api.answer_next("GetPageById", served_atlas(&[("atlas_group_h", &["a/b/two", "a/b/three"])]));
    file_changed(&mut h, "a/b/index_atlas", "file");

    let atlas = crate::screens::page::page_state(&h.model).and_then(|ps| ps.page.clone()).unwrap();
    assert!(atlas.properties.get("atlas_group_g").is_none(), "{:?}", atlas.properties);
    assert_eq!(atlas.properties["atlas_group_h"], serde_json::json!(["a/b/two", "a/b/three"]));
}

#[test]
fn the_groups_are_not_fetched_for_a_change_made_in_the_app_or_on_another_page() {
    let mut h = atlas_with_groups(&[("atlas_group_g", &["a/b/one", "a/b/two"])]);
    file_changed(&mut h, "a/b/index_atlas", "app");
    file_changed(&mut h, "a/b/one", "file");

    assert!(!h.api.called("GetPageById"));
}

fn ungroup(h: &mut Harness, page_ids: &[&str]) {
    h.send(Event::UngroupAtlasCards {
        page_ids: page_ids.iter().map(|id| id.to_string()).collect(),
    });
}

#[test]
fn a_card_dragged_out_of_its_group_leaves_it() {
    let mut h = atlas_with_groups(&[("atlas_group_g", &["a/b/one", "a/b/two", "a/b/three"])]);
    ungroup(&mut h, &["a/b/two"]);

    assert_eq!(
        upserts(&h),
        [("a/b/index_atlas".to_string(), "atlas_group_g".to_string(), "a/b/one,a/b/three".to_string())]
    );
    assert!(deletes(&h).is_empty());
}

#[test]
fn a_group_left_with_one_card_goes() {
    let mut h = atlas_with_groups(&[("atlas_group_g", &["a/b/one", "a/b/two"])]);
    ungroup(&mut h, &["a/b/one"]);

    assert_eq!(deletes(&h), ["atlas_group_g"]);
    let atlas = crate::screens::page::page_state(&h.model).and_then(|ps| ps.page.clone()).unwrap();
    assert!(atlas.properties.get("atlas_group_g").is_none());
}

#[test]
fn a_card_in_no_group_leaves_nothing() {
    let mut h = atlas_with_groups(&[("atlas_group_g", &["a/b/one", "a/b/two"])]);
    ungroup(&mut h, &["a/b/three"]);

    assert!(upserts(&h).is_empty());
    assert!(deletes(&h).is_empty());
}

#[test]
fn a_refused_ungroup_puts_the_group_back() {
    let mut h = atlas_with_groups(&[("atlas_group_g", &["a/b/one", "a/b/two"])]);
    h.api.fail_next("DeletePageProperty", "refused");
    ungroup(&mut h, &["a/b/one"]);

    let atlas = crate::screens::page::page_state(&h.model).and_then(|ps| ps.page.clone()).unwrap();
    assert_eq!(atlas.properties["atlas_group_g"], serde_json::json!(["a/b/one", "a/b/two"]));
}

#[test]
fn the_hash_at_the_end_of_a_group_key_picks_one_of_the_six_colors() {
    use crate::screens::atlas::group_color;
    assert_eq!(group_color("atlas_group_00000000"), "#F9A2A2");
    assert_eq!(group_color("atlas_group_00000001"), "#B5B5FF");
    assert_eq!(group_color("atlas_group_0000000b"), "#C3C372");
    // Wraps around the six
    assert_eq!(group_color("atlas_group_00000006"), "#F9A2A2");
    // A key without a hash is hashed, and still gets a color of the six
    let six = ["#F9A2A2", "#B5B5FF", "#94D094", "#E1A6E1", "#5FD3D3", "#C3C372"];
    assert!(six.contains(&group_color("atlas_group_by-hand").as_str()));
}

#[test]
fn a_card_in_a_group_has_the_color_of_its_group() {
    let mut h = atlas_with_groups(&[("atlas_group_00000001", &["a/b/one", "a/b/two"])]);
    h.model.sidebar.index_pages = vec![child("one", None), child("three", None)];

    let colors: Vec<_> = h.view().atlas_cards.into_iter().map(|c| (c.page_id, c.group_color)).collect();
    assert_eq!(
        colors,
        [("a/b/one".to_string(), Some("#B5B5FF".to_string())), ("a/b/three".to_string(), None)]
    );
}

#[test]
fn a_card_draws_its_icon_from_the_tags_of_its_page() {
    use crate::screens::atlas::icon_tags;
    let mut page = doc("notes/", "post");
    assert!(icon_tags(&page, "tags/").is_empty());
    page.properties.insert("tags".into(), serde_json::json!(["cardbox", "card"]));
    assert_eq!(icon_tags(&page, "tags/"), ["cardbox", "card"]);
}

#[test]
fn a_tag_page_draws_its_icon_from_its_own_tag() {
    use crate::screens::atlas::icon_tags;
    assert_eq!(icon_tags(&doc("tags/", "CG"), "tags/"), ["CG"]);
    assert_eq!(icon_tags(&doc("tags/DailyReport/", "WPF"), "tags/"), ["DailyReport/WPF"]);
}

#[test]
fn a_folder_draws_its_icon_from_its_tag_its_tags_or_its_name() {
    use crate::screens::atlas::icon_tags;
    // A folder of tag pages is a tag too
    assert_eq!(icon_tags(&doc("tags/DailyReport/", "index"), "tags/"), ["DailyReport"]);
    assert_eq!(icon_tags(&doc("tags/a/b/", "index"), "tags/"), ["a/b"]);
    // Any other folder: the tags of its index page, or else its name
    assert_eq!(icon_tags(&doc("notes/rust/", "index"), "tags/"), ["rust"]);
    let mut tagged = doc("notes/rust/", "index");
    tagged.properties.insert("tags".into(), serde_json::json!(["lang"]));
    assert_eq!(icon_tags(&tagged, "tags/"), ["lang"]);
    // The folder of the tags itself is no tag
    assert_eq!(icon_tags(&doc("tags/", "index"), "tags/"), ["tags"]);
}

#[test]
fn an_icon_is_an_svg_of_the_size_asked() {
    use crate::screens::atlas::tag_icon_svg;
    let tags: Vec<String> = ["a", "b", "c", "d", "e"].iter().map(|t| t.to_string()).collect();
    let svg = tag_icon_svg(&tags, 24);
    assert!(svg.starts_with("<svg"), "{svg}");
    assert!(svg.contains(r#"width="24""#), "{svg}");
    // The same tags draw the same icon
    assert_eq!(svg, tag_icon_svg(&tags, 24));
}
