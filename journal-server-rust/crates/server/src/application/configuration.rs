//! The startup step that prepares the markdown directory.

use std::path::Path;

/// Makes the folders of the markdown directory that are not there yet.
pub fn ensure_sub_dirs(markdown_path: &Path) {
    for name in ["assets", "docs", "journals"] {
        let dir = markdown_path.join(name);
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
    use super::ensure_sub_dirs;

    // Markdownフォルダーに、assets、docs、journalsのフォルダーが作られる。
    #[test]
    fn the_markdown_directory_gets_its_three_folders() {
        let dir = tempfile::tempdir().unwrap();

        ensure_sub_dirs(dir.path());

        for name in ["assets", "docs", "journals"] {
            assert!(dir.path().join(name).is_dir(), "{name}");
        }
    }
}
