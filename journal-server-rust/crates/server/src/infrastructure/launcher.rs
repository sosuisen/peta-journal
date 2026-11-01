//! Opens a URL (http or a custom scheme like `peta-journal://page/123`) or a folder with the OS.

use std::path::Path;

/// Returns true if the OS accepted the request.
pub fn launch(url: &str) -> bool {
    match open::that_detached(url) {
        Ok(_) => {
            tracing::info!("Launched app with URL: {url}");
            true
        }
        Err(e) => {
            tracing::error!("Failed to launch app with URL {url}: {e}");
            false
        }
    }
}

/// Opens a folder in the file manager of the OS. Returns true if the OS accepted the request.
pub fn open_folder(path: &Path) -> bool {
    match open::that_detached(path) {
        Ok(_) => {
            tracing::info!("Opened folder: {}", path.display());
            true
        }
        Err(e) => {
            tracing::error!("Failed to open folder {}: {e}", path.display());
            false
        }
    }
}
