//! The canvas of an atlas page: the pages and folders of its directory as cards. The atlas
//! page is a plain page (ADR 053): the server writes nothing into it by itself, and its
//! outline may be edited here, in the outline or in the `.md`.
//! A card is an item whose body is one wiki link. It stands for the page of the directory
//! that the link names, or else it is a link card. A card lies where the `atlas_bounds`
//! property of its item says, in coordinates whose origin is the center of the canvas. A card
//! without the property waits in the list of unplaced cards, and so does a page of the
//! directory that no card links to.
//! A group is an item of level 1 with the cards of the group under it; a callout is an item
//! whose `atlas_type` is `callout`.
//! The canvas edits the page with the item APIs that the outline uses, and no other. The
//! place of a card or a callout is saved as a property of its item. A change of the outline
//! is one batch of item operations, however many items it writes, so one operation of the
//! user is one change on the server. The outline is then read again and shown.

use crate::app::{cancel_timer, done, log_error, run, Cmd, Event, Model};
use crate::effects::{Api, ApiResult, TimerOutput, UiOp};
use crate::model::ItemExt;
use crate::model::{BatchOpInput, Item, ItemPosition, Page, PropertyValue};
use crate::ops::file_reload;
use crate::screens::page::{split_doc_path, ATLAS_NAME};
use crate::screens::sidebar;
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

/// The width of a card in the list of unplaced cards, which a drop from the list reads.
pub const CARD_WIDTH: f64 = 120.0;

type Entry<'a> = plain::Entry<'a, Item>;

/// The items of the shown atlas page that the canvas can show, in their order; none on any
/// other page. A card below level 1 belongs to the group above it.
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
    /// The group item the card is under
    group: Option<&'a Item>,
    /// The page of the directory the card stands for; none for a link card
    page_id: Option<&'a str>,
}

impl Card<'_> {
    /// The name the shell calls the card by: the id of its page, or of the item of a link card.
    fn id(&self) -> &str {
        self.page_id.unwrap_or(&self.item.id)
    }
}

/// The page of the directory of the shown atlas that a link names by its path.
fn page_of<'a>(model: &'a Model, target: &str) -> Option<&'a Page> {
    let target = target.strip_suffix(".md").unwrap_or(target);
    model
        .sidebar
        .index_pages
        .iter()
        .find(|p| format!("{}{}", p.dir_path, p.name) == target)
}

/// The cards of the shown atlas page, in the order of its items. The first card that links
/// to a page of the directory stands for that page. Any other card is a link card: one that
/// links out of the directory, to no page, or to a page that has a card already.
fn card_entries(model: &Model) -> Vec<Card<'_>> {
    let mut taken: Vec<&str> = Vec::new();
    entries(model)
        .into_iter()
        .filter(|e| e.kind == Kind::Card)
        .map(|e| {
            let page_id = link_target(&e.item.body)
                .and_then(|target| page_of(model, target))
                .map(|page| page.id.as_str())
                .filter(|id| !taken.contains(id));
            taken.extend(page_id);
            Card {
                item: e.item,
                group: e.group,
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

/// Places a card with its center at the given point of the canvas. A card is named by its
/// page, or a link card by its item. The shell knows where the card was grabbed, so it gives
/// the center rather than the drop point. The card moves at once; a refusal of the server
/// puts it back. A page of the directory that has no card yet gets one there.
pub fn place_card(model: &mut Model, card_id: String, x: f64, y: f64) -> Cmd {
    if !x.is_finite() || !y.is_finite() {
        return done();
    }
    let Some(item) = card_item(model, &card_id) else {
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
    let Some(page) = model.sidebar.index_pages.iter().find(|p| p.id == page_id) else {
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

/// Places a card dropped from the list of unplaced cards. The card is placed by its center,
/// which lies this far from the grabbed spot of a card of the list.
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

/// Takes the card of a page off the canvas: its bounds are removed, so it waits in the list
/// of unplaced cards again. The card leaves at once; a refusal of the server puts it back.
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

/// What one change of the outline of the atlas page sends to the server.
#[derive(Default)]
struct Change {
    /// The item operations of the change, sent as one batch
    ops: Vec<BatchOpInput>,
    /// The side to save for an item, as `(item, side)`.
    // Sent in a request of its own after the batch, as the batch of the server cannot write
    // a property of an item that is there already (#1238). It belongs into the batch.
    side: Option<(String, String)>,
}

impl From<Vec<BatchOpInput>> for Change {
    fn from(ops: Vec<BatchOpInput>) -> Change {
        Change { ops, side: None }
    }
}

impl Change {
    fn is_empty(&self) -> bool {
        self.ops.is_empty() && self.side.is_none()
    }
}

/// Sends a change of the outline of the shown atlas page and shows the outline as the server
/// has it then. A change that writes nothing sends nothing.
fn change(model: &Model, change: impl Into<Change>) -> Cmd {
    let change = change.into();
    let Some(atlas_id) = shown_atlas_page(model).map(|p| p.id.clone()) else {
        return done();
    };
    if change.is_empty() {
        return done();
    }
    run(move |api| async move { send(&api, atlas_id, change).await })
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
        let change = make(atlas_id.clone(), id.as_str().to_string()).into();
        send(&api, atlas_id, change).await
    })
}

/// Sends the change, reads the outline again and shows it. A card keeps the place shown for
/// it: a card placed and grouped at once may be read before its place is saved.
async fn send(api: &Api<Event>, atlas_id: String, change: Change) {
    match written(api, &atlas_id, change).await {
        Ok(items) => api.send_event(Event::AtlasChanged { atlas_id, items }),
        Err(e) => api.alert(e.0),
    }
}

/// Writes the change and answers with the items of the atlas page as they are then.
async fn written(api: &Api<Event>, atlas_id: &str, change: Change) -> ApiResult<Vec<Item>> {
    if !change.ops.is_empty() {
        api.apply_batch(change.ops).await?;
    }
    if let Some((item_id, side)) = change.side {
        api.upsert_item_property(&item_id, SIDE_KEY, "text", &side)
            .await?;
    }
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

/// The operation that writes the body of an item.
fn new_body(item_id: &str, body: String) -> BatchOpInput {
    BatchOpInput::Update {
        id: item_id.to_string(),
        body: Some(body),
        level: None,
        collapsed: None,
        task: None,
    }
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
    let joining: Vec<&Item> = cards
        .iter()
        .filter(|card| card.id() != target_id && card_ids.iter().any(|id| id == card.id()))
        .filter(|card| !in_group_of_target(card))
        .map(|card| card.item)
        .collect();
    if joining.is_empty() {
        return done();
    }
    match target.group {
        Some(group) => {
            let last = last_staying(model, group, &joining);
            change(
                model,
                taken_behind(&group.page_id, &joining, &last.id, group.level + 1),
            )
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
                ops
            })
        }
    }
}

/// Cards taken out of their groups, as when they are dragged out of the outline of their
/// group with Ctrl held. A card goes behind the tree of its group, at the level of the group
/// item. The item of the group stays, even with one card or none under it (ADR 053).
pub fn ungroup_cards(model: &mut Model, card_ids: Vec<String>) -> Cmd {
    let cards = card_entries(model);
    let leaving: Vec<(&Item, &Item)> = cards
        .iter()
        .filter(|card| card_ids.iter().any(|id| id == card.id()))
        .filter_map(|card| Some((card.group?, card.item)))
        .collect();
    let all: Vec<&Item> = leaving.iter().map(|(_, item)| *item).collect();
    let mut groups: Vec<&Item> = Vec::new();
    for (group, _) in &leaving {
        if !groups.iter().any(|g| g.id == group.id) {
            groups.push(group);
        }
    }
    let ops: Vec<BatchOpInput> = groups
        .into_iter()
        .flat_map(|group| {
            let of_group: Vec<&Item> = leaving
                .iter()
                .filter(|(g, _)| g.id == group.id)
                .map(|(_, item)| *item)
                .collect();
            let last = last_staying(model, group, &all);
            taken_behind(&group.page_id, &of_group, &last.id, group.level)
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

/// Gives a group of the shown atlas page a title along a side of its outline. An empty title
/// takes the title away. The title is the body of the item of the group, and the side a
/// property of it.
pub fn set_group_title(model: &mut Model, group_id: String, side: String, title: String) -> Cmd {
    let Some(group) = entries(model)
        .into_iter()
        .find(|e| e.kind == Kind::Group && e.item.id == group_id)
        .map(|e| e.item.clone())
    else {
        return done();
    };
    let side = side_or_default(&side).to_string();
    let title = title.trim().to_string();
    // The title shown, and not the body: a body that is a link shows its label as the title
    let shown = plain::group_title(&group).unwrap_or_default();
    change(
        model,
        Change {
            ops: (shown != title)
                .then(|| new_body(&group_id, title))
                .into_iter()
                .collect(),
            side: (side_of(&group) != side).then_some((group_id, side)),
        },
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
        .filter(|e| e.kind == Kind::Callout)
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
        return change(model, vec![BatchOpInput::Delete { id: key }]);
    }
    let side = side_or_default(&side).to_string();
    change(
        model,
        Change {
            ops: (callout.text != text)
                .then(|| new_body(&key, text))
                .into_iter()
                .collect(),
            side: (callout.side != side).then_some((key, side)),
        },
    )
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

/// A link card: an item of the atlas page whose body is a wiki link to `target`, which is no
/// page of the directory, placed by its own `atlas_bounds`.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LinkCard {
    pub item_id: String,
    /// The page the link names, as it is written in the link
    pub target: String,
    pub bounds: Bounds,
    /// The id of the group item the card is under
    pub group: Option<String>,
    pub group_title: Option<GroupTitle>,
}

/// The link cards of the shown atlas page, in the order of its items. A link card without a
/// place is left out.
pub fn link_cards(model: &Model) -> Vec<LinkCard> {
    card_entries(model)
        .into_iter()
        .filter(|card| card.page_id.is_none())
        .filter_map(|card| {
            Some(LinkCard {
                item_id: card.item.id.clone(),
                target: link_target(&card.item.body)?.to_string(),
                bounds: bounds_of(card.item)?,
                group: card.group.map(|g| g.id.clone()),
                group_title: card.group.and_then(group_title),
            })
        })
        .collect()
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
/// a card away.
pub fn set_link(model: &mut Model, item_id: String, target: String) -> Cmd {
    let Some(card) = link_cards(model).into_iter().find(|c| c.item_id == item_id) else {
        return done();
    };
    let Some(target) = typed_link_target(&target) else {
        return done();
    };
    if target == card.target {
        return done();
    }
    change(model, vec![new_body(&item_id, link_body(&target))])
}

/// Takes a link card off the atlas: its item is deleted, and with it the link.
pub fn remove_link(model: &mut Model, item_id: String) -> Cmd {
    if !link_cards(model).iter().any(|c| c.item_id == item_id) {
        return done();
    }
    change(model, vec![BatchOpInput::Delete { id: item_id }])
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

/// The cards of the pages of the shown atlas page, in the order of the sidebar's index list,
/// each with its place, the id of its group and the title of that group. The item on the
/// atlas page that links to the page holds the place and the group. A page that no item links
/// to has neither: it waits in the list of unplaced cards until it is dropped on the canvas.
#[allow(clippy::type_complexity)]
pub fn cards(model: &Model) -> Vec<(Page, Option<Bounds>, Option<String>, Option<GroupTitle>)> {
    if shown_atlas_page(model).is_none() {
        return Vec::new();
    }
    let cards = card_entries(model);
    sidebar::sorted_index_pages(&model.sidebar.index_pages, sidebar::index_sort(model))
        .into_iter()
        .map(|p| {
            let card = cards.iter().find(|c| c.page_id == Some(p.id.as_str()));
            let bounds = card.and_then(|c| bounds_of(c.item));
            let group = card.and_then(|c| c.group);
            (
                p,
                bounds,
                group.map(|g| g.id.clone()),
                group.and_then(group_title),
            )
        })
        .collect()
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
