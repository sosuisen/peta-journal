//! Moves the files of `assets/` that were saved under another name to a name made of an ID,
//! and the links to them. Run by hand, once, with the server stopped. See ADR-022.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::domain::asset_name::saved_name;
use crate::domain::id_generator::IdGenerator;

/// One file of `assets/` and the name it moves to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rename {
    pub from: String,
    pub to: String,
}

/// The new name of every file in the folder, oldest file first. Nothing is changed.
pub fn plan(assets_dir: &Path) -> std::io::Result<Vec<Rename>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(assets_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        files.push((millis(entry.metadata()?.modified()?), name));
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

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
