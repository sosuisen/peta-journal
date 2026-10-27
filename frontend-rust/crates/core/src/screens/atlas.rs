//! The canvas of an atlas page: the pages and folders of its directory as cards. A card lies
//! where the `atlas_bounds` property of its page says, in coordinates whose origin is the
//! center of the canvas. A page without the property waits in the list of unplaced cards.

use crate::app::{apply, done, run, Cmd, Model};
use crate::effects::UiOp;
use crate::model::Page;
use crate::screens::page::{split_doc_path, ATLAS_NAME};
use crate::screens::sidebar;
use crux_core::Command;
use picturesque_marks::{
    background_svg, signature_svg, BackgroundStyle, Color, ColorName, ComposeLayout, SignatureStyle,
    MAX_COMPOSED_TAGS,
};
use serde_json::Value;
use std::collections::BTreeMap;

/// The page property that holds where a card lies: `x,y,width,height`.
pub const BOUNDS_KEY: &str = "atlas_bounds";

/// The width of a card in the list of unplaced cards, which a drop from the list reads.
pub const CARD_WIDTH: f64 = 120.0;
/// The height of a card at scale 1, which it takes when it is first placed.
pub const CARD_HEIGHT: f64 = 40.0;
/// The stored width that lets the label set the width of a card. Any other width is the
/// width of the card. A card first placed takes this one.
pub const AUTO_WIDTH: f64 = 0.0;
/// The smallest a card can be scaled to. A larger card has no limit.
pub const MIN_CARD_SCALE: f64 = 0.5;

/// The place and size of a card. `x` and `y` are its center; `y` grows downward,
/// as on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    /// Reads a bounds value as the server writes it. A value that is not four numbers
    /// counts as no value, so the card stays in the list and can be placed again.
    pub fn parse(value: &str) -> Option<Bounds> {
        let numbers = value
            .split(',')
            .map(|n| n.trim().parse::<f64>().ok().filter(|n| n.is_finite()))
            .collect::<Option<Vec<f64>>>()?;
        match numbers[..] {
            [x, y, width, height] => Some(Bounds { x, y, width, height }),
            _ => None,
        }
    }

    pub fn of(page: &Page) -> Option<Bounds> {
        page.properties
            .get(BOUNDS_KEY)
            .and_then(|v| v.as_str())
            .and_then(Bounds::parse)
    }

    fn to_value(self) -> String {
        format!("{},{},{},{}", self.x, self.y, self.width, self.height)
    }

    /// How much the card is scaled, read from the height: the width may be left to the label.
    pub fn scale(&self) -> f64 {
        self.height / CARD_HEIGHT
    }
}

/// Places the card of a page with its center at the given point of the canvas. The shell
/// knows where the card was grabbed, so it gives the center rather than the drop point.
/// The card moves at once; a refusal of the server puts it back.
pub fn place_card(model: &mut Model, page_id: String, x: f64, y: f64) -> Cmd {
    let Some(page) = model
        .sidebar
        .index_pages
        .iter_mut()
        .find(|p| p.id == page_id)
    else {
        return done();
    };
    // A card keeps its size when it moves; whole pixels keep the stored value short
    let (width, height) = Bounds::of(page)
        .map(|b| (b.width, b.height))
        .unwrap_or((AUTO_WIDTH, CARD_HEIGHT));
    let bounds = Bounds {
        x: x.round(),
        y: y.round(),
        width,
        height,
    };
    save_bounds(page, bounds)
}

/// Scales the card of a page, no smaller than [`MIN_CARD_SCALE`]. The card keeps its place.
/// Its height is stored in whole pixels, so the scale moves in small steps. The width goes
/// back to the label: a set width would not fit the label at the new scale.
pub fn scale_card(model: &mut Model, page_id: String, scale: f64) -> Cmd {
    let Some(page) = index_page_mut(model, &page_id) else {
        return done();
    };
    let Some(before) = Bounds::of(page).filter(|_| scale.is_finite()) else {
        return done();
    };
    let scale = scale.max(MIN_CARD_SCALE);
    let bounds = Bounds {
        width: AUTO_WIDTH,
        height: (CARD_HEIGHT * scale).round(),
        ..before
    };
    if bounds == before {
        return done();
    }
    save_bounds(page, bounds)
}

/// Gives the page the bounds at once and saves them; a refusal of the server puts the old
/// ones back.
fn save_bounds(page: &mut Page, bounds: Bounds) -> Cmd {
    let page_id = page.id.clone();
    let before = page.properties.clone();
    page.properties
        .insert(BOUNDS_KEY.to_string(), bounds.to_value().into());
    run(move |api| async move {
        let result = api
            .upsert_page_property(&page_id, BOUNDS_KEY, "bounds", &bounds.to_value())
            .await;
        settle(&api, page_id, before, result);
    })
}

/// Takes the card of a page off the canvas: its bounds are removed, so it waits in the list
/// of unplaced cards again. The card leaves at once; a refusal of the server puts it back.
pub fn remove_card(model: &mut Model, page_id: String) -> Cmd {
    let Some(page) = index_page_mut(model, &page_id) else {
        return done();
    };
    let before = page.properties.clone();
    if page.properties.remove(BOUNDS_KEY).is_none() {
        return done();
    }
    run(move |api| async move {
        let result = api.delete_page_property(&page_id, BOUNDS_KEY).await;
        settle(&api, page_id, before, result);
    })
}

/// Takes the server's properties of the page, or puts the old ones back when it refused.
fn settle(
    api: &crate::effects::Api<crate::app::Event>,
    page_id: String,
    before: BTreeMap<String, Value>,
    result: Result<Page, crate::effects::ApiError>,
) {
    match result {
        Ok(updated) => apply(api, "atlas card saved", move |m| {
            if let Some(page) = index_page_mut(m, &updated.id) {
                page.properties = updated.properties;
            }
            done()
        }),
        Err(e) => {
            api.notify(UiOp::Alert(e.0));
            apply(api, "atlas card put back", move |m| {
                if let Some(page) = index_page_mut(m, &page_id) {
                    page.properties = before;
                }
                done()
            });
        }
    }
}

/// The start of the keys of the card groups, each a page property of the atlas page.
pub const GROUP_PREFIX: &str = "atlas_group_";

/// The card groups of an atlas page: the key of each group and the page ids in it.
pub fn groups(atlas: &Page) -> Vec<(String, Vec<String>)> {
    atlas
        .properties
        .iter()
        .filter(|(key, _)| key.starts_with(GROUP_PREFIX))
        .filter_map(|(key, value)| {
            let ids = value
                .as_array()?
                .iter()
                .filter_map(|id| id.as_str().map(str::to_string))
                .collect();
            Some((key.clone(), ids))
        })
        .collect()
}

/// Cards dropped on another card join its group. The target's group takes them, or a new
/// group is made for them and the target. A card belongs to one group only, so the cards
/// leave the groups they were in. A group left with one card is no group, and goes.
/// The groups change at once; a refusal of the server puts them back.
pub fn group_cards(model: &mut Model, page_ids: Vec<String>, target_id: String) -> Cmd {
    let Some(atlas) = super::page::page_mut(model)
        .and_then(|ps| ps.page.as_mut())
        .filter(|p| super::page::is_atlas(p))
    else {
        return done();
    };
    let moving: Vec<String> = page_ids.into_iter().filter(|id| *id != target_id).collect();
    if moving.is_empty() {
        return done();
    }
    let groups = groups(atlas);
    let target_key = groups
        .iter()
        .find(|(_, ids)| ids.contains(&target_id))
        .map(|(key, _)| key.clone())
        .unwrap_or_else(|| new_group_key(&groups, &target_id, &moving));
    let mut changed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (key, ids) in &groups {
        if *key == target_key {
            continue;
        }
        let kept: Vec<String> = ids.iter().filter(|id| !moving.contains(id)).cloned().collect();
        if kept.len() != ids.len() {
            changed.insert(key.clone(), if kept.len() < 2 { Vec::new() } else { kept });
        }
    }
    let mut joined = groups
        .iter()
        .find(|(key, _)| *key == target_key)
        .map(|(_, ids)| ids.clone())
        .unwrap_or_else(|| vec![target_id.clone()]);
    for id in moving {
        if !joined.contains(&id) {
            joined.push(id);
        }
    }
    changed.insert(target_key, joined);
    change_groups(atlas, changed)
}

/// Cards taken out of their groups, as when they are dragged out of the outline of their
/// group with Ctrl held. A group left with one card is no group, and goes. The groups
/// change at once; a refusal of the server puts them back.
pub fn ungroup_cards(model: &mut Model, page_ids: Vec<String>) -> Cmd {
    let Some(atlas) = super::page::page_mut(model)
        .and_then(|ps| ps.page.as_mut())
        .filter(|p| super::page::is_atlas(p))
    else {
        return done();
    };
    let mut changed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (key, ids) in groups(atlas) {
        let kept: Vec<String> = ids.iter().filter(|id| !page_ids.contains(id)).cloned().collect();
        if kept.len() != ids.len() {
            changed.insert(key, if kept.len() < 2 { Vec::new() } else { kept });
        }
    }
    if changed.is_empty() {
        return done();
    }
    change_groups(atlas, changed)
}

/// Gives the atlas page the changed groups at once and saves them. An empty list stands for
/// a group that goes, and its title goes with it.
fn change_groups(atlas: &mut Page, changed: BTreeMap<String, Vec<String>>) -> Cmd {
    let mut changes = BTreeMap::new();
    for (key, ids) in changed {
        if ids.is_empty() {
            if let Some(title) = title_key(&key).filter(|t| atlas.properties.contains_key(t)) {
                changes.insert(title, None);
            }
            changes.insert(key, None);
        } else {
            changes.insert(key, Some(Value::from(ids)));
        }
    }
    change_atlas(atlas, changes)
}

/// Gives the atlas page the changed properties at once and saves them one by one: a list as
/// a list, text as text, and none as a property that goes. A refusal of the server puts them
/// all back.
fn change_atlas(atlas: &mut Page, changes: BTreeMap<String, Option<Value>>) -> Cmd {
    let before = atlas.properties.clone();
    for (key, value) in &changes {
        match value {
            Some(value) => atlas.properties.insert(key.clone(), value.clone()),
            None => atlas.properties.remove(key),
        };
    }
    let atlas_id = atlas.id.clone();
    run(move |api| async move {
        let mut last = None;
        for (key, value) in changes {
            let result = match value {
                None => api.delete_page_property(&atlas_id, &key).await,
                Some(Value::Array(ids)) => {
                    let ids: Vec<&str> = ids.iter().filter_map(Value::as_str).collect();
                    api.upsert_page_property(&atlas_id, &key, "list", &ids.join(",")).await
                }
                Some(value) => {
                    let text = value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string());
                    api.upsert_page_property(&atlas_id, &key, "text", &text).await
                }
            };
            match result {
                Ok(page) => last = Some(page),
                Err(e) => {
                    api.notify(UiOp::Alert(e.0));
                    apply(&api, "atlas groups put back", move |m| {
                        if let Some(atlas) = shown_atlas_mut(m, &atlas_id) {
                            atlas.properties = before;
                        }
                        done()
                    });
                    return;
                }
            }
        }
        if let Some(updated) = last {
            apply(&api, "atlas groups saved", move |m| {
                if let Some(atlas) = shown_atlas_mut(m, &updated.id) {
                    atlas.properties = updated.properties;
                }
                done()
            });
        }
    })
}

/// The start of the keys of the group titles: the key of a group with this prefix instead.
pub const TITLE_PREFIX: &str = "atlas_title_";
/// The sides of the outline of a group that its title can lie along, the first by default.
pub const TITLE_SIDES: [&str; 4] = ["top", "bottom", "left", "right"];

/// The title of a group and the side of its outline it lies along. Stored as `side:text`.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupTitle {
    pub side: String,
    pub text: String,
}

fn title_key(group_key: &str) -> Option<String> {
    group_key
        .strip_prefix(GROUP_PREFIX)
        .map(|key| format!("{TITLE_PREFIX}{key}"))
}

/// The title of a group of the atlas page. A value without a known side lies along the top,
/// all of it as the text, so a title written by hand is not lost.
pub fn group_title(atlas: &Page, group_key: &str) -> Option<GroupTitle> {
    let value = atlas.properties.get(&title_key(group_key)?)?.as_str()?;
    let (side, text) = match value.split_once(':') {
        Some((side, text)) if TITLE_SIDES.contains(&side) => (side, text),
        _ => (TITLE_SIDES[0], value),
    };
    Some(GroupTitle { side: side.to_string(), text: text.to_string() })
        .filter(|t| !t.text.trim().is_empty())
}

/// Gives a group of the shown atlas page a title along a side of its outline. An empty title
/// takes the title away. The title changes at once; a refusal of the server puts it back.
pub fn set_group_title(model: &mut Model, group_key: String, side: String, title: String) -> Cmd {
    let Some(atlas) = super::page::page_mut(model)
        .and_then(|ps| ps.page.as_mut())
        .filter(|p| super::page::is_atlas(p))
    else {
        return done();
    };
    let is_group = groups(atlas).iter().any(|(key, _)| *key == group_key);
    let Some(key) = title_key(&group_key).filter(|_| is_group) else {
        return done();
    };
    let side = if TITLE_SIDES.contains(&side.as_str()) { side } else { TITLE_SIDES[0].to_string() };
    let title = title.trim();
    let value = (!title.is_empty()).then(|| Value::from(format!("{side}:{title}")));
    if atlas.properties.get(&key) == value.as_ref() {
        return done();
    }
    change_atlas(atlas, BTreeMap::from([(key, value)]))
}

/// The start of the keys of the callouts of an atlas page.
pub const CALLOUT_PREFIX: &str = "atlas_callout_";

/// A callout on the canvas of an atlas page: a character at a point, with a speech bubble on
/// one side of it. Stored under a key of its own as `x,y,scale,side:text`. A value without the
/// scale, as the first callouts were stored, has a scale of 1.
#[derive(Debug, Clone, PartialEq)]
pub struct Callout {
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

/// The callouts of the atlas page, in the order of their keys. A value that is not
/// `x,y,scale,side:text` or `x,y,side:text` is left out.
pub fn callouts(atlas: &Page) -> Vec<Callout> {
    atlas
        .properties
        .iter()
        .filter(|(key, _)| key.len() > CALLOUT_PREFIX.len() && key.starts_with(CALLOUT_PREFIX))
        .filter_map(|(key, value)| {
            let (head, text) = value.as_str()?.split_once(':')?;
            let parts: Vec<&str> = head.split(',').map(str::trim).collect();
            let (x, y, scale, side) = match parts[..] {
                [x, y, side] => (x, y, "1", side),
                [x, y, scale, side] => (x, y, scale, side),
                _ => return None,
            };
            let x: f64 = x.parse().ok()?;
            let y: f64 = y.parse().ok()?;
            let scale: f64 = scale.parse().ok().filter(|s: &f64| s.is_finite() && *s > 0.0)?;
            (TITLE_SIDES.contains(&side) && !text.trim().is_empty()).then(|| Callout {
                key: key.clone(),
                x,
                y,
                scale: scale.max(MIN_CARD_SCALE),
                side: side.to_string(),
                text: text.to_string(),
            })
        })
        .collect()
}

/// The stored value of a callout. The place in whole units and the scale to a hundredth, so a
/// small move of the pointer does not show as a change.
fn callout_value(x: f64, y: f64, scale: f64, side: &str, text: &str) -> Value {
    let side = if TITLE_SIDES.contains(&side) { side } else { TITLE_SIDES[0] };
    let scale = (scale.max(MIN_CARD_SCALE) * 100.0).round() / 100.0;
    Value::from(format!("{},{},{scale},{side}:{text}", x.round(), y.round()))
}

/// Saves a callout of the shown atlas page as changed by `change`, unless its value stays
/// the same. It changes at once; a refusal of the server puts it back.
fn change_callout(model: &mut Model, key: &str, change: impl FnOnce(&mut Callout)) -> Cmd {
    let Some(atlas) = shown_atlas(model) else {
        return done();
    };
    let Some(mut callout) = callouts(atlas).into_iter().find(|c| c.key == key) else {
        return done();
    };
    change(&mut callout);
    let value = callout_value(callout.x, callout.y, callout.scale, &callout.side, &callout.text);
    if atlas.properties.get(key) == Some(&value) {
        return done();
    }
    change_atlas(atlas, BTreeMap::from([(key.to_string(), Some(value))]))
}

/// Moves a callout of the shown atlas page so the center of its character is at the point.
pub fn move_callout(model: &mut Model, key: String, x: f64, y: f64) -> Cmd {
    if !x.is_finite() || !y.is_finite() {
        return done();
    }
    change_callout(model, &key, |c| {
        c.x = x;
        c.y = y;
    })
}

/// Scales a callout of the shown atlas page around the center of its character, no smaller
/// than a card can be.
pub fn scale_callout(model: &mut Model, key: String, scale: f64) -> Cmd {
    if !scale.is_finite() {
        return done();
    }
    change_callout(model, &key, |c| c.scale = scale)
}

fn shown_atlas(model: &mut Model) -> Option<&mut Page> {
    super::page::page_mut(model)
        .and_then(|ps| ps.page.as_mut())
        .filter(|p| super::page::is_atlas(p))
}

/// Puts a new callout on the shown atlas page with the center of its character at the
/// point. An empty text puts nothing.
pub fn add_callout(model: &mut Model, x: f64, y: f64, side: String, text: String) -> Cmd {
    let text = text.trim();
    let Some(atlas) = shown_atlas(model).filter(|_| !text.is_empty()) else {
        return done();
    };
    // A hash of what the callout is, so no counter has to be kept. A key already in use is
    // hashed again.
    let mut seed = format!("{x},{y},{text}");
    let key = loop {
        let key = format!("{CALLOUT_PREFIX}{:08x}", fnv1a(&seed) as u32);
        if !atlas.properties.contains_key(&key) {
            break key;
        }
        seed.push('+');
    };
    change_atlas(atlas, BTreeMap::from([(key, Some(callout_value(x, y, 1.0, &side, text)))]))
}

/// Changes the text and the side of a callout of the shown atlas page. It keeps its place and
/// its scale. An empty text takes the callout away.
pub fn set_callout(model: &mut Model, key: String, side: String, text: String) -> Cmd {
    let text = text.trim();
    if !text.is_empty() {
        return change_callout(model, &key, |c| {
            c.side = side;
            c.text = text.to_string();
        });
    }
    let Some(atlas) = shown_atlas(model).filter(|a| callouts(a).iter().any(|c| c.key == key)) else {
        return done();
    };
    change_atlas(atlas, BTreeMap::from([(key, None)]))
}

/// The color of the character of a callout
const CALLOUT_COLOR: Color = Color::rgb(0x00, 0x00, 0x00);

/// The width of the lines of the character of a callout, in pixels of the SVG as drawn
const CALLOUT_STROKE_WIDTH: &str = "3.5";

/// The character of a callout as an SVG of `size` pixels square: the face picturesque-marks
/// draws for the text as its signature, in black, its lines thinner.
pub fn callout_face_svg(text: &str, size: i32) -> String {
    // picturesque-marks has no option for the width of the lines of a face; it writes 5
    signature_svg(text, SignatureStyle::Face, CALLOUT_COLOR, size, size)
        .replace(r#"stroke-width="5""#, &format!(r#"stroke-width="{CALLOUT_STROKE_WIDTH}""#))
}

/// A key for a new group: a hash of the cards that make it, so no counter has to be kept.
/// A key already in use is hashed again.
fn new_group_key(groups: &[(String, Vec<String>)], target_id: &str, moving: &[String]) -> String {
    let mut seed = format!("{target_id},{}", moving.join(","));
    loop {
        let key = format!("{GROUP_PREFIX}{:08x}", fnv1a(&seed) as u32);
        if groups.iter().all(|(k, _)| *k != key) {
            return key;
        }
        seed.push('+');
    }
}

/// The color of a group as `#RRGGBB`, one of the six of picturesque-marks, the colors of the
/// tag icons. The key ends in the hash it was made from, which picks the color, so a group
/// keeps its color on every machine. A key that does not end in one is hashed here.
pub fn group_color(key: &str) -> String {
    let hash = key
        .strip_prefix(GROUP_PREFIX)
        .and_then(|hash| u32::from_str_radix(hash, 16).ok())
        .unwrap_or_else(|| fnv1a(key) as u32);
    ColorName::from_index(hash).color().to_hex()
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
    let tags: Vec<String> = match page.properties.get("tags") {
        Some(Value::Array(tags)) => tags.iter().filter_map(|t| t.as_str().map(str::to_string)).collect(),
        _ => Vec::new(),
    };
    match path.rsplit('/').next() {
        Some(name) if is_dir && tags.is_empty() && !name.is_empty() => vec![name.to_string()],
        _ => tags,
    }
}

/// The icon of the tags as an SVG of `size` pixels square, as picturesque-marks draws it for
/// a tag. Only the first tags fit in one icon.
pub fn tag_icon_svg(tags: &[String], size: i32) -> String {
    let tags = &tags[..tags.len().min(MAX_COMPOSED_TAGS)];
    background_svg(tags, ComposeLayout::default(), BackgroundStyle::default(), size, size)
}

/// The light blue of the blob behind the icon of a card.
const BLOB_COLOR: Color = Color::rgb(0xCF, 0xE3, 0xFF);

/// The blob of a tag as an SVG of `size` pixels square: the outline picturesque-marks draws
/// for the tag name, filled in light blue. The blob fills the whole SVG, which the card
/// stretches to its own size.
pub fn tag_blob_svg(tag: &str, size: i32) -> String {
    let svg = signature_svg(tag, SignatureStyle::Blob, BLOB_COLOR, size, size);
    // picturesque-marks leaves a margin around the blob, which would keep it off the edges of
    // the card. The view box is cut to the points of its path instead. The control points of a
    // curve lie a little outside it, so a thin margin stays.
    let Some((min_x, min_y, max_x, max_y)) = path_bounds(&svg) else {
        return svg;
    };
    let Some(start) = svg.find("viewBox=\"") else {
        return svg;
    };
    let Some(end) = svg[start + 9..].find('"').map(|i| start + 9 + i) else {
        return svg;
    };
    format!(
        r#"{}viewBox="{} {} {} {}" preserveAspectRatio="none"{}"#,
        &svg[..start],
        min_x,
        min_y,
        max_x - min_x,
        max_y - min_y,
        &svg[end + 1..]
    )
}

/// The smallest box around the points of the first path in the SVG.
fn path_bounds(svg: &str) -> Option<(f64, f64, f64, f64)> {
    let start = svg.find(" d=\"")? + 4;
    let end = start + svg[start..].find('"')?;
    let numbers: Vec<f64> = svg[start..end]
        .split(|c: char| c.is_ascii_whitespace() || c == ',' || c.is_ascii_alphabetic())
        .filter_map(|n| n.parse().ok())
        .collect();
    let (xs, ys): (Vec<f64>, Vec<f64>) = numbers.chunks_exact(2).map(|p| (p[0], p[1])).unzip();
    let min = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (!xs.is_empty()).then(|| (min(&xs), min(&ys), max(&xs), max(&ys)))
}

fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Takes the card groups of the shown atlas page from the server again, after its file
/// changed on disk. The groups are properties of the page itself, which a reload of its
/// items leaves as they were.
pub fn reload_groups(model: &Model, page_id: &str) -> Cmd {
    let shown = super::page::page_state(model)
        .and_then(|ps| ps.page.as_ref())
        .is_some_and(|p| p.id == page_id && super::page::is_atlas(p));
    if !shown {
        return done();
    }
    let page_id = page_id.to_string();
    run(move |api| async move {
        match api.get_page_by_id(&page_id).await {
            Ok(result) => apply(&api, "atlas groups reloaded", move |m| {
                if let Some(atlas) = shown_atlas_mut(m, &result.page.id) {
                    atlas.properties = result.page.properties;
                }
                done()
            }),
            Err(e) => crate::app::log_error(format!("Failed to reload atlas {page_id}: {e}")),
        }
    })
}

fn shown_atlas_mut<'a>(model: &'a mut Model, page_id: &str) -> Option<&'a mut Page> {
    super::page::page_mut(model)
        .and_then(|ps| ps.page.as_mut())
        .filter(|p| p.id == page_id)
}

fn index_page_mut<'a>(model: &'a mut Model, page_id: &str) -> Option<&'a mut Page> {
    model.sidebar.index_pages.iter_mut().find(|p| p.id == page_id)
}

/// The callouts of the shown atlas page; none on any other page.
pub fn shown_callouts(model: &Model) -> Vec<Callout> {
    super::page::page_state(model)
        .and_then(|ps| ps.page.as_ref())
        .filter(|p| super::page::is_atlas(p))
        .map(callouts)
        .unwrap_or_default()
}

/// The cards of the shown atlas page, in the order of the sidebar's index list, each with
/// its place, the key of its group and the title of that group.
#[allow(clippy::type_complexity)]
pub fn cards(model: &Model) -> Vec<(Page, Option<Bounds>, Option<String>, Option<GroupTitle>)> {
    let Some(atlas) = super::page::page_state(model)
        .and_then(|ps| ps.page.as_ref())
        .filter(|p| super::page::is_atlas(p))
    else {
        return Vec::new();
    };
    let groups = groups(atlas);
    sidebar::sorted_index_pages(&model.sidebar.index_pages, sidebar::index_sort(model))
        .into_iter()
        .map(|p| {
            let bounds = Bounds::of(&p);
            let group = groups
                .iter()
                .find(|(_, ids)| ids.contains(&p.id))
                .map(|(key, _)| key.clone());
            let title = group.as_deref().and_then(|key| group_title(atlas, key));
            (p, bounds, group, title)
        })
        .collect()
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
/// The name of the directory above a directory, as the button that goes up shows it: its path
/// with a slash at the end, or `(root)/` for the root, as in the title of an atlas.
pub fn parent_dir_label(dir_path: &str) -> String {
    match dir_path.trim_end_matches('/').rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/"),
        None => "(root)/".to_string(),
    }
}

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
