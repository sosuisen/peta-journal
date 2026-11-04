//! Writing and deleting files so that other readers never see them half done.

use std::path::Path;

/// Writes to a temp file in the same directory and renames it over the target. A reader then
/// finds the old content or the new one, never a half-written or empty file: not Dropbox, not
/// the folder watcher, not a second request reading the config.
pub fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    std::fs::write(&tmp, content)?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        delete_quietly(&tmp);
        return Err(e);
    }
    Ok(())
}

pub fn delete_quietly(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => tracing::debug!("Could not delete {}: {e}", path.display()),
    }
}
