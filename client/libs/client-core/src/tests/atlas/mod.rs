//! The atlas of a directory: the map that shows its pages as cards.
//!
//! The helpers here build an atlas of `a/b/` for the specs in the submodules.

// アトラスを開く
mod navigation;

// 項目を描き分ける
mod kinds;

// カードを置く
mod cards;

// 置かれるのを待つ
mod waiting;

// まとまりを作る
mod groups;

// 動かして親子と並びを変える
mod moving;

// 書き込む
mod callouts;
mod links;

// ほかで眺める
mod sync;

use crate::app::{Event, PageState, Screen};
use crate::effects::{ApiOp, ApiOutput, UiOp};
use crate::harness::Harness;
use crate::model::PageExt;
use crate::model::{BatchOpInput, Item, ItemPosition, Page, PageWithItems};
use crate::screens::atlas::{AtlasGroup, Bounds, Callout, GroupTitle};
use crate::screens::page::{page_title, ATLAS_NAME};

/// The id of the item a card is right under
fn group_of(groups: &[AtlasGroup]) -> Option<String> {
    groups.last().map(|g| g.id.clone())
}

/// The title of the item a card is right under
fn title_of(groups: &[AtlasGroup]) -> Option<GroupTitle> {
    groups.last().and_then(|g| g.title.clone())
}

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

/// The id a batch is shown with for the item it makes, whose real id is new each time
const NEW: &str = "new";

/// The batches sent to change the outline, each one operation of the user. The id of the
/// item that a batch makes is written as [`NEW`].
fn batches(h: &Harness) -> Vec<Vec<BatchOpInput>> {
    crate::harness::batch_calls(&h.api)
        .into_iter()
        .map(|ops| {
            let made: Vec<String> = ops
                .iter()
                .filter_map(|op| match op {
                    BatchOpInput::Create { id, .. } => Some(id.clone()),
                    _ => None,
                })
                .collect();
            let named = |position: ItemPosition| match position {
                ItemPosition::After(id) if made.contains(&id) => ItemPosition::after(NEW),
                position => position,
            };
            ops.into_iter()
                .map(|op| match op {
                    BatchOpInput::Create {
                        page_id,
                        position,
                        level,
                        body,
                        transclude_item_id,
                        properties,
                        ..
                    } => BatchOpInput::Create {
                        id: NEW.into(),
                        page_id,
                        position: named(position),
                        level,
                        body,
                        transclude_item_id,
                        properties,
                    },
                    BatchOpInput::Move {
                        ids,
                        page_id,
                        position,
                    } => BatchOpInput::Move {
                        ids,
                        page_id,
                        position: named(position),
                    },
                    op => op,
                })
                .collect()
        })
        .collect()
}

/// The operation that makes an item of level 1 on the atlas page
fn made(position: ItemPosition, body: &str, props: &[(&str, &str)]) -> BatchOpInput {
    BatchOpInput::Create {
        id: NEW.into(),
        page_id: ATLAS_ID.into(),
        position,
        level: 1,
        body: body.into(),
        transclude_item_id: None,
        properties: props
            .iter()
            .map(|(key, value)| ((*key).into(), (*value).into()))
            .collect(),
    }
}

/// The operation that gives an item a level
fn leveled(id: &str, level: i32) -> BatchOpInput {
    BatchOpInput::Update {
        id: id.into(),
        body: None,
        level: Some(level),
        collapsed: None,
        task: None,
        properties: Default::default(),
        dropped_properties: Vec::new(),
    }
}

/// The operation that gives an item a body
fn written(id: &str, body: &str) -> BatchOpInput {
    BatchOpInput::Update {
        id: id.into(),
        body: Some(body.into()),
        level: None,
        collapsed: None,
        task: None,
        properties: Default::default(),
        dropped_properties: Vec::new(),
    }
}

/// The operation that moves items of the atlas page behind another
fn moved(ids: &[&str], after: &str) -> BatchOpInput {
    BatchOpInput::Move {
        ids: ids.iter().map(|id| id.to_string()).collect(),
        page_id: ATLAS_ID.into(),
        position: ItemPosition::after(after),
    }
}

fn deleted(id: &str) -> BatchOpInput {
    BatchOpInput::Delete { id: id.into() }
}

/// The outline the server answers when the atlas page is read again after a change
fn answer_outline(h: &mut Harness, items: Vec<Item>) {
    h.api.answer_next(
        "GetPageById",
        Ok(ApiOutput::Page(PageWithItems {
            page: doc("a/b/", ATLAS_NAME),
            items,
            ..Default::default()
        })),
    );
}

/// The operation that writes the side of an item, and its body when one is given
fn sided(id: &str, body: Option<&str>, side: &str) -> BatchOpInput {
    BatchOpInput::Update {
        id: id.into(),
        body: body.map(str::to_string),
        level: None,
        collapsed: None,
        task: None,
        properties: [("atlas_side".to_string(), side.into())].into(),
        dropped_properties: Vec::new(),
    }
}

/// The operation that takes the side away from an item that is no parent any more
fn unsided(id: &str) -> BatchOpInput {
    BatchOpInput::Update {
        id: id.into(),
        body: None,
        level: None,
        collapsed: None,
        task: None,
        properties: Default::default(),
        dropped_properties: vec!["atlas_side".into()],
    }
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
