//! The URLs of the core's navigation as the routes the core understands.

use peta_core::Route;

/// The URL as the core understands it: `/today`, `/pages/<name>`, `/docs/<name>`,
/// `/journals/<name>` or `/settings`, with the item to go to after a `#`. None when it names
/// nothing.
pub fn parse_url(url: &str) -> Option<Route> {
    let (path, fragment) = match url.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment.to_string())),
        None => (url, None),
    };
    let path = path.trim_start_matches('/');
    if path == "settings" {
        return Some(Route::Settings);
    }
    if path.is_empty() {
        return None;
    }
    let name = ["pages/", "docs/", "journals/"]
        .iter()
        .find_map(|prefix| path.strip_prefix(prefix))
        .map(|rest| rest.trim_end_matches('/'))
        .unwrap_or(path);
    let name = if name.is_empty() { "today" } else { name };
    Some(Route::Page {
        name: name.to_string(),
        fragment,
    })
}
