//! The canvas of an atlas page: the pages and folders of its directory as cards. The atlas
//! page is a plain page (ADR 053): the server writes nothing into it by itself, and its
//! outline may be edited here, in the outline or in the `.md`.
//! A card is an item with something written in it and no item under it. One whose first line
//! is one wiki link stands for the page of the directory that the link names, or else it is a
//! link card; any other is a plain card, which shows its first line. A card lies where the `atlas_bounds`
//! property of its item says. A parent has `atlas_bounds` too, and those of an item are counted
//! from the item it is right under; at the top, from the center of the canvas. An item without
//! the property is not drawn, and neither is any item under it: they wait beside the canvas
//! among the items without a place.
//! A page of the directory that no card links to waits there too, among the pages not added:
//! dropped on the canvas, it gets an item that links to it.
//! A group is an item with items under it, at any depth, and a card is in every group above
//! it; a callout is an item whose `atlas_type` is `callout`. A group that is a page card or a
//! callout is drawn as one along its outline.
//! The canvas edits the page with the item APIs that the outline uses, and no other. The
//! place of a card or a callout is saved as a property of its item. A change of the outline
//! is one batch of item operations, however many items it writes, so one operation of the
//! user is one change on the server. The outline is then read again and shown.
//! A card that goes under an item goes in among the items under it by where it lies: before
//! the first of them that lies below it. The others keep their order, so the canvas does not
//! write over an order made in the outline (ADR 053).
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

/// The `atlas_bounds` of an item as they are stored: counted from the item it is right under,
/// or from the center of the canvas at the top (ADR 053).
fn bounds_of(item: &Item) -> Option<Bounds> {
    peta_atlas::bounds_of(item)
}

/// A card of the shown atlas page.
struct Card<'a> {
    item: &'a Item,
    /// The items the card is under, the outermost first
    parents: Vec<&'a Item>,
    /// The page of the directory the card stands for; none for a link card
    page_id: Option<&'a str>,
    /// Where the card lies on the canvas; none for a card that is not drawn
    place: Option<Bounds>,
}

impl Card<'_> {
    /// The name the shell calls the card by: the id of its page, or of the item of a link card.
    fn id(&self) -> &str {
        self.page_id.unwrap_or(&self.item.id)
    }

    /// The bounds of the card as they are stored, when the card is drawn: a card is not drawn
    /// while an item it is under has no place.
    fn drawn_bounds(&self) -> Option<Bounds> {
        self.place.and_then(|_| bounds_of(self.item))
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
                parents: e.parents,
                page_id,
                place: e.place,
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

/// The item the shell names to place it, with the point of the canvas its bounds are counted
/// from: a card by the id of its page or of its item, a callout or a parent by the id of its
/// item. The point is none while an item it is under is not drawn.
fn placeable(model: &Model, id: &str) -> Option<(Item, Option<(f64, f64)>)> {
    let item_id = card_item(model, id).map_or_else(|| id.to_string(), |item| item.id);
    entries(model)
        .into_iter()
        .find(|e| e.item.id == item_id)
        .map(|e| (e.item.clone(), e.frame))
}

/// Places an item with its center at the given point of the canvas: a card, named by its page
/// or a link card by its item, or a callout or a parent, named by its item. The shell knows
/// where it was grabbed, so it gives the center rather than the drop point. The point is saved
/// counted from the item it is under (ADR 053). The item moves at once; a refusal of the server
/// puts it back. A parent takes all that is under it along, as their bounds are counted from
/// it and do not change. An item under an item that is not drawn is not on the canvas, and
/// is not placed here: it is dropped from the list beside the canvas ([`drop_card`]). A page
/// of the directory that has no card yet gets a card there.
pub fn place_card(model: &mut Model, card_id: String, x: f64, y: f64) -> Cmd {
    if !x.is_finite() || !y.is_finite() {
        return done();
    }
    let Some((item, frame)) = placeable(model, &card_id) else {
        return add_page_card(model, &card_id, x, y);
    };
    let Some((frame_x, frame_y)) = frame else {
        return done();
    };
    let bounds = placed_at(&item, x - frame_x, y - frame_y);
    save_bounds(model, &item, bounds)
}

/// The bounds of an item placed with its center at a point. It keeps its size when it moves;
/// whole pixels keep the stored value short.
fn placed_at(item: &Item, x: f64, y: f64) -> Bounds {
    let (width, height) = bounds_of(item)
        .map(|b| (b.width, b.height))
        .unwrap_or((AUTO_WIDTH, CARD_HEIGHT));
    Bounds {
        x: x.round(),
        y: y.round(),
        width,
        height,
    }
}

/// Gives a page of the directory that no card links to a card with its center at the point:
/// an item that links to the page, at the top of the outline, so the point is counted from the
/// center of the canvas. The card shows once the server has made the item.
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
    change(model, new_card(model, target, x, y).into_iter().collect())
}

/// The operation that makes an item that links to `target` at the end of the outline, at the
/// top, with its center at the point of the canvas. Its id is [`NEW_CARD`] until it is made.
/// None on a page that is no atlas.
fn new_card(model: &Model, target: &str, x: f64, y: f64) -> Option<BatchOpInput> {
    let atlas_id = shown_atlas_page(model)?.id.clone();
    let bounds = Bounds {
        x: x.round(),
        y: y.round(),
        width: AUTO_WIDTH,
        height: CARD_HEIGHT,
    };
    Some(new_item(
        atlas_id,
        NEW_CARD.to_string(),
        ItemPosition::Last,
        link_body(target),
        &[(BOUNDS_KEY, bounds.to_string())],
    ))
}

/// Places what was dropped from the lists beside the canvas: a page not added, or an item
/// without a place. It is placed by its center, which lies this far from the grabbed spot of
/// a card of the list.
/// Whatever is dropped becomes an item of the top at the end of the page, with the point of
/// the drop as its bounds, wherever it was in the outline (ADR 053): a page not added gets its
/// item there, and an item without a place moves there ([`take_to_top`]).
/// With Ctrl held on a card, `target_id` names that card, and what is dropped becomes its
/// sibling ([`sibling_ops`]). With Ctrl held on a group, or with Shift and Ctrl on a card,
/// `parent_id` names the item it goes under ([`under_ops`]). Either is one batch, the same as
/// for what is dragged on the canvas. When the named item cannot take it, it goes to the top.
#[allow(clippy::too_many_arguments)]
pub fn drop_card(
    model: &mut Model,
    card_id: String,
    x: f64,
    y: f64,
    grab_x: f64,
    grab_y: f64,
    target_id: Option<String>,
    parent_id: Option<String>,
) -> Cmd {
    let x = x + (0.5 - grab_x) * CARD_WIDTH;
    let y = y + (0.5 - grab_y) * CARD_HEIGHT;
    if !x.is_finite() || !y.is_finite() {
        return done();
    }
    let dropped = [AtlasPlace {
        id: card_id.clone(),
        x,
        y,
    }];
    let joined = |pieces: &[Piece<'_>]| match (&target_id, &parent_id) {
        (Some(target_id), _) => sibling_ops(model, pieces, target_id),
        (None, Some(parent_id)) => under_ops(model, pieces, parent_id),
        (None, None) => Vec::new(),
    };
    if let Some(piece) = pieces(model, std::slice::from_ref(&card_id), &dropped).pop() {
        let ops = joined(std::slice::from_ref(&piece));
        let item = piece.item.clone();
        if ops.is_empty() {
            return take_to_top(model, &item, x, y);
        }
        // The item is not drawn, so nothing shows that its bounds go: the outline that comes
        // back then shows the bounds the batch wrote, whatever it had
        set_property(model, &item.id, None);
        return change(model, ops);
    }
    let Some(page) = dir_pages(model).find(|p| p.id == card_id) else {
        return done();
    };
    // The item of the page is made at the end of the outline, and joins from there
    let made = new_card(model, &format!("{}{}", page.dir_path, page.name), x, y);
    let Some(BatchOpInput::Create { properties, .. }) = &made else {
        return done();
    };
    // What the join reads of the item that is not made yet: its name and its bounds
    let card = Item {
        id: NEW_CARD.to_string(),
        properties: properties.clone(),
        ..Item::default()
    };
    let piece = Piece {
        item: &card,
        below: Vec::new(),
        group: None,
        place: bounds_of(&card),
        frame: Some((0.0, 0.0)),
        dropped: true,
    };
    let ops = joined(std::slice::from_ref(&piece));
    change(model, made.into_iter().chain(ops).collect())
}

/// Makes an item an item of the top at the end of the page, with its center at the point of
/// the canvas, in one batch. It leaves the item it is under, drawn or not. The items under it
/// go with it, each as deep under it as before, and keep their bounds, which are counted from
/// it. The parent it leaves loses its side when nothing stays under it.
/// The outline shows the move at once, so what the shell sends next finds the item where it
/// was dropped. A refusal of the server puts the outline back.
fn take_to_top(model: &mut Model, item: &Item, x: f64, y: f64) -> Cmd {
    let Some(atlas_id) = shown_atlas_page(model).map(|p| p.id.clone()) else {
        return done();
    };
    let bounds = PropertyValue::Text(placed_at(item, x, y).to_string());
    let below: Vec<&Item> = under(model, item).collect();
    let leaving: Vec<&Item> = std::iter::once(item).chain(below.iter().copied()).collect();
    let rise = item.level - 1;
    let leveled = |item: &Item, bounds: Option<&PropertyValue>| BatchOpInput::Update {
        id: item.id.clone(),
        body: None,
        level: Some(item.level - rise),
        collapsed: None,
        task: None,
        properties: bounds
            .map(|bounds| (BOUNDS_KEY.to_string(), bounds.clone()))
            .into_iter()
            .collect(),
        dropped_properties: Vec::new(),
    };
    let mut ops = vec![leveled(item, Some(&bounds))];
    ops.extend(below.iter().map(|below| leveled(below, None)));
    ops.push(BatchOpInput::Move {
        ids: leaving.iter().map(|item| item.id.clone()).collect(),
        page_id: atlas_id.clone(),
        position: ItemPosition::Last,
    });
    let parent = parent_of(model, item);
    ops.extend(parent.and_then(|parent| emptied(model, parent, &leaving)));

    let before: Vec<Item> = page_items(model, &atlas_id).cloned().collect();
    let is_leaving = |shown: &Item| leaving.iter().any(|item| item.id == shown.id);
    let moved = before
        .iter()
        .filter(|shown| is_leaving(shown))
        .map(|shown| {
            let mut moved = shown.clone();
            moved.level -= rise;
            if moved.id == item.id {
                let bounds = bounds.clone();
                moved.properties.insert(BOUNDS_KEY.to_string(), bounds);
            }
            moved
        });
    let staying = before.iter().filter(|shown| !is_leaving(shown)).cloned();
    let after: Vec<Item> = staying.chain(moved).collect();
    file_reload::replace_page_items(model, &atlas_id, after);

    let item_id = item.id.clone();
    let before_bounds = item.properties.get(BOUNDS_KEY).cloned();
    run(move |api| async move {
        if let Err(e) = api.apply_batch(ops).await {
            api.alert(e.0);
            // The bounds first: the outline that comes next keeps the bounds shown for an
            // item that stays under the same item
            api.send_event(Event::AtlasBoundsRefused {
                item_id,
                before: before_bounds,
            });
            api.send_event(Event::AtlasChanged {
                atlas_id,
                items: before,
            });
            return;
        }
        match api.get_page_by_id(&atlas_id).await {
            Ok(page) => api.send_event(Event::AtlasChanged {
                atlas_id,
                items: page.items,
            }),
            Err(e) => api.alert(e.0),
        }
    })
}

/// The items of a page as they are shown, in their order.
fn page_items<'a>(model: &'a Model, page_id: &'a str) -> impl Iterator<Item = &'a Item> {
    let items = model.state.items().iter();
    items
        .filter(move |item| item.page_id == page_id)
        .map(|item| &**item)
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

/// The id of the card that a batch makes, until the id is made ([`change`]).
const NEW_CARD: &str = "\u{0}card";
/// The id of the group that a batch makes, until the id is made ([`change`]).
const NEW_GROUP: &str = "\u{0}group";

/// Sends a change of the outline of the shown atlas page as one batch and shows the outline
/// as the server has it then. A change that writes nothing sends nothing.
/// An item that the batch makes is named [`NEW_CARD`] or [`NEW_GROUP`] in the operations. Its
/// id is made here, so the operations that follow in the same batch can name the item.
fn change(model: &Model, ops: Vec<BatchOpInput>) -> Cmd {
    let Some(atlas_id) = shown_atlas_page(model).map(|p| p.id.clone()) else {
        return done();
    };
    if ops.is_empty() {
        return done();
    }
    run(move |api| async move {
        let mut ops = ops;
        for made in [NEW_CARD, NEW_GROUP] {
            if !ops
                .iter_mut()
                .any(|op| ids_in(op).any(|id| id.as_str() == made))
            {
                continue;
            }
            let (_, now_ms) = api.local_now().await;
            // A generator of its own: the one of the model cannot be reached from here, and
            // an id is unique by its random part
            let id = hmtid::Generator::for_pages_and_items().generate_at(now_ms);
            for named in ops
                .iter_mut()
                .flat_map(ids_in)
                .filter(|id| id.as_str() == made)
            {
                *named = id.as_str().to_string();
            }
        }
        send(&api, atlas_id, ops).await
    })
}

/// The ids of items that an operation names: the item it writes, and the item it puts others
/// behind.
fn ids_in(op: &mut BatchOpInput) -> impl Iterator<Item = &mut String> {
    let (ids, position): (Vec<&mut String>, Option<&mut ItemPosition>) = match op {
        BatchOpInput::Create { id, position, .. } => (vec![id], Some(position)),
        BatchOpInput::Move { ids, position, .. } => (ids.iter_mut().collect(), Some(position)),
        BatchOpInput::Update { id, .. } | BatchOpInput::Delete { id } => (vec![id], None),
    };
    let behind = position.and_then(|position| match position {
        ItemPosition::After(id) => Some(id),
        _ => None,
    });
    ids.into_iter().chain(behind)
}

/// Sends the batch, reads the outline again and shows it. A card keeps the place shown for
/// it, unless the batch put it under another item ([`show_changed`]): a card placed and then
/// changed at once may be read before its place is saved.
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

/// The body an item gets when its last child leaves it and it has none of its own: it is a
/// card then, and a card with nothing written is not drawn (ADR 053).
pub const NO_TITLE: &str = "no title";

/// The operation that tidies `parent` when no item stays under it while the items `leaving`
/// go: it is no parent then. It stays in the outline. It loses its side, as it has no title
/// to lay along one; not a callout, whose side is of its own bubble. An empty body becomes
/// [`NO_TITLE`], so the item stays on the canvas as a card. None for an item that keeps an
/// item under it, and for one that needs neither.
fn emptied(model: &Model, parent: &Item, leaving: &[&Item]) -> Option<BatchOpInput> {
    let is_emptied = last_staying(model, parent, leaving).id == parent.id;
    let has_side = parent.prop_str(SIDE_KEY).is_some();
    let is_callout = parent.prop_str(TYPE_KEY) == Some(CALLOUT_TYPE);
    let body = parent.body.trim().is_empty().then(|| NO_TITLE.to_string());
    let side: Vec<String> = (has_side && !is_callout)
        .then(|| SIDE_KEY.to_string())
        .into_iter()
        .collect();
    (is_emptied && (body.is_some() || !side.is_empty())).then(|| BatchOpInput::Update {
        id: parent.id.clone(),
        body,
        level: None,
        collapsed: None,
        task: None,
        properties: Default::default(),
        dropped_properties: side,
    })
}

/// Deletes an item of the shown atlas page. The item it was under is tidied in the same batch
/// when nothing else is under it ([`emptied`]).
fn delete(model: &Model, item_id: String) -> Cmd {
    let Some(item) = model.state.get_item_by_id(&item_id) else {
        return done();
    };
    let side = parent_of(model, &item).and_then(|parent| emptied(model, parent, &[&*item]));
    let ops = std::iter::once(BatchOpInput::Delete { id: item_id })
        .chain(side)
        .collect();
    change(model, ops)
}

/// The operations that take items behind the item `after` at a level, in their order; none
/// for no items. The level is written first: the server checks the levels of the items it
/// moves. With the level go the bounds counted from the new parent, when they change. What is
/// under an item goes with it, as deep under it as before.
fn taken_behind(atlas_id: &str, items: &[Moving], after: &str, level: i32) -> Vec<BatchOpInput> {
    if items.is_empty() {
        return Vec::new();
    }
    let leveled = |item: &Item, level: i32, bounds: Option<Bounds>| BatchOpInput::Update {
        id: item.id.clone(),
        body: None,
        level: Some(level),
        collapsed: None,
        task: None,
        properties: bounds
            .map(|bounds| {
                let value = PropertyValue::Text(bounds.to_string());
                (BOUNDS_KEY.to_string(), value)
            })
            .into_iter()
            .collect(),
        dropped_properties: Vec::new(),
    };
    let mut ops = Vec::new();
    for moving in items {
        ops.push(leveled(moving.item, level, moving.new_bounds()));
        let deeper = level - moving.item.level;
        let below = moving.below.iter();
        ops.extend(below.map(|below| leveled(below, below.level + deeper, None)));
    }
    ops.push(BatchOpInput::Move {
        ids: items
            .iter()
            .flat_map(Moving::items)
            .map(|item| item.id.clone())
            .collect(),
        page_id: atlas_id.to_string(),
        position: ItemPosition::after(after),
    });
    ops
}

/// The last item of the tree of `parent` that stays where it is while the items `leaving`
/// move: the parent itself when every item under it leaves.
fn last_staying<'a>(model: &'a Model, parent: &'a Item, leaving: &[&Item]) -> &'a Item {
    under(model, parent)
        .filter(|item| !leaving.iter().any(|l| l.id == item.id))
        .last()
        .unwrap_or(parent)
}

/// The server changed the outline of an atlas page: its items are shown. A card keeps the
/// place shown for it, as its move may not be saved yet. A card that went under another item
/// takes the bounds of the server: the batch wrote them anew, counted from that item.
pub fn show_changed(model: &mut Model, atlas_id: &str, mut items: Vec<Item>) -> Cmd {
    let parents: Vec<Option<String>> = (0..items.len())
        .map(|at| {
            let above = items[..at].iter().rev();
            let parent = above.into_iter().find(|i| i.level < items[at].level);
            parent.map(|parent| parent.id.clone())
        })
        .collect();
    for (item, parent) in items.iter_mut().zip(parents) {
        let Some(shown) = model.state.get_item_by_id(&item.id) else {
            continue;
        };
        let stays = parent_of(model, &shown).map(|p| &p.id) == parent.as_ref();
        if let Some(bounds) = shown.properties.get(BOUNDS_KEY).filter(|_| stays) {
            item.properties
                .insert(BOUNDS_KEY.to_string(), bounds.clone());
        }
    }
    file_reload::replace_page_items(model, atlas_id, items);
    done()
}

/// Where a card, a callout or a group that was dragged was let go, as a point of the canvas:
/// the center of a card or of the character of a callout, or the place of the item of a
/// group. `id` names it as the shell does: a card by the id of its page or of its item, a
/// callout and a group by the id of its item.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AtlasPlace {
    pub id: String,
    pub x: f64,
    pub y: f64,
}

/// What is dragged to join a group or to leave one: a card, a callout or a parent, with all
/// that is under it.
struct Piece<'a> {
    item: &'a Item,
    /// The items under it, at any depth, which go where it goes
    below: Vec<&'a Item>,
    /// The item it is right under
    group: Option<&'a Item>,
    /// Where it lies on the canvas: where it was let go, or else where it is shown. None for
    /// one that is not drawn.
    place: Option<Bounds>,
    /// The point of the canvas its bounds are counted from: where its group lies. None while
    /// an item it is under is not drawn.
    frame: Option<(f64, f64)>,
    /// True when it was let go at a place of its own
    dropped: bool,
}

impl<'a> Piece<'a> {
    /// The item and the items under it, in their order.
    fn items(&self) -> impl Iterator<Item = &'a Item> + '_ {
        std::iter::once(self.item).chain(self.below.iter().copied())
    }

    /// Whether the item is this one or one under it: nothing goes under an item of its own
    /// tree.
    fn holds(&self, item: &Item) -> bool {
        self.items().any(|own| own.id == item.id)
    }

    /// The point of the canvas the bounds of its group are counted from: where the piece is
    /// counted from once it is taken out of the group.
    fn frame_of_group(&self) -> Option<(f64, f64)> {
        let (x, y) = self.frame?;
        let group = bounds_of(self.group?)?;
        Some((x - group.x, y - group.y))
    }
}

/// The pieces the shell names, in the order of the outline: a card by the id of its page or
/// of its item, a callout and a parent by the id of its item. One that was let go at a place
/// lies there. One under another of them is no piece of its own: it goes with that one.
fn pieces<'a>(model: &'a Model, ids: &[String], places: &[AtlasPlace]) -> Vec<Piece<'a>> {
    let cards = card_entries(model);
    let name_of = |item: &Item| -> String {
        let card = cards.iter().find(|card| card.item.id == item.id);
        card.map_or(item.id.as_str(), |card| card.id()).to_string()
    };
    let entries = entries(model);
    let named: Vec<(&Entry, String)> = entries
        .iter()
        .map(|e| (e, name_of(e.item)))
        .filter(|(_, name)| ids.contains(name))
        .collect();
    let is_named = |item: &Item| named.iter().any(|(e, _)| e.item.id == item.id);
    named
        .iter()
        .filter(|(e, _)| !e.parents.iter().any(|parent| is_named(parent)))
        .map(|(e, name)| {
            let dropped = places.iter().find(|place| &place.id == name);
            Piece {
                item: e.item,
                below: under(model, e.item).collect(),
                group: e.group(),
                place: match dropped {
                    Some(at) => Some(placed_at(e.item, at.x, at.y)),
                    None => e.place,
                },
                frame: e.frame,
                dropped: dropped.is_some(),
            }
        })
        .collect()
}

/// An item that goes under another item, with the bounds it takes there.
#[derive(Clone)]
struct Moving<'a> {
    item: &'a Item,
    /// The items under it, which go with it
    below: Vec<&'a Item>,
    /// Its bounds counted from its new parent, so it stays where it lies on the canvas. None
    /// for an item that lies nowhere: it gets no bounds.
    bounds: Option<Bounds>,
}

impl<'a> Moving<'a> {
    /// The piece as it goes right under the item that lies at `origin` on the canvas, or to
    /// the top for the center of the canvas.
    fn of(piece: &Piece<'a>, origin: Option<(f64, f64)>) -> Moving<'a> {
        let bounds = match (piece.place, origin) {
            (Some(place), Some((x, y))) => Some(place.moved_by(-x, -y)),
            _ => None,
        };
        Moving {
            item: piece.item,
            below: piece.below.clone(),
            bounds,
        }
    }

    /// The bounds to write: none when they stay as they are stored.
    fn new_bounds(&self) -> Option<Bounds> {
        self.bounds
            .filter(|bounds| bounds_of(self.item) != Some(*bounds))
    }

    /// The item and the items under it, in their order.
    fn items(&self) -> impl Iterator<Item = &'a Item> + '_ {
        std::iter::once(self.item).chain(self.below.iter().copied())
    }
}

/// Cards, callouts or groups let go on a card with Ctrl held become siblings of that card
/// ([`sibling_ops`]), in one batch. `places` are where they were let go; one without a place
/// stays where it is shown.
pub fn group_cards(
    model: &mut Model,
    card_ids: Vec<String>,
    target_id: String,
    places: Vec<AtlasPlace>,
) -> Cmd {
    let pieces = pieces(model, &card_ids, &places);
    change(model, sibling_ops(model, &pieces, &target_id))
}

/// The operations that make pieces siblings of the card `target_id`: they join its group, each
/// where it lies among the items under the item of the group ([`put_under`]). When the card
/// is in no group, a new item with an empty body is made where the card lies in the outline,
/// and the card and the pieces go under it, the upper ones first. The new item lies at the
/// centroid of them on the canvas. Either way, their bounds are written anew in the same
/// batch, counted from the item they go under, so they stay where they lie (ADR 053).
/// A piece belongs to one group only, so the pieces leave the groups they were in. The item of
/// a group they leave stays ([`emptied`]). A piece does not join a group of its own tree, and
/// one already in the group stays unless it was let go at a place. None for no card of that
/// name, and when nothing joins.
fn sibling_ops(model: &Model, pieces: &[Piece], target_id: &str) -> Vec<BatchOpInput> {
    let is_card = card_entries(model).iter().any(|c| c.id() == target_id);
    let target = pieces_named(model, target_id).filter(|_| is_card);
    let Some(target) = target else {
        return Vec::new();
    };
    let joining = pieces.iter().filter(|piece| !piece.holds(target.item));
    match target.group {
        Some(group) => {
            let joining: Vec<&Piece> = joining
                .filter(|piece| !piece.holds(group))
                .filter(|piece| piece.dropped || !is_under(piece, group))
                .collect();
            // Counted from where the group of the target lies, as the target is
            let moving: Vec<Moving> = joining
                .iter()
                .map(|piece| Moving::of(piece, target.frame))
                .collect();
            let mut ops = put_under(model, group, &moving);
            ops.extend(tidied(model, &joining, Some(group)));
            ops
        }
        None => {
            let joining: Vec<&Piece> = joining.collect();
            if joining.is_empty() {
                return Vec::new();
            }
            let members: Vec<&Piece> = std::iter::once(&target)
                .chain(joining.iter().copied())
                .collect();
            let origin = centroid(&members);
            let moving: Vec<Moving> = members
                .iter()
                .map(|piece| Moving::of(piece, origin))
                .collect();
            let bounds = origin.map(|(x, y)| Bounds {
                x,
                y,
                width: AUTO_WIDTH,
                height: CARD_HEIGHT,
            });
            let bounds: Vec<(&str, String)> = bounds
                .map(|bounds| (BOUNDS_KEY, bounds.to_string()))
                .into_iter()
                .collect();
            let page_id = &target.item.page_id;
            let mut ops = vec![new_item(
                page_id.clone(),
                NEW_GROUP.to_string(),
                ItemPosition::after(&target.item.id),
                String::new(),
                &bounds,
            )];
            let members = in_reading_order(&moving);
            let level = target.item.level + 1;
            ops.extend(taken_behind(page_id, &members, NEW_GROUP, level));
            ops.extend(tidied(model, &joining, None));
            ops
        }
    }
}

/// The piece of one name, as it is shown.
fn pieces_named<'a>(model: &'a Model, id: &str) -> Option<Piece<'a>> {
    pieces(model, &[id.to_string()], &[]).pop()
}

/// Whether the piece is right under the item.
fn is_under(piece: &Piece, parent: &Item) -> bool {
    piece.group.is_some_and(|group| group.id == parent.id)
}

/// The centroid of where the pieces lie on the canvas, in whole numbers: the place of a parent
/// that is made for them (ADR 053). A piece that is not drawn does not count; none when no
/// piece is drawn.
fn centroid(pieces: &[&Piece]) -> Option<(f64, f64)> {
    let places: Vec<Bounds> = pieces.iter().filter_map(|piece| piece.place).collect();
    let mean =
        |of: fn(&Bounds) -> f64| (places.iter().map(of).sum::<f64>() / places.len() as f64).round();
    (!places.is_empty()).then(|| (mean(|b| b.x), mean(|b| b.y)))
}

/// Cards, callouts or groups let go on the blob of a parent with Ctrl held, or on a card with
/// Shift and Ctrl held, go under that item ([`under_ops`]), in one batch. `places` are where
/// they were let go; one without a place stays where it is shown.
pub fn move_cards(
    model: &mut Model,
    card_ids: Vec<String>,
    parent_id: String,
    places: Vec<AtlasPlace>,
) -> Cmd {
    let pieces = pieces(model, &card_ids, &places);
    change(model, under_ops(model, &pieces, &parent_id))
}

/// The operations that put pieces under the item `parent_id`, each where it lies among the
/// items under it ([`put_under`]). A parent is named by its item, and a card as the shell names
/// it: a card becomes a parent this way (ADR 053). Its own bounds stay, and those of the pieces
/// are written anew in the same batch, counted from it, so they stay where they lie on the
/// canvas. A piece right under the item already stays where it is in the outline, unless it
/// was let go at a place. Nothing goes under an item of its own tree. The parent a piece
/// leaves stays ([`emptied`]).
fn under_ops(model: &Model, pieces: &[Piece], parent_id: &str) -> Vec<BatchOpInput> {
    let cards = card_entries(model);
    let of_parent = entries(model)
        .into_iter()
        .find(|e| e.is_parent && e.item.id == parent_id)
        .map(|e| (e.item, e.place));
    let of_card = || {
        let card = cards.iter().find(|card| card.id() == parent_id);
        card.map(|card| (card.item, card.place))
    };
    let Some((parent, place)) = of_parent.or_else(of_card) else {
        return Vec::new();
    };
    let origin = place.map(|place| (place.x, place.y));
    let joining: Vec<&Piece> = pieces
        .iter()
        .filter(|piece| !piece.holds(parent))
        .filter(|piece| piece.dropped || !is_under(piece, parent))
        .collect();
    let moving: Vec<Moving> = joining
        .iter()
        .map(|piece| Moving::of(piece, origin))
        .collect();
    let mut ops = put_under(model, parent, &moving);
    ops.extend(tidied(model, &joining, Some(parent)));
    ops
}

/// The items under an item, at any depth, in their order.
fn under<'a>(model: &'a Model, parent: &Item) -> impl Iterator<Item = &'a Item> {
    let items = model.state.items();
    let start = items.iter().position(|item| item.id == parent.id);
    let (page_id, level) = (parent.page_id.clone(), parent.level);
    items
        .iter()
        .skip(start.map_or(items.len(), |at| at + 1))
        .take_while(move |item| item.page_id == page_id && item.level > level)
        .map(|item| &**item)
}

/// Where bounds lie as the order of the outline reads the canvas: the upper one comes first,
/// and of two at the same height the left one (ADR 053).
fn reading_place(bounds: Bounds) -> (f64, f64) {
    (bounds.y, bounds.x)
}

/// Whether an item lies below a card that goes under the same parent, both by their bounds
/// counted from that parent. An item without bounds lies nowhere, and neither is any item
/// below a card without them.
fn lies_below(item: &Item, card: &Moving) -> bool {
    match (bounds_of(item), card.bounds) {
        (Some(item), Some(card)) => reading_place(item) > reading_place(card),
        _ => false,
    }
}

/// The cards in the order the outline reads the canvas. Two at the same place keep their
/// order, and a card without a place comes last.
fn in_reading_order<'a>(cards: &[Moving<'a>]) -> Vec<Moving<'a>> {
    let place = |card: &Moving| card.bounds.map_or((f64::MAX, f64::MAX), reading_place);
    let mut cards = cards.to_vec();
    cards.sort_by(|a, b| {
        let (a, b) = (place(a), place(b));
        a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1))
    });
    cards
}

/// The operations that put cards under `parent`: each goes in before the first item right
/// under it that lies below the card, or else at the end of its tree. The card and the item
/// are compared by their bounds counted from the parent. Cards that go in at the same place go
/// in the upper ones first. The other items keep their order, so the canvas does not write
/// over an order made in the outline (ADR 053).
fn put_under(model: &Model, parent: &Item, joining: &[Moving]) -> Vec<BatchOpInput> {
    let level = parent.level + 1;
    let is_joining = |item: &Item| {
        let mut going = joining.iter().flat_map(Moving::items);
        going.any(|going| going.id == item.id)
    };
    let staying: Vec<&Item> = under(model, parent)
        .filter(|item| !is_joining(item))
        .collect();
    let joining = in_reading_order(joining);
    let siblings = || staying.iter().filter(|item| item.level == level);
    // The cards that go in before the sibling; before none, the ones no sibling lies below
    let going_before = |sibling: Option<&Item>| -> Vec<Moving> {
        let first_below = |card: &Moving| siblings().find(|s| lies_below(s, card)).map(|s| &s.id);
        joining
            .iter()
            .filter(|card| first_below(card) == sibling.map(|s| &s.id))
            .cloned()
            .collect()
    };
    let mut ops = Vec::new();
    // Behind the last item above the sibling: the end of the tree of the sibling before it
    let mut above = parent;
    for item in &staying {
        if item.level == level {
            let cards = going_before(Some(item));
            ops.extend(taken_behind(&parent.page_id, &cards, &above.id, level));
        }
        above = item;
    }
    let cards = going_before(None);
    ops.extend(taken_behind(&parent.page_id, &cards, &above.id, level));
    ops
}

/// The groups the pieces are right under, each once, in the order of the pieces.
fn groups_of<'a>(pieces: &[&Piece<'a>]) -> Vec<&'a Item> {
    let mut groups: Vec<&Item> = Vec::new();
    for group in pieces.iter().filter_map(|piece| piece.group) {
        if !groups.iter().any(|g| g.id == group.id) {
            groups.push(group);
        }
    }
    groups
}

/// The operations that tidy each group that the pieces leave with no item under it
/// ([`emptied`]). Not `joined`, the item they go under: a piece that only moves in it does
/// not leave it.
fn tidied(model: &Model, leaving: &[&Piece], joined: Option<&Item>) -> Vec<BatchOpInput> {
    let items: Vec<&Item> = leaving.iter().flat_map(|piece| piece.items()).collect();
    groups_of(leaving)
        .into_iter()
        .filter(|group| joined.is_none_or(|joined| joined.id != group.id))
        .filter_map(|group| emptied(model, group, &items))
        .collect()
}

/// Cards, callouts or groups taken out of their groups, as when they are dragged out of the
/// outline of their group with Ctrl held. A piece goes behind the tree of its group, at the
/// level of the group item. Its bounds are written anew in the same batch, counted from what
/// the group is counted from, so it stays where it lies on the canvas, or where it was let go
/// (ADR 053). The item of the group stays ([`emptied`]).
pub fn ungroup_cards(model: &mut Model, card_ids: Vec<String>, places: Vec<AtlasPlace>) -> Cmd {
    let pieces = pieces(model, &card_ids, &places);
    let leaving: Vec<&Piece> = pieces
        .iter()
        .filter(|piece| piece.group.is_some())
        .collect();
    let all: Vec<&Item> = leaving.iter().flat_map(|piece| piece.items()).collect();
    let ops: Vec<BatchOpInput> = groups_of(&leaving)
        .into_iter()
        .flat_map(|group| {
            let of_group: Vec<Moving> = leaving
                .iter()
                .filter(|piece| is_under(piece, group))
                .map(|piece| Moving::of(piece, piece.frame_of_group()))
                .collect();
            let last = last_staying(model, group, &all);
            let mut ops = taken_behind(&group.page_id, &of_group, &last.id, group.level);
            ops.extend(emptied(model, group, &all));
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
    /// Where the item lies, counted from the item it is right under: the `x` and `y` of its
    /// `atlas_bounds`. What is under the item is counted from here.
    pub x: f64,
    pub y: f64,
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
        .map(|parent| {
            let place = bounds_of(parent);
            AtlasGroup {
                id: parent.id.clone(),
                title: group_title(parent),
                look: group_look(model, parent),
                x: place.map_or(0.0, |b| b.x),
                y: place.map_or(0.0, |b| b.y),
            }
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
    /// The center of the character, counted from the item the callout is right under
    pub x: f64,
    pub y: f64,
    /// How large the whole callout is drawn, no smaller than `MIN_CARD_SCALE` as a card
    pub scale: f64,
    /// The side of the character the bubble is on, one of `TITLE_SIDES`
    pub side: String,
    pub text: String,
    /// The items the callout is under, the outermost first
    pub groups: Vec<AtlasGroup>,
}

/// The callouts of the shown atlas page, in their order. A callout without a place is left
/// out, and so is one under an item without a place.
pub fn shown_callouts(model: &Model) -> Vec<Callout> {
    entries(model)
        .into_iter()
        .filter(|e| e.kind == Kind::Callout && !e.is_parent && e.place.is_some())
        .filter_map(|e| {
            let bounds = bounds_of(e.item).filter(|b| b.height > 0.0)?;
            Some(Callout {
                key: e.item.id.clone(),
                x: bounds.x,
                y: bounds.y,
                scale: bounds.scale().max(MIN_CARD_SCALE),
                side: side_of(e.item),
                text: e.item.body.clone(),
                groups: groups(model, &e.parents),
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

/// Moves a callout of the shown atlas page so the center of its character is at the point of
/// the canvas. The point is saved counted from the item the callout is under.
pub fn move_callout(model: &mut Model, key: String, x: f64, y: f64) -> Cmd {
    let Some((item, callout)) = callout_item(model, &key) else {
        return done();
    };
    // A callout that is shown is under items that are drawn, so there is a point to count from
    let Some((_, Some((frame_x, frame_y)))) = placeable(model, &key) else {
        return done();
    };
    if !x.is_finite() || !y.is_finite() {
        return done();
    }
    let bounds = callout_bounds(x - frame_x, y - frame_y, callout.scale);
    save_bounds(model, &item, bounds)
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
    let Some(atlas_id) = shown_atlas_page(model).map(|p| p.id.clone()) else {
        return done();
    };
    let callout = new_item(
        atlas_id,
        NEW_CARD.to_string(),
        ItemPosition::Last,
        text,
        &[
            (TYPE_KEY, CALLOUT_TYPE.to_string()),
            (BOUNDS_KEY, bounds),
            (SIDE_KEY, side),
        ],
    );
    change(model, vec![callout])
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
/// place is left out, and so is one under an item without a place. The bounds of a card are
/// counted from the item it is right under.
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
                bounds: card.drawn_bounds()?,
                groups: groups(model, &card.parents),
            })
        })
        .collect()
}

/// How a card that stands for no page of the directory is colored: blue for another card to
/// a page of the directory, gray for one to a page the directory does not have, orange for a
/// link out of it, yellow for a plain card. The directory is known to lack a page only once its
/// list is in. A link to `today` is orange on any atlas: today's page can always be opened, and
/// the list of no directory holds it (2026-11-28).
fn tone_of(model: &Model, link: Option<plain::CardLink>) -> CardTone {
    match link {
        Some(link) if page_of(model, link.target).is_some() => CardTone::Page,
        Some(link) if link.target == TODAY => CardTone::Link,
        Some(link) if in_atlas_dir(model, link.target) && index_loaded(model) => CardTone::Missing,
        Some(_) => CardTone::Link,
        None => CardTone::Plain,
    }
}

/// The name a link opens today's page by.
const TODAY: &str = "today";

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
/// page that no card links to has none of them, and neither has a card without a place or
/// under an item without one: they wait beside the canvas ([`unplaced`]).
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
                bounds: card.and_then(Card::drawn_bounds),
                groups: card.map(|c| groups(model, &c.parents)).unwrap_or_default(),
                page,
            }
        })
        .collect()
}

/// What waits beside the canvas of the shown atlas page: the pages not added, then the items
/// without a place (ADR 053).
/// A page not added waits among the pages: a page of the directory that no card links to, or
/// one whose page card has no `atlas_bounds` of its own, in the order the pages are listed. A
/// page card always goes back to the pages, so the shell names it by its page and its item is
/// placed again. The link of a parent counts as a link, so no page waits in both lists.
/// An item without a place is one that is not drawn, but for those page cards: an item without
/// `atlas_bounds`, a parent too, and every item under such a parent, whatever its own
/// `atlas_bounds`. They wait in the order of the items.
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
    let cards = card_entries(model);
    // The cards of pages that have no bounds of their own: their pages wait among the pages
    let back: Vec<&Card> = cards
        .iter()
        .filter(|card| card.page_id.is_some() && bounds_of(card.item).is_none())
        .collect();
    let not_added = sorted_dir_pages(model)
        .into_iter()
        .filter(|page| {
            !linked.contains(&page.id.as_str())
                || back
                    .iter()
                    .any(|card| card.page_id == Some(page.id.as_str()))
        })
        .map(|page| UnplacedCard {
            added: false,
            ..waiting_page(model, &page, None)
        });
    let no_place = entries
        .iter()
        .filter(|e| e.place.is_none())
        .filter(|e| !back.iter().any(|card| card.item.id == e.item.id))
        .map(|e| {
            match cards.iter().find(|card| card.item.id == e.item.id) {
                Some(card) => waiting_card(model, card.item, card.page_id),
                None if e.kind == Kind::Callout => {
                    waiting_text(e.item, first_line(&e.item.body).trim().to_string())
                }
                // A parent is named as a card with its body would be
                None => waiting_card(model, e.item, None),
            }
        });
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

/// The item of a card or of a parent as it waits beside the canvas, named as the canvas names
/// it. `page_id` is the page of the directory a card stands for.
fn waiting_card(model: &Model, item: &Item, page_id: Option<&str>) -> UnplacedCard {
    let link = plain::card_link(&item.body);
    let page = page_id.and_then(|id| dir_pages(model).find(|page| page.id == id));
    match (page, link) {
        (Some(page), link) => waiting_page(model, page, link.and_then(|link| link.label)),
        (None, Some(link)) => UnplacedCard {
            path: link_path(model, link),
            ..waiting_text(item, link.shown())
        },
        (None, None) => {
            let runs = first_line_runs(&item.body, &model.config.tags_dir);
            waiting_text(item, runs.iter().map(|run| run.text.as_str()).collect())
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
