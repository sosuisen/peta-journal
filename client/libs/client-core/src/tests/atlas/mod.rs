//! The atlas of a directory: the map that shows its pages as cards.
//!
//! The helpers here build an atlas of `a/b/` for the specs in the submodules.

// アトラスを開く
mod navigation;

// カードを置く
mod cards;

// まとまりを作る
mod groups;

// 書き込む
mod callouts;
mod links;

// ほかで眺める
mod sync;

use crate::app::{Event, PageState, Screen};
use crate::effects::{ApiOp, ApiOutput, AtlasChange, UiOp};
use crate::harness::Harness;
use crate::model::PageExt;
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

/// The item of the card of the page `a/b/<name>`, `i-<name>`, placed by the bounds if any.
/// Its body links to the page, which is all that joins the two.
fn card_item(name: &str, level: i32, bounds: Option<&str>) -> Item {
    let props: Vec<(&str, &str)> = bounds.map(|b| ("atlas_bounds", b)).into_iter().collect();
    atlas_item(
        &format!("i-{name}"),
        level,
        &format!("[[a/b/{name}]]"),
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

fn group(h: &mut Harness, card_ids: &[&str], target_id: &str) {
    h.send(Event::GroupAtlasCards {
        card_ids: card_ids.iter().map(|id| id.to_string()).collect(),
        target_id: target_id.into(),
    });
}
