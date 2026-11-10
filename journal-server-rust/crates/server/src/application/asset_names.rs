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

/// The new name of every file in the folder. Nothing is changed.
pub fn plan(assets_dir: &Path) -> std::io::Result<Vec<Rename>> {
    let ids = IdGenerator::new();
    let mut plan = Vec::new();
    for entry in std::fs::read_dir(assets_dir)? {
        let entry = entry?;
        let from = entry.file_name().to_string_lossy().into_owned();
        let id = ids.generate_asset_id_at(millis(entry.metadata()?.modified()?));
        let to = saved_name(&id, &from);
        plan.push(Rename { from, to });
    }
    Ok(plan)
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
