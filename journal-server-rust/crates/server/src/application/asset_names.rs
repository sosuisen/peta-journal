//! Moves the files of `assets/` that were saved under another name to a name made of an ID,
//! and the links to them. Run by hand, once, with the server stopped. See ADR-022.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::domain::asset_name::{is_saved_name, saved_name};
use crate::domain::id_generator::IdGenerator;
use crate::state::AppState;

/// One file of `assets/` and the name it moves to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rename {
    pub from: String,
    pub to: String,
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

/// Moves the files to their new names.
pub fn apply(state: &AppState, renames: &[Rename]) -> anyhow::Result<()> {
    let assets_dir = state
        .export
        .markdown_root()
        .context("The markdown folder is not set or not there")?
        .join("assets");
    for rename in renames {
        std::fs::rename(assets_dir.join(&rename.from), assets_dir.join(&rename.to))?;
    }
    Ok(())
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
