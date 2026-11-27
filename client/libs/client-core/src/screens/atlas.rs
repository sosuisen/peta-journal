//! The canvas of an atlas page: the pages and folders of its directory as cards. The atlas
//! page is a plain page (ADR 053): the server writes nothing into it by itself, and its
//! outline may be edited here, in the outline or in the `.md`.
//! A card is an item with something written in it and no item under it. One whose first line
//! is one wiki link stands for the page of the directory that the link names, or else it is a
//! link card; any other is a plain card, which shows its first line. A card lies where the `atlas_bounds`
//! property of its item says, in coordinates whose origin is the center of the canvas. A card
//! or a callout without the property waits beside the canvas among the items without a place.
//! A page of the directory that no card links to waits there too, among the pages not added:
//! dropped on the canvas, it gets an item that links to it.
//! A group is an item with items under it, at any depth, and a card is in every group above
//! it; a callout is an item whose `atlas_type` is `callout`. A group that is a page card or a
//! callout is drawn as one along its outline.
//! The canvas edits the page with the item APIs that the outline uses, and no other. The
//! place of a card or a callout is saved as a property of its item. A change of the outline
//! is one batch of item operations, however many items it writes, so one operation of the
//! user is one change on the server. The outline is then read again and shown.
//! A change that leaves an item with no item under it takes the `atlas_side` of that item
//! away in the same batch: the side is of the title of a group, and the item is no group any
//! more. Nothing is cleaned up for a change made in the outline or in the `.md`.

use crate::app::{cancel_timer, done, log_error, run, Cmd, Event, Model};
use crate::effects::{Api, ApiResult, TimerOutput, UiOp};
use crate::model::{BatchOpInput, Item, ItemPosition, Page, PropertyValue};
use crate::model::{ItemExt, PageExt};
use crate::ops::file_reload;
use crate::rich_text::{RichBlock, RichRun, RichText};
use crate::screens::page::{split_doc_path, ATLAS_NAME};
use crate::screens::sidebar;
use crate::view::UnplacedCard;
use crate::BodyModel;
use crux_core::Command;
use peta_atlas::plain::{self, Kind};

pub use peta_atlas::marks::{
    callout_face_svg, group_color, link_blob_svg, tag_blob_svg, tag_icon_svg,
};
pub use peta_atlas::{
    card_label as link_label, link_target, Bounds, AUTO_WIDTH, BOUNDS_KEY, CARD_HEIGHT,
    MIN_CARD_SCALE, SIDES as TITLE_SIDES, SIDE_KEY, TYPE_KEY,
};
use peta_atlas::{link_body, side_or_default, CALLOUT_TYPE};

/// The width of a card in the lists beside the canvas, which a drop from a list reads.
pub const CARD_WIDTH: f64 = 120.0;

type Entry<'a> = plain::Entry<'a, Item>;

/// The items of the shown atlas page that the canvas can show, in their order; none on any
/// other page. An item belongs to every item above it.
fn entries(model: &Model) -> Vec<Entry<'_>> {
    let Some(atlas) = shown_atlas_page(model) else {
        return Vec::new();
    };
    plain::entries(
        model
            .state
            .items()
            .iter()
            .filter(|item| item.page_id == atlas.id)
            .map(|item| &**item),
    )
}

/// Where a card or a callout lies, read from its item.
fn bounds_of(item: &Item) -> Option<Bounds> {
    item.prop_str(BOUNDS_KEY).and_then(Bounds::parse)
}

/// A card of the shown atlas page.
struct Card<'a> {
    item: &'a Item,
    /// The group item the card is under: the last of its parents
    group: Option<&'a Item>,
    /// The items the card is under, the outermost first
    parents: Vec<&'a Item>,
    /// The page of the directory the card stands for; none for a link card
    page_id: Option<&'a str>,
}

impl Card<'_> {
    /// The name the shell calls the card by: the id of its page, or of the item of a link card.
    fn id(&self) -> &str {
        self.page_id.unwrap_or(&self.item.id)
    }
}

/// The pages of the directory of the shown atlas: the index page of the directory itself,
/// once it is known, and the pages of the list of the directory.
fn dir_pages(model: &Model) -> impl Iterator<Item = &Page> {
    let sidebar = &model.sidebar;
    sidebar.index_page.iter().chain(&sidebar.index_pages)
}

/// The page of the directory of the shown atlas that a link names by its path.
fn page_of<'a>(model: &'a Model, target: &str) -> Option<&'a Page> {
    let target = target.strip_suffix(".md").unwrap_or(target);
    dir_pages(model).find(|p| format!("{}{}", p.dir_path, p.name) == target)
}

/// Whether a page is the index page of the directory of the shown atlas itself.
fn is_own_index(model: &Model, page: &Page) -> bool {
    let index = model.sidebar.index_page.as_ref();
    index.is_some_and(|index| index.id == page.id)
}

/// The cards of the shown atlas page, in the order of its items. The first card that links
/// to a page of the directory stands for that page. Any other card is a link card, one that
/// links out of the directory, to no page, or to a page that has a card already, or a plain
/// card.
fn card_entries(model: &Model) -> Vec<Card<'_>> {
    let mut taken: Vec<&str> = Vec::new();
    entries(model)
        .into_iter()
        .filter(|e| e.kind != Kind::Callout && !e.is_parent)
        .map(|e| {
            let page_id = plain::card_link(&e.item.body)
                .and_then(|link| page_of(model, link.target))
                .map(|page| page.id.as_str())
                .filter(|id| !taken.contains(id));
            taken.extend(page_id);
            Card {
                item: e.item,
                group: e.group(),
                parents: e.parents,
                page_id,
            }
        })
        .collect()
}

/// The item of a card: of a page card by the id of its page, of a link card by its own id.
fn card_item(model: &Model, card_id: &str) -> Option<Item> {
    card_entries(model)
        .into_iter()
        .find(|card| card.id() == card_id)
        .map(|card| card.item.clone())
}

/// The item of a callout that is no parent, by its id.
fn callout_leaf(model: &Model, item_id: &str) -> Option<Item> {
    entries(model)
        .into_iter()
        .find(|e| e.kind == Kind::Callout && !e.is_parent && e.item.id == item_id)
        .map(|e| e.item.clone())
}

/// Places a card with its center at the given point of the canvas. A card is named by its
/// page, or a link card by its item. The shell knows where the card was grabbed, so it gives
/// the center rather than the drop point. The card moves at once; a refusal of the server
/// puts it back. A page of the directory that has no card yet gets one there. A callout that
/// waits for a place is named by its item, and is placed as a card is.
pub fn place_card(model: &mut Model, card_id: String, x: f64, y: f64) -> Cmd {
    if !x.is_finite() || !y.is_finite() {
        return done();
    }
    let item = card_item(model, &card_id).or_else(|| callout_leaf(model, &card_id));
    let Some(item) = item else {
        return add_page_card(model, &card_id, x, y);
    };
    // A card keeps its size when it moves; whole pixels keep the stored value short
    let (width, height) = bounds_of(&item)
        .map(|b| (b.width, b.height))
        .unwrap_or((AUTO_WIDTH, CARD_HEIGHT));
    let bounds = Bounds {
        x: x.round(),
        y: y.round(),
        width,
        height,
    };
    save_bounds(model, &item, bounds)
}

/// Gives a page of the directory that no card links to a card with its center at the point:
/// an item that links to the page. The card shows once the server has made the item.
fn add_page_card(model: &Model, page_id: &str, x: f64, y: f64) -> Cmd {
    let Some(page) = dir_pages(model).find(|p| p.id == page_id) else {
        return done();
    };
    // The path of a folder is the path of its index page, as the server once wrote it
    let path = format!("{}{}", page.dir_path, page.name);
    add_card(model, &path, x, y)
}

/// Makes an item that links to `target`, placed with its center at the point, at the end of
/// the outline.
fn add_card(model: &Model, target: &str, x: f64, y: f64) -> Cmd {
    let bounds = Bounds {
        x: x.round(),
        y: y.round(),
        width: AUTO_WIDTH,
        height: CARD_HEIGHT,
    };
    let body = link_body(target);
    change_with_new_item(model, move |atlas_id, id| {
        vec![new_item(
            atlas_id,
            id,
            ItemPosition::Last,
            body,
            &[(BOUNDS_KEY, bounds.to_string())],
        )]
    })
}

/// Places what was dropped from the lists beside the canvas: a page not added, or an item
/// without a place. It is placed by its center, which lies this far from the grabbed spot of
/// a card of the list.
pub fn drop_card(
    model: &mut Model,
    card_id: String,
    x: f64,
    y: f64,
    grab_x: f64,
    grab_y: f64,
) -> Cmd {
    let center_x = x + (0.5 - grab_x) * CARD_WIDTH;
    let center_y = y + (0.5 - grab_y) * CARD_HEIGHT;
    place_card(model, card_id, center_x, center_y)
}

/// Scales a card, no smaller than [`MIN_CARD_SCALE`]. The card keeps its place. Its height is
/// stored in whole pixels, so the scale moves in small steps. The width goes back to the
/// label: a set width would not fit the label at the new scale.
pub fn scale_card(model: &mut Model, card_id: String, scale: f64) -> Cmd {
    let Some(item) = card_item(model, &card_id) else {
        return done();
    };
    let Some(before) = bounds_of(&item).filter(|_| scale.is_finite()) else {
        return done();
    };
    let bounds = Bounds {
        width: AUTO_WIDTH,
        height: (CARD_HEIGHT * scale.max(MIN_CARD_SCALE)).round(),
        ..before
    };
    save_bounds(model, &item, bounds)
}

/// Gives an item new bounds at once and saves them, unless they stay the same. A refusal of
/// the server puts the old ones back.
fn save_bounds(model: &mut Model, item: &Item, bounds: Bounds) -> Cmd {
    if bounds_of(item) == Some(bounds) {
        return done();
    }
    let item_id = item.id.clone();
    let before = item.properties.get(BOUNDS_KEY).cloned();
    let value = bounds.to_string();
    set_property(model, &item_id, Some(PropertyValue::Text(value.clone())));
    run(move |api| async move {
        if let Err(e) = api
            .upsert_item_property(&item_id, BOUNDS_KEY, "bounds", &value)
            .await
        {
            api.alert(e.0);
            api.send_event(Event::AtlasBoundsRefused { item_id, before });
        }
    })
}

/// The server refused the new bounds of an item: the ones it had before are shown again.
pub fn put_bounds_back(model: &mut Model, item_id: &str, before: Option<PropertyValue>) -> Cmd {
    set_property(model, item_id, before);
    done()
}

fn set_property(model: &mut Model, item_id: &str, value: Option<PropertyValue>) {
    model.state.update_item(item_id, |item| match value {
        Some(value) => {
            item.properties.insert(BOUNDS_KEY.to_string(), value);
        }
        None => {
            item.properties.remove(BOUNDS_KEY);
        }
    });
}

/// Takes the card of a page off the canvas: its bounds are removed, so it waits among the
/// items without a place again. The card leaves at once; a refusal of the server puts it back.
pub fn remove_card(model: &mut Model, page_id: String) -> Cmd {
    let Some(item) = card_item(model, &page_id).filter(|i| bounds_of(i).is_some()) else {
        return done();
    };
    let item_id = item.id.clone();
    let before = item.properties.get(BOUNDS_KEY).cloned();
    set_property(model, &item_id, None);
    run(move |api| async move {
        if let Err(e) = api.delete_item_property(&item_id, BOUNDS_KEY).await {
            api.alert(e.0);
            api.send_event(Event::AtlasBoundsRefused { item_id, before });
        }
    })
}

/// Sends a change of the outline of the shown atlas page as one batch and shows the outline
/// as the server has it then. A change that writes nothing sends nothing.
fn change(model: &Model, ops: Vec<BatchOpInput>) -> Cmd {
    let Some(atlas_id) = shown_atlas_page(model).map(|p| p.id.clone()) else {
        return done();
    };
    if ops.is_empty() {
        return done();
    }
    run(move |api| async move { send(&api, atlas_id, ops).await })
}

/// Sends a change of the outline that makes an item. `make` takes the id of the atlas page
/// and the id of the new item. The id is made here, so the operations that follow in the same
/// batch can name the item.
fn change_with_new_item(
    model: &Model,
    make: impl FnOnce(String, String) -> Vec<BatchOpInput> + Send + 'static,
) -> Cmd {
    let Some(atlas_id) = shown_atlas_page(model).map(|p| p.id.clone()) else {
        return done();
    };
    run(move |api| async move {
        let (_, now_ms) = api.local_now().await;
        // A generator of its own: the one of the model cannot be reached from here, and an
        // id is unique by its random part
        let id = hmtid::Generator::for_pages_and_items().generate_at(now_ms);
        let ops = make(atlas_id.clone(), id.as_str().to_string());
        send(&api, atlas_id, ops).await
    })
}

/// Sends the batch, reads the outline again and shows it. A card keeps the place shown for
/// it: a card placed and grouped at once may be read before its place is saved.
async fn send(api: &Api<Event>, atlas_id: String, ops: Vec<BatchOpInput>) {
    match written(api, &atlas_id, ops).await {
        Ok(items) => api.send_event(Event::AtlasChanged { atlas_id, items }),
        Err(e) => api.alert(e.0),
    }
}

/// Writes the batch and answers with the items of the atlas page as they are then.
async fn written(api: &Api<Event>, atlas_id: &str, ops: Vec<BatchOpInput>) -> ApiResult<Vec<Item>> {
    api.apply_batch(ops).await?;
    Ok(api.get_page_by_id(atlas_id).await?.items)
}

/// The operation that makes an item of level 1 on the atlas page.
fn new_item(
    atlas_id: String,
    id: String,
    position: ItemPosition,
    body: String,
    properties: &[(&str, String)],
) -> BatchOpInput {
    BatchOpInput::Create {
        id,
        page_id: atlas_id,
        position,
        level: 1,
        body,
        transclude_item_id: None,
        properties: properties
            .iter()
            .map(|(key, value)| (key.to_string(), PropertyValue::Text(value.clone())))
            .collect(),
    }
}

/// The operation that writes the body and the side of an item, each when it is given; none
/// when neither is.
fn rewritten(item_id: &str, body: Option<String>, side: Option<String>) -> Option<BatchOpInput> {
    (body.is_some() || side.is_some()).then(|| BatchOpInput::Update {
        id: item_id.to_string(),
        body,
        level: None,
        collapsed: None,
        task: None,
        properties: side
            .map(|side| (SIDE_KEY.to_string(), PropertyValue::Text(side)))
            .into_iter()
            .collect(),
        dropped_properties: Vec::new(),
    })
}

/// The item an item is under: the nearest item above it with a lower level.
fn parent_of<'a>(model: &'a Model, item: &Item) -> Option<&'a Item> {
    let items = model.state.items();
    let at = items.iter().position(|i| i.id == item.id)?;
    items[..at]
        .iter()
        .rev()
        .take_while(|above| above.page_id == item.page_id)
        .find(|above| above.level < item.level)
        .map(|above| &**above)
}

/// The operation that takes the side away from `parent` when no item stays under it while
/// the items `leaving` go: it is no parent then, and has no title to lay along a side. None
/// for an item that keeps an item under it, for one without a side, and for a callout, whose
/// side is of its own bubble.
fn side_dropped(model: &Model, parent: &Item, leaving: &[&Item]) -> Option<BatchOpInput> {
    let emptied = last_staying(model, parent, leaving).id == parent.id;
    let has_side = parent.prop_str(SIDE_KEY).is_some();
    let is_callout = parent.prop_str(TYPE_KEY) == Some(CALLOUT_TYPE);
    (emptied && has_side && !is_callout).then(|| BatchOpInput::Update {
        id: parent.id.clone(),
        body: None,
        level: None,
        collapsed: None,
        task: None,
        properties: Default::default(),
        dropped_properties: vec![SIDE_KEY.to_string()],
    })
}

/// Deletes an item of the shown atlas page. The item it was under loses its side in the same
/// batch when nothing else is under it.
fn delete(model: &Model, item_id: String) -> Cmd {
    let Some(item) = model.state.get_item_by_id(&item_id) else {
        return done();
    };
    let side = parent_of(model, &item).and_then(|parent| side_dropped(model, parent, &[&*item]));
    let ops = std::iter::once(BatchOpInput::Delete { id: item_id })
        .chain(side)
        .collect();
    change(model, ops)
}

/// The operations that take items behind the item `after` at a level, in their order. The
/// level is written first: the server checks the levels of the items it moves.
fn taken_behind(atlas_id: &str, items: &[&Item], after: &str, level: i32) -> Vec<BatchOpInput> {
    let mut ops: Vec<BatchOpInput> = items
        .iter()
        .map(|item| BatchOpInput::Update {
            id: item.id.clone(),
            body: None,
            level: Some(level),
            collapsed: None,
            task: None,
            properties: Default::default(),
            dropped_properties: Vec::new(),
        })
        .collect();
    ops.push(BatchOpInput::Move {
        ids: items.iter().map(|item| item.id.clone()).collect(),
        page_id: atlas_id.to_string(),
        position: ItemPosition::after(after),
    });
    ops
}

/// The last item of the tree of `parent` that stays where it is while the items `leaving`
/// move: the parent itself when every item under it leaves.
fn last_staying<'a>(model: &'a Model, parent: &'a Item, leaving: &[&Item]) -> &'a Item {
    let items = model.state.items();
    let start = items.iter().position(|item| item.id == parent.id);
    let under = items
        .iter()
        .skip(start.map_or(items.len(), |at| at + 1))
        .take_while(|item| item.page_id == parent.page_id && item.level > parent.level);
    under
        .filter(|item| !leaving.iter().any(|l| l.id == item.id))
        .last()
        .map_or(parent, |item| &**item)
}

/// The server changed the outline of an atlas page: its items are shown. A card keeps the
/// place shown for it.
pub fn show_changed(model: &mut Model, atlas_id: &str, mut items: Vec<Item>) -> Cmd {
    for item in &mut items {
        let shown = model.state.get_item_by_id(&item.id);
        if let Some(bounds) = shown.and_then(|i| i.properties.get(BOUNDS_KEY).cloned()) {
            item.properties.insert(BOUNDS_KEY.to_string(), bounds);
        }
    }
    file_reload::replace_page_items(model, atlas_id, items);
    done()
}

/// Cards dropped on another card join its group: they go to the end of the group, under its
/// item. When the target is in no group, a new item with an empty body is made where the
/// target lies, and the target and the cards go under it. A card belongs to one group only,
/// so the cards leave the groups they were in. The item of a group they leave stays, even
/// with one card or none under it (ADR 053).
pub fn group_cards(model: &mut Model, card_ids: Vec<String>, target_id: String) -> Cmd {
    let cards = card_entries(model);
    let Some(target) = cards.iter().find(|card| card.id() == target_id) else {
        return done();
    };
    let in_group_of_target = |card: &Card| {
        target.group.is_some() && card.group.map(|g| &g.id) == target.group.map(|g| &g.id)
    };
    let joining: Vec<&Card> = cards
        .iter()
        .filter(|card| card.id() != target_id && card_ids.iter().any(|id| id == card.id()))
        .filter(|card| !in_group_of_target(card))
        .collect();
    if joining.is_empty() {
        return done();
    }
    let left = sides_dropped(model, &joining);
    let joining: Vec<&Item> = joining.iter().map(|card| card.item).collect();
    match target.group {
        Some(group) => {
            let last = last_staying(model, group, &joining);
            let mut ops = taken_behind(&group.page_id, &joining, &last.id, group.level + 1);
            ops.extend(left);
            change(model, ops)
        }
        None => {
            let target = target.item.clone();
            let joining: Vec<Item> = joining.into_iter().cloned().collect();
            change_with_new_item(model, move |atlas_id, group_id| {
                let members: Vec<&Item> = std::iter::once(&target).chain(&joining).collect();
                let mut ops = vec![new_item(
                    atlas_id.clone(),
                    group_id.clone(),
                    ItemPosition::after(&target.id),
                    String::new(),
                    &[],
                )];
                ops.extend(taken_behind(&atlas_id, &members, &group_id, 2));
                ops.extend(left);
                ops
            })
        }
    }
}

/// Cards let go on the blob of a parent go under its item.
pub fn move_cards(model: &mut Model, card_ids: Vec<String>, parent_id: String) -> Cmd {
    let cards = card_entries(model);
    let Some(parent) = model.state.get_item_by_id(&parent_id) else {
        return done();
    };
    let joining: Vec<&Item> = cards
        .iter()
        .filter(|card| card_ids.iter().any(|id| id == card.id()))
        .map(|card| card.item)
        .collect();
    let last = last_staying(model, &parent, &joining);
    let ops = taken_behind(&parent.page_id, &joining, &last.id, parent.level + 1);
    change(model, ops)
}

/// The groups the cards are in, each once, in the order of the cards.
fn groups_of<'a>(cards: &[&Card<'a>]) -> Vec<&'a Item> {
    let mut groups: Vec<&Item> = Vec::new();
    for group in cards.iter().filter_map(|card| card.group) {
        if !groups.iter().any(|g| g.id == group.id) {
            groups.push(group);
        }
    }
    groups
}

/// The operations that take the side away from each group that the cards leave with no item
/// under it.
fn sides_dropped(model: &Model, leaving: &[&Card]) -> Vec<BatchOpInput> {
    let items: Vec<&Item> = leaving.iter().map(|card| card.item).collect();
    groups_of(leaving)
        .into_iter()
        .filter_map(|group| side_dropped(model, group, &items))
        .collect()
}

/// Cards taken out of their groups, as when they are dragged out of the outline of their
/// group with Ctrl held. A card goes behind the tree of its group, at the level of the group
/// item. The item of the group stays, even with one card or none under it (ADR 053). With
/// none, it loses its side.
pub fn ungroup_cards(model: &mut Model, card_ids: Vec<String>) -> Cmd {
    let cards = card_entries(model);
    let leaving: Vec<&Card> = cards
        .iter()
        .filter(|card| card.group.is_some() && card_ids.iter().any(|id| id == card.id()))
        .collect();
    let all: Vec<&Item> = leaving.iter().map(|card| card.item).collect();
    let ops: Vec<BatchOpInput> = groups_of(&leaving)
        .into_iter()
        .flat_map(|group| {
            let of_group: Vec<&Item> = leaving
                .iter()
                .filter(|card| card.group.is_some_and(|g| g.id == group.id))
                .map(|card| card.item)
                .collect();
            let last = last_staying(model, group, &all);
            let mut ops = taken_behind(&group.page_id, &of_group, &last.id, group.level);
            ops.extend(side_dropped(model, group, &all));
            ops
        })
        .collect();
    change(model, ops)
}

/// The title of a group and the side of its outline it lies along.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GroupTitle {
    pub side: String,
    pub text: String,
}

fn side_of(item: &Item) -> String {
    peta_atlas::side_of(item).to_string()
}

/// The title of a group item; none while its body is empty.
fn group_title(group: &Item) -> Option<GroupTitle> {
    plain::group_title(group).map(|text| GroupTitle {
        side: side_of(group),
        text,
    })
}

/// An item that cards are under: a blob around them on the canvas, with its title along it.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AtlasGroup {
    /// The id of the item
    pub id: String,
    pub title: Option<GroupTitle>,
    pub look: GroupLook,
}

/// How the title of a group is drawn along its blob, by what its item is.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum GroupLook {
    /// The text of the item
    Text,
    /// The label of a page card, which opens its page, with the icon of the page. A link to a
    /// page that is not there opens nothing.
    Page {
        path: Option<String>,
        tone: CardTone,
        icon_tags: Vec<String>,
    },
    /// The character of a callout, with the text in its bubble
    Callout,
}

/// The groups of the items a card is under, the outermost first.
fn groups(model: &Model, parents: &[&Item]) -> Vec<AtlasGroup> {
    parents
        .iter()
        .map(|parent| AtlasGroup {
            id: parent.id.clone(),
            title: group_title(parent),
            look: group_look(model, parent),
        })
        .collect()
}

fn group_look(model: &Model, parent: &Item) -> GroupLook {
    if parent.prop_str(TYPE_KEY) == Some(CALLOUT_TYPE) {
        return GroupLook::Callout;
    }
    let Some(link) = plain::card_link(&parent.body) else {
        return GroupLook::Text;
    };
    GroupLook::Page {
        path: link_path(model, link),
        tone: tone_of(model, Some(link)),
        icon_tags: page_of(model, link.target)
            .map(|page| icon_tags(page, &model.config.tags_dir))
            .unwrap_or_default(),
    }
}

/// The path a click on a link of a card opens: of a page of the directory as its card opens
/// it, of any other page as the link names it. None for a page that is not there.
fn link_path(model: &Model, link: plain::CardLink) -> Option<String> {
    match page_of(model, link.target) {
        Some(page) => Some(opened_path(model, page)),
        None => (tone_of(model, Some(link)) != CardTone::Missing).then(|| link.target.to_string()),
    }
}

/// The path a click on the card of a page of the directory opens. A folder opens as its atlas,
/// so the map stays a map on the way down. The index page of the directory itself opens as
/// the index page: its atlas is the map that is open.
pub fn opened_path(model: &Model, page: &Page) -> String {
    if sidebar::is_directory(page) && !is_own_index(model, page) {
        format!("{}{}", page.dir_path, ATLAS_NAME)
    } else {
        page.canonical_path()
    }
}

/// Gives a group of the shown atlas page a title along a side of its outline. An empty title
/// takes the title away. The title is the body of the item of the group, and the side a
/// property of it.
pub fn set_group_title(model: &mut Model, group_id: String, side: String, title: String) -> Cmd {
    let Some(group) = entries(model)
        .into_iter()
        .find(|e| e.is_parent && e.item.id == group_id)
        .map(|e| e.item.clone())
    else {
        return done();
    };
    let side = side_or_default(&side).to_string();
    let title = title.trim().to_string();
    // The title shown, and not the body: a body that is a link shows its label as the title
    let shown = plain::group_title(&group).unwrap_or_default();
    let body = (shown != title).then_some(title);
    let side = (side_of(&group) != side).then_some(side);
    change(
        model,
        rewritten(&group_id, body, side).into_iter().collect(),
    )
}

/// A callout on the canvas of an atlas page: a character at a point, with a speech bubble on
/// one side of it. An item whose `atlas_type` is `callout`, its text the body.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Callout {
    /// The id of the item of the callout
    pub key: String,
    /// The center of the character, in the coordinates of the canvas
    pub x: f64,
    pub y: f64,
    /// How large the whole callout is drawn, no smaller than `MIN_CARD_SCALE` as a card
    pub scale: f64,
    /// The side of the character the bubble is on, one of `TITLE_SIDES`
    pub side: String,
    pub text: String,
}

/// The callouts of the shown atlas page, in their order. A callout without a place is left out.
pub fn shown_callouts(model: &Model) -> Vec<Callout> {
    entries(model)
        .into_iter()
        .filter(|e| e.kind == Kind::Callout && !e.is_parent)
        .filter_map(|e| {
            let bounds = bounds_of(e.item).filter(|b| b.height > 0.0)?;
            Some(Callout {
                key: e.item.id.clone(),
                x: bounds.x,
                y: bounds.y,
                scale: bounds.scale().max(MIN_CARD_SCALE),
                side: side_of(e.item),
                text: e.item.body.clone(),
            })
        })
        .collect()
}

/// The bounds of a callout: the place in whole units and the height of its scale to a
/// hundredth, so a small move of the pointer does not show as a change.
fn callout_bounds(x: f64, y: f64, scale: f64) -> Bounds {
    let scale = (scale.max(MIN_CARD_SCALE) * 100.0).round() / 100.0;
    Bounds {
        x: x.round(),
        y: y.round(),
        width: AUTO_WIDTH,
        height: CARD_HEIGHT * scale,
    }
}

fn callout_item(model: &Model, key: &str) -> Option<(Item, Callout)> {
    let callout = shown_callouts(model).into_iter().find(|c| c.key == key)?;
    let item = model.state.get_item_by_id(key)?;
    Some(((*item).clone(), callout))
}

/// Moves a callout of the shown atlas page so the center of its character is at the point.
pub fn move_callout(model: &mut Model, key: String, x: f64, y: f64) -> Cmd {
    let Some((item, callout)) = callout_item(model, &key) else {
        return done();
    };
    if !x.is_finite() || !y.is_finite() {
        return done();
    }
    save_bounds(model, &item, callout_bounds(x, y, callout.scale))
}

/// Scales a callout of the shown atlas page around the center of its character, no smaller
/// than a card can be.
pub fn scale_callout(model: &mut Model, key: String, scale: f64) -> Cmd {
    let Some((item, callout)) = callout_item(model, &key) else {
        return done();
    };
    if !scale.is_finite() {
        return done();
    }
    save_bounds(model, &item, callout_bounds(callout.x, callout.y, scale))
}

/// Puts a new callout on the shown atlas page with the center of its character at the
/// point: an item of level 1 at the end of the outline. An empty text puts nothing.
pub fn add_callout(model: &mut Model, x: f64, y: f64, side: String, text: String) -> Cmd {
    let text = text.trim().to_string();
    if text.is_empty() || !x.is_finite() || !y.is_finite() {
        return done();
    }
    let bounds = callout_bounds(x, y, 1.0).to_string();
    let side = side_or_default(&side).to_string();
    change_with_new_item(model, move |atlas_id, id| {
        vec![new_item(
            atlas_id,
            id,
            ItemPosition::Last,
            text,
            &[
                (TYPE_KEY, CALLOUT_TYPE.to_string()),
                (BOUNDS_KEY, bounds),
                (SIDE_KEY, side),
            ],
        )]
    })
}

/// Changes the text and the side of a callout of the shown atlas page. It keeps its place and
/// its scale. An empty text takes the callout away: its item is deleted.
pub fn set_callout(model: &mut Model, key: String, side: String, text: String) -> Cmd {
    let Some((_, callout)) = callout_item(model, &key) else {
        return done();
    };
    let text = text.trim().to_string();
    if text.is_empty() {
        return delete(model, key);
    }
    let side = side_or_default(&side).to_string();
    let body = (callout.text != text).then_some(text);
    let side = (callout.side != side).then_some(side);
    change(model, rewritten(&key, body, side).into_iter().collect())
}

/// The tags the icon of a card is drawn from. A tag page stands for its tag, and so does a
/// folder of tag pages, so its card takes the tag itself. Any other page takes its own tags.
/// A folder, whose card is its index page, takes the tags of that page, or else its name, so
/// every folder has an icon.
pub fn icon_tags(page: &Page, tags_dir: &str) -> Vec<String> {
    let is_dir = sidebar::is_directory(page);
    let path = if is_dir {
        page.dir_path.trim_end_matches('/').to_string()
    } else {
        format!("{}{}", page.dir_path, page.name)
    };
    if let Some(tag) = path.strip_prefix(tags_dir).filter(|tag| !tag.is_empty()) {
        return vec![tag.to_string()];
    }
    let tags: Vec<String> = page
        .properties
        .get("tags")
        .and_then(PropertyValue::as_list)
        .map(<[String]>::to_vec)
        .unwrap_or_default();
    match path.rsplit('/').next() {
        Some(name) if is_dir && tags.is_empty() && !name.is_empty() => vec![name.to_string()],
        _ => tags,
    }
}

/// Takes the items of the shown atlas page from the server again, after a file or another
/// client changed them. The reload is needed even while no items are shown.
pub fn reload_atlas(model: &Model, page_id: &str) -> Cmd {
    if shown_atlas_page(model).is_none_or(|p| p.id != page_id) {
        return done();
    }
    file_reload::read_again(model, page_id)
}

/// How a card is colored, by what it stands for.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum CardTone {
    /// A link to a page of the directory of the atlas: blue
    Page,
    /// A link to a page of another directory: orange
    Link,
    /// A link to a page that is not there: gray. A click opens nothing.
    Missing,
    /// A card whose first line is no link: yellow
    Plain,
}

/// A card of the atlas page that stands for no page of the directory, placed by its own
/// `atlas_bounds`: a link card, whose first line is a wiki link to `target`, or a plain card,
/// whose first line is anything else.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LinkCard {
    pub item_id: String,
    /// The page the link names, as it is written in the link; none for a plain card
    pub target: Option<String>,
    /// What the card shows: the label of the link, or else the name of the page it names. A
    /// plain card shows the text of its first line.
    pub label: String,
    pub tone: CardTone,
    /// The letters of the first line with their looks and their links, which a click follows
    pub runs: Vec<RichRun>,
    /// The whole body of the item, which the menu of the card shows
    pub text: String,
    pub bounds: Bounds,
    /// The items the card is under, the outermost first
    pub groups: Vec<AtlasGroup>,
}

/// The link cards of the shown atlas page, in the order of its items. A link card without a
/// place is left out.
pub fn link_cards(model: &Model) -> Vec<LinkCard> {
    card_entries(model)
        .into_iter()
        .filter(|card| card.page_id.is_none())
        .filter_map(|card| {
            let link = plain::card_link(&card.item.body);
            let runs = first_line_runs(&card.item.body, &model.config.tags_dir);
            let label = match link {
                Some(link) => link.shown(),
                None => runs.iter().map(|run| run.text.as_str()).collect(),
            };
            Some(LinkCard {
                item_id: card.item.id.clone(),
                target: link.map(|link| link.target.to_string()),
                label,
                runs,
                text: card.item.body.clone(),
                tone: tone_of(model, link),
                bounds: bounds_of(card.item)?,
                groups: groups(model, &card.parents),
            })
        })
        .collect()
}

/// How a card that stands for no page of the directory is colored: blue for another card to
/// a page of the directory, gray for one to a page the directory does not have, orange for a
/// link out of it, yellow for a plain card. The directory is known to lack a page only once its
/// list is in.
fn tone_of(model: &Model, link: Option<plain::CardLink>) -> CardTone {
    match link {
        Some(link) if page_of(model, link.target).is_some() => CardTone::Page,
        Some(link) if in_atlas_dir(model, link.target) && index_loaded(model) => CardTone::Missing,
        Some(_) => CardTone::Link,
        None => CardTone::Plain,
    }
}

/// Whether a path names a page of the directory of the shown atlas, or the index of a folder
/// in it: the pages the list of the directory holds.
fn in_atlas_dir(model: &Model, target: &str) -> bool {
    let Some(rest) =
        shown_atlas_page(model).and_then(|atlas| target.strip_prefix(&*atlas.dir_path))
    else {
        return false;
    };
    match rest.split_once('/') {
        None => true,
        Some((folder, name)) => !folder.is_empty() && name == "index",
    }
}

fn index_loaded(model: &Model) -> bool {
    model.sidebar.index_state == crate::view::LoadState::Loaded
}

/// The runs of the first line of a body, as a page shows them: the letters with their looks
/// and their links.
fn first_line_runs(body: &str, tags_dir: &str) -> Vec<RichRun> {
    RichText::of_markdown(first_line(body), tags_dir)
        .blocks
        .into_iter()
        .find_map(|block| match block {
            RichBlock::Text { paragraphs } => paragraphs.into_iter().next(),
            _ => None,
        })
        .map(|paragraph| paragraph.runs)
        .unwrap_or_default()
}

fn shown_atlas_page(model: &Model) -> Option<&Page> {
    super::page::shown_doc_page(model).filter(|p| super::page::is_atlas(p))
}

/// The target typed in the dialog of a link, without the brackets of a link around it, as the
/// link adds its own. None when nothing is left, or when brackets are left inside.
fn typed_link_target(target: &str) -> Option<String> {
    let target = target.trim();
    let target = link_target(target).unwrap_or(target).trim();
    (!target.is_empty() && !target.contains("[[") && !target.contains("]]"))
        .then(|| target.to_string())
}

/// Adds a link card to the shown atlas page with its center at the point. The card shows once
/// the server has made its item.
pub fn add_link(model: &mut Model, x: f64, y: f64, target: String) -> Cmd {
    let Some(target) = typed_link_target(&target) else {
        return done();
    };
    if !x.is_finite() || !y.is_finite() {
        return done();
    }
    add_card(model, &target, x, y)
}

/// Links a link card to another page. An empty target changes nothing, as "Remove Link" takes
/// a card away. The card keeps the label of its link and the lines after the first.
pub fn set_link(model: &mut Model, item_id: String, target: String) -> Cmd {
    let Some(card) = link_cards(model).into_iter().find(|c| c.item_id == item_id) else {
        return done();
    };
    let Some(target) = typed_link_target(&target) else {
        return done();
    };
    let Some(item) = model
        .state
        .get_item_by_id(&item_id)
        .filter(|_| card.target.as_deref().is_some_and(|t| t != target))
    else {
        return done();
    };
    let body = plain::relinked(&item.body, &target);
    change(
        model,
        rewritten(&item_id, Some(body), None).into_iter().collect(),
    )
}

/// Takes a link card off the atlas: its item is deleted, and with it the link.
pub fn remove_link(model: &mut Model, item_id: String) -> Cmd {
    if !link_cards(model).iter().any(|c| c.item_id == item_id) {
        return done();
    }
    delete(model, item_id)
}

/// How long typing in the dialog of a link may pause before the page names are searched.
pub const LINK_SEARCH_DELAY_MS: u32 = 150;
/// How many page names the dialog of a link offers.
pub const LINK_SUGGESTIONS: u32 = 8;

/// The page names that match the text typed in the dialog of a link, found after a pause in
/// the typing. An empty text clears them.
pub fn search_links(model: &mut Model, query: String) -> Cmd {
    let cancel = cancel_timer(&mut model.atlas_link_search.timer_id);
    let query = query.trim().to_string();
    if query.is_empty() {
        model.atlas_link_search.suggestions.clear();
        return cancel;
    }
    let timer_id = model.next_timer_id();
    model.atlas_link_search.timer_id = timer_id;
    let search = run(move |api| async move {
        if api.sleep(timer_id, LINK_SEARCH_DELAY_MS).await != TimerOutput::Fired {
            return;
        }
        let titles = match api.search_page_names(&query, LINK_SUGGESTIONS).await {
            Ok(results) => results.into_iter().map(|r| r.page_title).collect(),
            Err(e) => {
                log_error(format!("Atlas link search failed: {e}"));
                Vec::new()
            }
        };
        api.send_event(Event::AtlasLinksSuggested { timer_id, titles });
    });
    Command::all([cancel, search])
}

/// The page names the search the timer `timer_id` started found. The names of an older
/// search are not shown.
pub fn show_link_suggestions(model: &mut Model, timer_id: u64, titles: Vec<String>) -> Cmd {
    if model.atlas_link_search.timer_id == timer_id {
        model.atlas_link_search.timer_id = 0;
        model.atlas_link_search.suggestions = titles;
    }
    done()
}

/// The card of a page of the directory of the shown atlas page.
pub struct PageCard {
    pub page: Page,
    /// The label of the link of the card; none for a link without one, or a page with no card
    pub label: Option<String>,
    /// The whole body of the item of the card; empty for a page with no card
    pub text: String,
    pub bounds: Option<Bounds>,
    /// The items the card is under, the outermost first
    pub groups: Vec<AtlasGroup>,
}

/// The pages of the directory of the shown atlas in the order they are listed: the index page
/// of the directory itself, then the pages in the order of the sidebar's index list.
fn sorted_dir_pages(model: &Model) -> Vec<Page> {
    let sidebar = &model.sidebar;
    let listed = sidebar::sorted_index_pages(&sidebar.index_pages, sidebar::index_sort(model));
    sidebar.index_page.iter().cloned().chain(listed).collect()
}

/// The cards of the pages of the shown atlas page, in the order the pages are listed. The
/// item on the atlas page that links to the page holds the label, the place and the group. A
/// page that no card links to has none of them, and neither has a card without a place: both
/// wait beside the canvas ([`unplaced`]).
pub fn cards(model: &Model) -> Vec<PageCard> {
    if shown_atlas_page(model).is_none() {
        return Vec::new();
    }
    let cards = card_entries(model);
    sorted_dir_pages(model)
        .into_iter()
        .map(|page| {
            let card = cards.iter().find(|c| c.page_id == Some(page.id.as_str()));
            let link = card.and_then(|c| plain::card_link(&c.item.body));

            PageCard {
                label: link.and_then(|l| l.label).map(str::to_string),
                text: card.map(|c| c.item.body.clone()).unwrap_or_default(),
                bounds: card.and_then(|c| bounds_of(c.item)),
                groups: card.map(|c| groups(model, &c.parents)).unwrap_or_default(),
                page,
            }
        })
        .collect()
}

/// What waits beside the canvas of the shown atlas page: the pages not added, then the items
/// without a place (ADR 053).
/// A page not added is a page of the directory that no page card links to, in the order the
/// pages are listed. The link of a card without a place counts, and so does the link of a
/// parent, so no page waits in both lists.
/// An item without a place is a card or a callout without `atlas_bounds`, in the order of the
/// items. A parent is not placed by bounds of its own, as it is drawn around what is under
/// it, so it does not wait.
pub fn unplaced(model: &Model) -> Vec<UnplacedCard> {
    if shown_atlas_page(model).is_none() {
        return Vec::new();
    }
    let entries = entries(model);
    let linked: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == Kind::Card)
        .filter_map(|e| plain::card_link(&e.item.body))
        .filter_map(|link| page_of(model, link.target))
        .map(|page| page.id.as_str())
        .collect();
    let not_added = sorted_dir_pages(model)
        .into_iter()
        .filter(|page| !linked.contains(&page.id.as_str()))
        .map(|page| UnplacedCard {
            added: false,
            ..waiting_page(model, &page, None)
        });
    let cards = card_entries(model);
    let no_place = entries
        .iter()
        .filter(|e| !e.is_parent && bounds_of(e.item).is_none())
        .map(
            |e| match cards.iter().find(|card| card.item.id == e.item.id) {
                Some(card) => waiting_card(model, card),
                None => waiting_text(e.item, first_line(&e.item.body).trim().to_string()),
            },
        );
    not_added.chain(no_place).collect()
}

/// The first line of a body.
fn first_line(body: &str) -> &str {
    body.lines().next().unwrap_or("")
}

/// A page of the directory as it waits beside the canvas, shown by the label of its card when
/// it has one, or else by its name.
fn waiting_page(model: &Model, page: &Page, label: Option<&str>) -> UnplacedCard {
    UnplacedCard {
        id: page.id.clone(),
        label: label.map_or_else(|| sidebar::index_page_name(page), str::to_string),
        path: Some(opened_path(model, page)),
        is_dir: sidebar::is_directory(page),
        added: true,
    }
}

/// An item that opens nothing as it waits beside the canvas.
fn waiting_text(item: &Item, label: String) -> UnplacedCard {
    UnplacedCard {
        id: item.id.clone(),
        label,
        path: None,
        is_dir: false,
        added: true,
    }
}

/// A card without a place as it waits beside the canvas, named as the canvas names it.
fn waiting_card(model: &Model, card: &Card) -> UnplacedCard {
    let link = plain::card_link(&card.item.body);
    let page = card
        .page_id
        .and_then(|id| dir_pages(model).find(|page| page.id == id));
    match (page, link) {
        (Some(page), link) => waiting_page(model, page, link.and_then(|link| link.label)),
        (None, Some(link)) => UnplacedCard {
            path: link_path(model, link),
            ..waiting_text(card.item, link.shown())
        },
        (None, None) => {
            let runs = first_line_runs(&card.item.body, &model.config.tags_dir);
            waiting_text(
                card.item,
                runs.iter().map(|run| run.text.as_str()).collect(),
            )
        }
    }
}

/// The name of the directory above a directory, as the button that goes up shows it: its path
/// with a slash at the end, or `(root)/` for the root, as in the title of an atlas.
pub fn parent_dir_label(dir_path: &str) -> String {
    match dir_path.trim_end_matches('/').rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/"),
        None => "(root)/".to_string(),
    }
}

/// The path a folder opens at: its atlas page when the folder was last seen as a map, so
/// it opens as it was left. Any other path stays as it is. The map button and the URL open
/// the index page itself.
pub fn folder_path(model: &Model, path: &str) -> String {
    let (dir, name) = split_doc_path(path);
    if name == "index" && model.sidebar.atlas_dirs.contains(&dir) {
        format!("{dir}{ATLAS_NAME}")
    } else {
        path.to_string()
    }
}

/// Notes how a folder is seen when its index page or its atlas page is opened. The shell
/// keeps the list only when it changes.
pub fn remember_folder_view(model: &mut Model, path: &str) -> Cmd {
    let (dir, name) = split_doc_path(path);
    let changed = match name.as_str() {
        ATLAS_NAME => model.sidebar.atlas_dirs.insert(dir),
        "index" => model.sidebar.atlas_dirs.remove(&dir),
        _ => false,
    };
    if !changed {
        return done();
    }
    let dirs = model.sidebar.atlas_dirs.iter().cloned().collect();
    Command::notify_shell(UiOp::SaveAtlasDirs(dirs)).into()
}
