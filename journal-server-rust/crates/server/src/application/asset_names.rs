//! Moves the files of `assets/` that were saved under another name to a name made of an ID,
//! and the links to them. Run by hand, once, with the server stopped. See ADR-022.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use hmtid::Hmtid;
use serde::{Deserialize, Serialize};

use crate::application::config_service::ConfigService;
use crate::config::{AssetNamesStep, Settings};
use crate::domain::asset_name::{is_saved_name, relink, saved_name};
use crate::domain::id_generator::IdGenerator;
use crate::domain::model::Item;
use crate::infrastructure::persistence::ItemRepository;
use crate::state::{AppState, BuildOptions};

/// One file of `assets/` and the name it moves to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rename {
    pub from: String,
    pub to: String,
}

/// Runs one step of the command and returns what to tell the person who ran it.
pub fn run(settings: Settings, step: AssetNamesStep, table: &Path) -> anyhow::Result<String> {
    match step {
        AssetNamesStep::Plan => {
            let markdown_dir = ConfigService::new(settings.config_dir).markdown_dir();
            let renames = plan(&Path::new(&markdown_dir).join("assets"))?;
            std::fs::write(table, serde_json::to_string_pretty(&renames)?)?;
            Ok(format!(
                "{} files to rename. The table is in {}",
                renames.len(),
                table.display()
            ))
        }
        AssetNamesStep::Apply => {
            let renames = read_table(table)?;
            rename_all(settings, &renames)
        }
        AssetNamesStep::Revert => {
            let back: Vec<Rename> = read_table(table)?
                .into_iter()
                .map(|rename| Rename {
                    from: rename.to,
                    to: rename.from,
                })
                .collect();
            rename_all(settings, &back)
        }
    }
}

fn read_table(table: &Path) -> anyhow::Result<Vec<Rename>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(table)?)?)
}

/// Opens the database and the search index as the server does, and applies the renames.
/// The server must not run: it holds the search index.
fn rename_all(settings: Settings, renames: &[Rename]) -> anyhow::Result<String> {
    let state = AppState::build(settings, BuildOptions { interactive: false })?;
    apply(&state, renames)?;
    state.shutdown();
    Ok(format!("{} files renamed", renames.len()))
}

/// The new name of every file in the folder that has no ID for its name yet, oldest file
/// first. Nothing is changed.
pub fn plan(assets_dir: &Path) -> std::io::Result<Vec<Rename>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(assets_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_saved_name(&name) {
            continue;
        }
        let data = entry.metadata()?;
        // A copied file was made at the time of the copy and keeps its modified time; a file
        // changed later keeps the time it was made. The older one is when it was first saved
        let saved = data.modified()?.min(data.created()?);
        files.push((millis(saved), name));
    }
    // Oldest first, because the generator gives an older time the time of the one before
    files.sort();
    let ids = IdGenerator::new();
    Ok(files
        .into_iter()
        .map(|(saved_ms, from)| {
            let to = saved_name(&ids.generate_asset_id_at(saved_ms), &from);
            Rename { from, to }
        })
        .collect())
}

/// Moves the files to their new names, and points the links of the items to them.
pub fn apply(state: &AppState, renames: &[Rename]) -> anyhow::Result<()> {
    let assets_dir = state
        .export
        .markdown_root()
        .context("The markdown folder is not set or not there")?
        .join("assets");
    // First what the folder has and the database has not: a file that waits to be taken in
    // is not written over, and would be taken in later with its old links
    state.reconcile.scan();
    let relinked = relink_items(state, renames)?;
    for rename in renames {
        // A file that is not there moved already: in a run that stopped half way, or on
        // another device, where the synced folder brought the file before this ran
        let from = assets_dir.join(&rename.from);
        if from.exists() {
            std::fs::rename(from, assets_dir.join(&rename.to))?;
        }
    }
    for item in &relinked {
        let page_title = format!("{}{}", item.dir_path, item.name);
        state.search.index_item(item, &page_title);
    }
    // At once and not through the queue: the items keep their modified dates, so a file
    // with the old links would look edited by hand to the next import, and bring them back
    let pages: BTreeSet<&Hmtid> = relinked.iter().map(|item| &item.page_id).collect();
    for page_id in pages {
        state.export.export_page(page_id)?;
    }
    Ok(())
}

/// Points the links of the items to the new names, all of them or none. Returns the items
/// that changed, as they are now.
fn relink_items(state: &AppState, renames: &[Rename]) -> anyhow::Result<Vec<Item>> {
    let new_names: HashMap<String, String> = renames
        .iter()
        .map(|rename| (rename.from.clone(), rename.to.clone()))
        .collect();
    let mut conn = state.db.conn()?;
    let tx = conn.immediate()?;
    let mut relinked = Vec::new();
    for item in state.items.find_by_body_containing_asset_path()? {
        let body = relink(&item.body, &new_names);
        if body != item.body {
            let ast = state.parser.parse_to_ast(&body);
            ItemRepository::rewrite_body_in(&tx, &item.id, &body, &ast)?;
            relinked.push(Item { body, ast, ..item });
        }
    }
    tx.commit()?;
    Ok(relinked)
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
