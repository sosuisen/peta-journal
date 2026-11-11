//! The startup step that prepares the markdown directory.

use std::path::{Path, PathBuf};

/// The folders a markdown directory has. A setting that names no directory has none: a
/// folder joined to an empty path would be made where the server was started.
fn sub_dirs(markdown_dir: &str) -> Vec<PathBuf> {
    if markdown_dir.trim().is_empty() {
        return Vec::new();
    }
    ["assets", "docs", "journals"]
        .iter()
        .map(|name| Path::new(markdown_dir).join(name))
        .collect()
}

/// Makes the folders of the markdown directory that are not there yet.
pub fn ensure_sub_dirs(markdown_dir: &str) {
    for dir in sub_dirs(markdown_dir) {
        if !dir.exists() {
            match std::fs::create_dir_all(&dir) {
                Ok(_) => tracing::info!("Created directory: {}", dir.display()),
                Err(e) => tracing::error!("Failed to create directory: {}: {e}", dir.display()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_sub_dirs, sub_dirs};

    // Markdownフォルダーに、assets、docs、journalsのフォルダーが作られる。
    #[test]
    fn the_markdown_directory_gets_its_three_folders() {
        let dir = tempfile::tempdir().unwrap();

        ensure_sub_dirs(&dir.path().to_string_lossy());

        for name in ["assets", "docs", "journals"] {
            assert!(dir.path().join(name).is_dir(), "{name}");
        }
    }

    // Markdownフォルダーが設定されていなければ、作るフォルダーはない。
    #[test]
    fn a_setting_that_names_no_directory_has_no_folders() {
        assert!(sub_dirs("").is_empty());
        assert!(sub_dirs("  ").is_empty());
    }
}
