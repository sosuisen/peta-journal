//! The doc page screen, ported from `page.ts`: loading a page by its path, the title and
//! directory editing, and demoting an index page.

use crate::app::{done, log_error, run, Cmd, Event, Model, PageState, Screen};
use crate::effects::{Api, ApiError, PageUpdate, UiOp};
use crate::model::PageExt;
use crate::model::{list_type_of, BacklinkEntry, Page, PageWithItems, PropertyValue};
use crate::screens::{navigate, sidebar};
use crate::util::filename::validate_file_name;
use crate::view::{DirSegment, LoadState};
use crux_core::Command;
use std::collections::BTreeMap;

/// The reserved file name of the atlas page of a directory.
pub use peta_atlas::ATLAS_NAME;

/// Splits a path like `cat/01.md` into `("cat/", "01")`, as `ApiService.getOrCreateDoc` did.
pub fn split_doc_path(title: &str) -> (String, String) {
    let stripped = title.strip_suffix(".md").unwrap_or(title);
    match stripped.rfind('/') {
        Some(slash) => (
            stripped[..=slash].to_string(),
            stripped[slash + 1..].to_string(),
        ),
        None => (String::new(), stripped.to_string()),
    }
}

pub fn page_state(model: &Model) -> Option<&PageState> {
    match &model.screen {
        Screen::Page(p) => Some(p),
        _ => None,
    }
}

pub(crate) fn page_mut(model: &mut Model) -> Option<&mut PageState> {
    match &mut model.screen {
        Screen::Page(p) => Some(p),
        _ => None,
    }
}

/// The doc page on screen, once it is loaded.
pub fn shown_doc_page(model: &Model) -> Option<&Page> {
    page_state(model).and_then(|ps| ps.page.as_ref())
}

fn shown_doc_page_mut(model: &mut Model) -> Option<&mut Page> {
    page_mut(model).and_then(|ps| ps.page.as_mut())
}

/// Shows a doc page by its path; the fragment is the item to scroll to.
pub fn open(model: &mut Model, name: &str, fragment: Option<String>) -> Cmd {
    let remember = crate::screens::atlas::remember_folder_view(model, name);
    model.screen = Screen::Page(PageState {
        page_name: name.to_string(),
        focus_item_id: fragment.filter(|f| !f.is_empty()),
        page: None,
        can_demote_index: false,
        properties_open: false,
        backlinks: Vec::new(),
    });
    // The headings of the page left behind are wrong for this one
    model.headings_state = LoadState::Loading;
    let page_name = name.to_string();
    let load = run(move |api| async move {
        let (dir_path, title) = split_doc_path(&page_name);
        let result = match api.get_or_create_doc(&title, &dir_path).await {
            Ok(r) => r,
            Err(e) => {
                log_error(format!("Failed to load page {page_name}: {e}"));
                return api.send_event(Event::PageFailed { page_name });
            }
        };
        let p = result.page.clone();
        // If the URL does not match the canonical path, redirect, e.g. the server resolved
        // "Quarkus" to dirPath "Quarkus/" and name "index"
        let canonical = p.canonical_path();
        if canonical != page_name {
            api.send_event(Event::PageRedirected { path: canonical });
            return;
        }
        api.send_event(Event::PageLoaded {
            page: p.clone(),
            created: result.created,
        });
        // A non-root index page can be demoted when it has no children
        if p.name == "index" && !p.dir_path.is_empty() {
            // The atlas page does not stop the demotion: it is deleted on demoting
            let can_demote = api
                .get_child_pages(&p.id)
                .await
                .map(|c| c.iter().all(is_atlas))
                .unwrap_or(false);
            api.send_event(Event::CanDemoteLoaded {
                page_id: p.id.clone(),
                can_demote,
            });
        }
        // The outline loads the items of the page
        match api.get_page_by_id(&p.id).await {
            Ok(loaded) => api.send_event(Event::PageItemsLoaded {
                page_id: p.id,
                loaded: Box::new(loaded),
            }),
            Err(e) => {
                log_error(format!("Failed to load items for page {}: {e}", p.id));
                api.send_event(Event::PageFailed { page_name });
            }
        }
    });
    Command::all([remember, load])
}

/// The server answered the page of the path the screen opened: the screen shows it. An
/// answer for a path the screen has left is not shown.
pub fn show_page(model: &mut Model, page: Page, created: bool) -> Cmd {
    let Some(ps) = page_mut(model) else {
        return done();
    };
    if ps.page_name != page.canonical_path() && ps.page_name != page.name {
        return done();
    }
    ps.page = Some(page.clone());
    let mut cmds = vec![
        navigate::set_title(model, full_page_title(&page)),
        sidebar::set_current_page(model, Some(page.clone())),
        load_backlinks(page.id),
    ];
    if created {
        cmds.push(sidebar::refresh_recent_pages());
    }
    Command::all(cmds)
}

/// The server answered whether an index page can be demoted. An answer about a page the
/// screen has left is not shown.
pub fn show_can_demote(model: &mut Model, page_id: &str, can_demote: bool) -> Cmd {
    if shown_doc_page(model).map(|pg| pg.id.as_str()) != Some(page_id) {
        return done();
    }
    if let Some(ps) = page_mut(model) {
        ps.can_demote_index = can_demote;
    }
    done()
}

/// The server answered the items of a page: the outline shows them while the page is
/// still the shown one.
pub fn show_page_items(model: &mut Model, page_id: &str, loaded: PageWithItems) -> Cmd {
    if shown_doc_page(model).map(|pg| pg.id.as_str()) != Some(page_id) {
        return done();
    }
    model.state.set_items(loaded.items);
    model.state.set_transcluded_trees(loaded.transcluded_trees);
    model.headings_state = LoadState::Loaded;
    let focus_item_id = page_state(model).and_then(|p| p.focus_item_id.clone());
    scroll_to_item(model, focus_item_id)
}

/// The server could not answer the page of a path, or its items. The headings of the page
/// still on screen fail; a reload of a page already shown keeps its headings.
pub fn page_failed(model: &mut Model, page_name: &str) -> Cmd {
    let shown = page_state(model).is_some_and(|p| p.page_name == page_name);
    if shown && model.headings_state == LoadState::Loading {
        model.headings_state = LoadState::Failed;
    }
    done()
}

/// Loads the backlinks of the shown doc page again: any page may have changed its links.
pub fn reload_backlinks(model: &Model) -> Cmd {
    match shown_doc_page(model) {
        Some(p) => load_backlinks(p.id.clone()),
        None => done(),
    }
}

fn load_backlinks(page_id: String) -> Cmd {
    run(move |api| async move {
        match api.get_backlinks(&page_id).await {
            Ok(entries) => api.send_event(Event::BacklinksLoaded { page_id, entries }),
            Err(e) => log_error(format!("Failed to load backlinks of page {page_id}: {e}")),
        }
    })
}

/// The server answered the backlinks of a page: they are shown while the page is still
/// the shown one.
pub fn show_backlinks(model: &mut Model, page_id: &str, entries: Vec<BacklinkEntry>) -> Cmd {
    if let Some(ps) = page_mut(model) {
        if ps.page.as_ref().map(|p| p.id.as_str()) == Some(page_id) {
            ps.backlinks = entries;
        }
    }
    done()
}

/// The backlinks grouped by source page, in the server's order: the page, then the items
/// in it that link. A page that links by its tags only has no items.
pub fn backlink_groups(entries: &[BacklinkEntry]) -> Vec<(Page, Vec<crate::model::Item>)> {
    let mut groups: Vec<(Page, Vec<crate::model::Item>)> = Vec::new();
    for e in entries {
        if groups
            .last()
            .map(|(p, _)| p.id != e.source_page.id)
            .unwrap_or(true)
        {
            groups.push((e.source_page.clone(), Vec::new()));
        }
        if let (Some(item), Some(group)) = (&e.source_item, groups.last_mut()) {
            group.1.push(item.clone());
        }
    }
    groups
}

/// Selects and scrolls to the item the URL fragment names, when the outline has it.
pub fn scroll_to_item(model: &mut Model, item_id: Option<String>) -> Cmd {
    let Some(item_id) = item_id else {
        return done();
    };
    if model.state.get_visible_item(&item_id).is_none() {
        return done();
    }
    model.selection.select_subtree(&mut model.state, &item_id);
    Command::notify_shell(UiOp::ScrollToItem {
        item_id,
        smooth: false,
    })
    .into()
}

/// True when the page is the atlas page of its directory.
pub fn is_atlas(p: &Page) -> bool {
    p.page_type == "doc" && p.name == ATLAS_NAME
}

/// The fixed title of an atlas page: `Atlas of a/b/`.
pub fn atlas_title(p: &Page) -> String {
    let dir = if p.dir_path.is_empty() {
        "(root)/"
    } else {
        p.dir_path.as_str()
    };
    format!("Atlas of {dir}")
}

/// Opens the atlas page of the directory of the shown index page, or the index page of the
/// shown atlas page. The atlas page is created when it is opened for the first time.
pub fn toggle_atlas(model: &mut Model) -> Cmd {
    let Some(p) = shown_doc_page(model) else {
        return done();
    };
    let target = if p.name == "index" {
        ATLAS_NAME
    } else if is_atlas(p) {
        "index"
    } else {
        return done();
    };
    navigate::open_page(&format!("{}{target}", p.dir_path))
}

/// Opens the atlas page of the directory above the shown atlas page. The map stays a map
/// on the way up. The root atlas has no directory above.
pub fn atlas_up(model: &mut Model) -> Cmd {
    let Some(p) = shown_doc_page(model) else {
        return done();
    };
    let dir = p.dir_path.trim_end_matches('/');
    if !is_atlas(p) || dir.is_empty() {
        return done();
    }
    let parent = match dir.rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/"),
        None => String::new(),
    };
    navigate::open_page(&format!("{parent}{ATLAS_NAME}"))
}

/// The display title: journals as dates, index pages as their directory, docs as their name.
pub fn page_title(p: &Page) -> String {
    if p.page_type == "journal" {
        return crate::util::date::format_journal_date(&p.name);
    }
    if is_atlas(p) {
        return atlas_title(p);
    }
    if p.name == "index" && !p.dir_path.is_empty() {
        let last = p
            .dir_path
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("");
        return format!("{last}/");
    }
    p.name.clone()
}

/// Full dir path + name for the browser title. Index pages show `dir_path/` only.
pub fn full_page_title(p: &Page) -> String {
    if p.page_type == "journal" {
        return crate::util::date::format_journal_date(&p.name);
    }
    if is_atlas(p) {
        return atlas_title(p);
    }
    if p.name == "index" {
        return if p.dir_path.is_empty() {
            "(root)/".to_string()
        } else {
            p.dir_path.clone()
        };
    }
    format!("{}{}", p.dir_path, p.name)
}

/// The breadcrumb segments of a dir path: `a` with the path `a`, `b` with `a/b`, ...
pub fn dir_path_segments(dir_path: &str) -> Vec<DirSegment> {
    let trimmed = dir_path.trim_end_matches('/');
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut path = String::new();
    trimmed
        .split('/')
        .map(|part| {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
            DirSegment {
                name: part.to_string(),
                path: path.clone(),
                index_path: format!("{path}/index"),
            }
        })
        .collect()
}

/// The name a demoted index page gets: the last segment of its directory.
pub fn demote_name(p: &Page) -> String {
    let trimmed = p.dir_path.trim_end_matches('/');
    trimmed.rsplit('/').next().unwrap_or(trimmed).to_string()
}

/// Saves the edited page title and/or directory path to the server.
pub fn save_title(model: &mut Model, dir_path: String, file_name: String) -> Cmd {
    let Some(p) = shown_doc_page(model).cloned() else {
        return done();
    };
    let raw = file_name.trim().to_string();
    if raw.is_empty() {
        return Command::notify_shell(UiOp::Alert("Page name cannot be empty".into())).into();
    }
    if let Some(error) = validate_file_name(&raw) {
        return Command::notify_shell(UiOp::Alert(error)).into();
    }
    let mut new_dir = dir_path.trim().to_string();
    if !new_dir.is_empty() && !new_dir.ends_with('/') {
        new_dir.push('/');
    }
    // Split the title by '/': the last segment becomes the name, the rest joins the dir path
    let mut new_name = raw.clone();
    if raw.contains('/') {
        let mut segments: Vec<&str> = raw.split('/').collect();
        new_name = segments.pop().unwrap_or("").to_string();
        new_dir = format!("{new_dir}{}/", segments.join("/"));
    }
    let title_changed = new_name != p.name;
    let dir_changed = new_dir != p.dir_path;
    if !title_changed && !dir_changed {
        return done();
    }
    run(move |api| async move {
        let Some(updated) = update_page(
            &api,
            &p.id,
            title_changed.then(|| new_name.clone()),
            dir_changed.then(|| new_dir.clone()),
            &format!("{new_dir}{new_name}"),
        )
        .await
        else {
            return;
        };
        api.send_event(Event::TitleSaved { page: updated });
    })
}

/// The server saved the new title or folder of the shown page: the screen and the URL
/// show it.
pub fn show_title_saved(model: &mut Model, updated: Page) -> Cmd {
    let canonical = updated.canonical_path();
    let title = full_page_title(&updated);
    if let Some(ps) = page_mut(model) {
        ps.page = Some(updated);
        ps.page_name = canonical.clone();
    }
    Command::all([
        navigate::set_title(model, title),
        sidebar::refresh_recent_pages(),
        navigate::replace_page(&canonical),
    ])
}

/// Shows the property rows of the doc page, so a first property can be added.
pub fn edit_properties(model: &mut Model) -> Cmd {
    if let Some(ps) = page_mut(model) {
        ps.properties_open = true;
    }
    done()
}

fn is_list_type(value_type: &str) -> bool {
    value_type == "tags" || value_type == "list"
}

/// The items of a comma separated list, each once. A `#` typed out of habit from hashtags
/// is not part of a tag, so `#abc` is kept as `abc`.
fn list_items(value: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for t in value
        .split(',')
        .map(|t| t.trim().trim_start_matches('#').trim())
        .filter(|t| !t.is_empty())
    {
        if !items.iter().any(|x| x == t) {
            items.push(t.to_string());
        }
    }
    items
}

/// The shown value of a property as the server would store it: `tags` is a list.
fn property_value(value_type: &str, value: &str) -> PropertyValue {
    if is_list_type(value_type) {
        PropertyValue::List(list_items(value))
    } else {
        PropertyValue::Text(value.to_string())
    }
}

/// Sets a property of the doc page. The screen changes at once; the server's answer (which
/// formats numbers) replaces it, and a refusal puts the old properties back. The page's
/// dates (`created`, `modified`) only change when the server has accepted them.
pub fn set_property(model: &mut Model, key: String, value_type: String, value: String) -> Cmd {
    let Some(page) = shown_doc_page_mut(model) else {
        return done();
    };
    // The server gets the same list as the screen shows
    let value = if is_list_type(&value_type) {
        list_items(&value).join(", ")
    } else {
        value
    };
    let before = page.properties.clone();
    if key != "created" && key != "modified" {
        page.properties
            .insert(key.clone(), property_value(&value_type, &value));
    }
    let page_id = page.id.clone();
    run(move |api| async move {
        let result = api
            .upsert_page_property(&page_id, &key, &value_type, &value)
            .await;
        settle_property_change(&api, page_id, before, result).await;
    })
}

/// Removes a property of the doc page, the same way `set_property` sets one.
/// The tags of the list property `key` of the shown doc page; none when it has no such list.
fn tags_of(model: &Model, key: &str) -> Vec<String> {
    match shown_doc_page(model).and_then(|page| page.properties.get(key)) {
        Some(PropertyValue::List(tags)) => tags.clone(),
        _ => Vec::new(),
    }
}

/// Saves the tags as the list property `key`. An empty list would stay as a row with no tags,
/// so the property goes instead.
fn save_tags(model: &mut Model, key: String, tags: &[String]) -> Cmd {
    if tags.is_empty() {
        return delete_property(model, key);
    }
    let value_type = list_type_of(&key).to_string();
    set_property(model, key, value_type, tags.join(", "))
}

/// Takes one tag out of the list property `key`.
pub fn remove_tag(model: &mut Model, key: String, tag: &str) -> Cmd {
    let rest: Vec<String> = tags_of(model, &key)
        .into_iter()
        .filter(|t| t != tag)
        .collect();
    save_tags(model, key, &rest)
}

/// Moves `tag` of the list property `key` to before `target`, or after it.
pub fn move_tag(model: &mut Model, key: String, tag: &str, target: &str, after: bool) -> Cmd {
    let old = tags_of(model, &key);
    let mut new: Vec<String> = old.iter().filter(|t| *t != tag).cloned().collect();
    let Some(at) = new.iter().position(|t| t == target) else {
        return done();
    };
    new.insert(if after { at + 1 } else { at }, tag.to_string());
    // A drop back where the tag was would build the page again for nothing
    if new == old {
        return done();
    }
    save_tags(model, key, &new)
}

/// Adds the comma separated `typed` tags to the list property `key`. `set_property` drops a
/// leading `#` and a tag that is already there.
pub fn add_tags(model: &mut Model, key: String, typed: &str) -> Cmd {
    if typed.trim().is_empty() {
        return done();
    }
    let value = format!("{}, {typed}", tags_of(model, &key).join(", "));
    let value_type = list_type_of(&key).to_string();
    set_property(model, key, value_type, value)
}

pub fn delete_property(model: &mut Model, key: String) -> Cmd {
    let Some(page) = shown_doc_page_mut(model) else {
        return done();
    };
    let before = page.properties.clone();
    page.properties.remove(&key);
    let page_id = page.id.clone();
    run(move |api| async move {
        let result = api.delete_page_property(&page_id, &key).await;
        settle_property_change(&api, page_id, before, result).await;
    })
}

/// Settles a property change with the server's answer.
async fn settle_property_change(
    api: &crate::effects::Api<crate::app::Event>,
    page_id: String,
    before: BTreeMap<String, PropertyValue>,
    result: Result<Page, crate::effects::ApiError>,
) {
    match result {
        Ok(updated) => api.send_event(Event::PagePropertySaved { page: updated }),
        Err(e) => {
            api.alert(e.0);
            api.send_event(Event::PagePropertyRefused { page_id, before });
        }
    }
}

/// The server saved a property change: its properties and dates replace the shown ones.
pub fn show_property_saved(model: &mut Model, updated: Page) -> Cmd {
    if let Some(page) = shown_page_mut(model, &updated.id) {
        page.properties = updated.properties;
        page.created_date = updated.created_date;
        page.modified_date = updated.modified_date;
    }
    done()
}

/// The server refused a property change: the properties from before it are shown again.
pub fn put_properties_back(
    model: &mut Model,
    page_id: &str,
    before: BTreeMap<String, PropertyValue>,
) -> Cmd {
    if let Some(page) = shown_page_mut(model, page_id) {
        page.properties = before;
    }
    done()
}

/// The doc page on screen when it is still the given one.
fn shown_page_mut<'a>(model: &'a mut Model, page_id: &str) -> Option<&'a mut Page> {
    shown_doc_page_mut(model).filter(|p| p.id == page_id)
}

/// Updates the title or folder of a page. When another page has the new name, asks first
/// whether that page may be overwritten with this one. None when the user said no or the
/// update failed, which has been told.
async fn update_page(
    api: &Api<Event>,
    page_id: &str,
    title: Option<String>,
    dir_path: Option<String>,
    new_path: &str,
) -> Option<Page> {
    let mut overwrite = false;
    loop {
        match api
            .update_page(page_id, title.clone(), dir_path.clone(), overwrite)
            .await
        {
            Ok(PageUpdate::Updated(page)) => return Some(page),
            Ok(PageUpdate::NameTaken(_)) if !overwrite => {
                if !api.confirm(&overwrite_question(new_path)).await {
                    return None;
                }
                overwrite = true;
            }
            Ok(PageUpdate::NameTaken(message)) | Err(ApiError(message)) => {
                api.alert(ApiError(message).message_or("Failed to rename page"));
                return None;
            }
        }
    }
}

/// Renames a childless index page `a/b/index` to the page `a/b`.
pub fn demote_index(model: &mut Model) -> Cmd {
    let Some(p) = shown_doc_page(model).cloned() else {
        return done();
    };
    if p.dir_path.is_empty() {
        return done();
    }
    let name = demote_name(&p);
    if name.is_empty() {
        return done();
    }
    let trimmed = p.dir_path.trim_end_matches('/');
    let parent_dir = match trimmed.rfind('/') {
        Some(slash) => trimmed[..=slash].to_string(),
        None => String::new(),
    };
    let path = format!("{parent_dir}{name}");
    run(move |api| async move {
        // Asked again here: an atlas page may have been made since the button was shown
        let has_atlas = match api.get_child_pages(&p.id).await {
            Ok(children) => children.iter().any(is_atlas),
            Err(e) => return api.alert(e.0),
        };
        if has_atlas
            && !api
                .confirm(&format!(
                    "Rename to \"{name}\" and delete the atlas page of {}?",
                    p.dir_path
                ))
                .await
        {
            return;
        }
        if demote_page(&api, &p.id, &path).await {
            api.send_event(Event::Demoted { path });
        }
    })
}

/// Demotes the page with one request: the server deletes the atlas page and renames the
/// page in one write. Asks before overwriting a page that has the new name. True when the
/// page was demoted.
async fn demote_page(api: &Api<Event>, page_id: &str, new_path: &str) -> bool {
    let mut overwrite = false;
    loop {
        match api.demote_page(page_id, overwrite).await {
            Ok(PageUpdate::Updated(_)) => return true,
            Ok(PageUpdate::NameTaken(_)) if !overwrite => {
                if !api.confirm(&overwrite_question(new_path)).await {
                    return false;
                }
                overwrite = true;
            }
            Ok(PageUpdate::NameTaken(message)) | Err(ApiError(message)) => {
                api.alert(ApiError(message).message_or("Failed to demote page"));
                return false;
            }
        }
    }
}

fn overwrite_question(new_path: &str) -> String {
    format!(
        "A page named \"{new_path}\" already exists. Overwrite it with the content of this page?"
    )
}

/// The server demoted the shown index page: it opens under its new path.
pub fn show_demoted(path: &str) -> Cmd {
    Command::all([sidebar::refresh_recent_pages(), navigate::open_page(path)])
}
