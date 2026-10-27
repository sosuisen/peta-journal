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

/// Gives the atlas page the changed groups at once and saves them one by one. An empty
/// list stands for a group that goes.
fn change_groups(atlas: &mut Page, changed: BTreeMap<String, Vec<String>>) -> Cmd {
    let before = atlas.properties.clone();
    for (key, ids) in &changed {
        if ids.is_empty() {
            atlas.properties.remove(key);
        } else {
            atlas.properties.insert(key.clone(), Value::from(ids.clone()));
        }
    }
    let atlas_id = atlas.id.clone();
    run(move |api| async move {
        let mut last = None;
        for (key, ids) in changed {
            let result = if ids.is_empty() {
                api.delete_page_property(&atlas_id, &key).await
            } else {
                api.upsert_page_property(&atlas_id, &key, "list", &ids.join(",")).await
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

/// The cards of the shown atlas page, in the order of the sidebar's index list, each with
/// its place and the key of its group.
pub fn cards(model: &Model) -> Vec<(Page, Option<Bounds>, Option<String>)> {
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
            (p, bounds, group)
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
