//! The doc page screen, ported from `page.ts`: loading a page by its path, the title and
//! directory editing, and demoting an index page.

use crate::app::{apply, done, log_error, run, Cmd, Model, PageState, Screen};
use crate::effects::UiOp;
use crate::model::Page;
use crate::screens::{navigate, sidebar};
use crate::util::filename::validate_file_name;
use crux_core::Command;
use serde_json::Value;
use std::collections::BTreeMap;

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

fn page_mut(model: &mut Model) -> Option<&mut PageState> {
    match &mut model.screen {
        Screen::Page(p) => Some(p),
        _ => None,
    }
}

/// Shows a doc page by its path; the fragment is the item to scroll to.
pub fn open(model: &mut Model, name: &str, fragment: Option<String>) -> Cmd {
    model.screen = Screen::Page(PageState {
        page_name: name.to_string(),
        focus_item_id: fragment.filter(|f| !f.is_empty()),
        page: None,
        can_demote_index: false,
        properties_open: false,
    });
    let page_name = name.to_string();
    run(move |api| async move {
        let (dir_path, title) = split_doc_path(&page_name);
        let result = match api.get_or_create_doc(&title, &dir_path).await {
            Ok(r) => r,
            Err(e) => return log_error(format!("Failed to load page {page_name}: {e}")),
        };
        let p = result.page.clone();
        // If the URL does not match the canonical path, redirect, e.g. the server resolved
        // "Quarkus" to dirPath "Quarkus/" and name "index"
        let canonical = p.canonical_path();
        if canonical != page_name {
            apply(&api, "page: redirect", move |_| {
                navigate::to_page(&canonical, false, false, true, None)
            });
            return;
        }
        let created = result.created;
        let page_for_state = p.clone();
        apply(&api, "page loaded", move |m| {
            let Some(ps) = page_mut(m) else { return done() };
            if ps.page_name != page_for_state.canonical_path()
                && ps.page_name != page_for_state.name
            {
                return done();
            }
            ps.page = Some(page_for_state.clone());
            let mut cmds = vec![
                navigate::set_title(full_page_title(&page_for_state)),
                sidebar::set_current_page(m, Some(page_for_state.clone())),
            ];
            if created {
                cmds.push(sidebar::refresh_recent_pages());
                cmds.push(
                    Command::notify_shell(UiOp::ExtensionPageCreate {
                        page_id: page_for_state.id.clone(),
                        file_name: page_for_state.name.clone(),
                        dir_name: page_for_state.dir_path.clone(),
                        page_type: page_for_state.page_type.clone(),
                    })
                    .into(),
                );
            }
            Command::all(cmds)
        });
        // A non-root index page can be demoted when it has no children
        if p.name == "index" && !p.dir_path.is_empty() {
            let can_demote = api
                .get_child_pages(&p.id)
                .await
                .map(|c| c.is_empty())
                .unwrap_or(false);
            apply(&api, "page: can demote", move |m| {
                if let Some(ps) = page_mut(m) {
                    ps.can_demote_index = can_demote;
                }
                done()
            });
        }
        // The outline loads the items of the page
        match api.get_page_by_id(&p.id).await {
            Ok(loaded) => apply(&api, "page items", move |m| {
                if page_state(m)
                    .and_then(|ps| ps.page.as_ref())
                    .map(|pg| pg.id.as_str())
                    != Some(&p.id)
                {
                    return done();
                }
                m.state.set_items(loaded.items);
                if let Some(trees) = loaded.transcluded_trees {
                    m.state.set_transcluded_trees(trees);
                }
                scroll_to_focus_item(m)
            }),
            Err(e) => log_error(format!("Failed to load items for page {}: {e}", p.id)),
        }
    })
}

/// Selects and scrolls to the item the URL fragment names, when the page has it.
fn scroll_to_focus_item(model: &mut Model) -> Cmd {
    let Some(item_id) = page_state(model).and_then(|p| p.focus_item_id.clone()) else {
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

/// The display title: journals as dates, index pages as their directory, docs as their name.
pub fn page_title(p: &Page) -> String {
    if p.page_type == "journal" {
        return crate::util::date::format_journal_date(&p.name);
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
    if p.name == "index" {
        return if p.dir_path.is_empty() {
            "(root)/".to_string()
        } else {
            p.dir_path.clone()
        };
    }
    format!("{}{}", p.dir_path, p.name)
}

/// The breadcrumb segments of a dir path: `("a", "a")`, `("b", "a/b")`, ...
pub fn dir_path_segments(dir_path: &str) -> Vec<(String, String)> {
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
            (part.to_string(), path.clone())
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
    let Some(p) = page_state(model).and_then(|ps| ps.page.clone()) else {
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
        let updated = match api
            .update_page(
                &p.id,
                title_changed.then(|| new_name.clone()),
                dir_changed.then(|| new_dir.clone()),
            )
            .await
        {
            Ok(u) => u,
            Err(e) => {
                api.notify(UiOp::Alert(e.0));
                return;
            }
        };
        let canonical = updated.canonical_path();
        apply(&api, "title saved", move |m| {
            if let Some(ps) = page_mut(m) {
                ps.page = Some(updated.clone());
                ps.page_name = canonical.clone();
            }
            Command::all([
                navigate::set_title(full_page_title(&updated)),
                sidebar::refresh_recent_pages(),
                navigate::to_page(&canonical, false, false, true, None),
            ])
        });
    })
}

/// Shows the property rows of the doc page, so a first property can be added.
pub fn edit_properties(model: &mut Model) -> Cmd {
    if let Some(ps) = page_mut(model) {
        ps.properties_open = true;
    }
    done()
}

/// The shown value of a property as the server would store it: `tag` is a list.
fn property_value(value_type: &str, value: &str) -> Value {
    if value_type == "tags" || value_type == "list" {
        Value::Array(
            value
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(|t| Value::String(t.to_string()))
                .collect(),
        )
    } else {
        Value::String(value.to_string())
    }
}

/// Sets a property of the doc page. The screen changes at once; the server's answer (which
/// formats numbers) replaces it, and a refusal puts the old properties back. The page's
/// dates (`created`, `modified`) only change when the server has accepted them.
pub fn set_property(model: &mut Model, key: String, value_type: String, value: String) -> Cmd {
    let Some(page) = page_mut(model).and_then(|ps| ps.page.as_mut()) else {
        return done();
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
pub fn delete_property(model: &mut Model, key: String) -> Cmd {
    let Some(page) = page_mut(model).and_then(|ps| ps.page.as_mut()) else {
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

async fn settle_property_change(
    api: &crate::effects::Api<crate::app::Event>,
    page_id: String,
    before: BTreeMap<String, Value>,
    result: Result<Page, crate::effects::ApiError>,
) {
    match result {
        Ok(updated) => apply(api, "page property saved", move |m| {
            if let Some(page) = shown_page_mut(m, &updated.id) {
                page.properties = updated.properties;
                page.created_date = updated.created_date;
                page.modified_date = updated.modified_date;
            }
            done()
        }),
        Err(e) => {
            api.notify(UiOp::Alert(e.0));
            apply(api, "page property reverted", move |m| {
                if let Some(page) = shown_page_mut(m, &page_id) {
                    page.properties = before;
                }
                done()
            });
        }
    }
}

/// The doc page on screen when it is still the given one.
fn shown_page_mut<'a>(model: &'a mut Model, page_id: &str) -> Option<&'a mut Page> {
    page_mut(model)
        .and_then(|ps| ps.page.as_mut())
        .filter(|p| p.id == page_id)
}

/// Renames a childless index page `a/b/index` to the page `a/b`.
pub fn demote_index(model: &mut Model) -> Cmd {
    let Some(p) = page_state(model).and_then(|ps| ps.page.clone()) else {
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
    run(move |api| async move {
        if let Err(e) = api
            .update_page(&p.id, Some(name.clone()), Some(parent_dir.clone()))
            .await
        {
            api.notify(UiOp::Alert(if e.0.is_empty() {
                "Failed to rename page".into()
            } else {
                e.0
            }));
            return;
        }
        apply(&api, "demoted", move |_| {
            Command::all([
                sidebar::refresh_recent_pages(),
                navigate::to_page(&format!("{parent_dir}{name}"), false, false, false, None),
            ])
        });
    })
}
