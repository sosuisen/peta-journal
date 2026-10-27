//! The canvas of an atlas page: the pages and folders of its directory as cards. A card lies
//! where the `atlas_bounds` property of its page says, in coordinates whose origin is the
//! center of the canvas. A page without the property waits in the list of unplaced cards.

use crate::app::{apply, done, run, Cmd, Model};
use crate::effects::UiOp;
use crate::model::Page;
use crate::screens::page::{split_doc_path, ATLAS_NAME};
use crate::screens::sidebar;
use crux_core::Command;
use serde_json::Value;
use std::collections::BTreeMap;

/// The page property that holds where a card lies: `x,y,width,height`.
pub const BOUNDS_KEY: &str = "atlas_bounds";

/// The size a card takes when it is first placed.
pub const CARD_WIDTH: f64 = 120.0;
pub const CARD_HEIGHT: f64 = 40.0;

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
        .unwrap_or((CARD_WIDTH, CARD_HEIGHT));
    let bounds = Bounds {
        x: x.round(),
        y: y.round(),
        width,
        height,
    };
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
    let before = atlas.properties.clone();
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
    // An empty list stands for a group that goes
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

fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3)
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
