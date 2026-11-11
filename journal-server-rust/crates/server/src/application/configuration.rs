//! The startup check of the markdown directory, which may ask the user for it.

use std::path::Path;

use journal_core::application::config_service::ConfigService;
use journal_core::infrastructure::export::export_service::MarkdownExportService;

use super::folder_chooser;

/// Makes sure a markdown directory is configured (asking with the native folder chooser if not),
/// creates its sub directories and queues a first export when it is empty.
pub fn check_configuration(config: &ConfigService, export: &MarkdownExportService) {
    let mut markdown_dir = config.markdown_dir();
    tracing::info!("Current markdown directory: [{markdown_dir}]");
    if markdown_dir.trim().is_empty() {
        tracing::warn!("Markdown directory not configured. Opening folder chooser...");
        let choice = folder_chooser::choose_folder();
        tracing::info!(
            "Folder chooser returned: selected={}, path=[{}]",
            choice.selected,
            choice.path
        );
        if choice.selected && !choice.path.is_empty() {
            config.set_markdown_dir(&choice.path);
            markdown_dir = choice.path;
            tracing::info!("Markdown directory configured: {markdown_dir}");
        } else {
            tracing::error!(
                "No markdown directory selected. Application may not function correctly."
            );
        }
    }
    if !markdown_dir.trim().is_empty() {
        ensure_sub_dirs(Path::new(&markdown_dir));
        export.export_all_if_empty(&markdown_dir);
    }
}

fn ensure_sub_dirs(markdown_path: &Path) {
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
