//! Opens a URL (http or a custom scheme like `peta-journal://page/123`) with the OS.

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
