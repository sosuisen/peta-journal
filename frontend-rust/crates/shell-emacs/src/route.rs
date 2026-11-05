//! The URLs of the core's navigation as the routes the core understands.

use peta_core::Route;

/// The URL as the core understands it, with the item to go to after a `#`. The URLs that reach
/// this shell are not encoded, so there is nothing to decode.
pub fn parse_url(url: &str) -> Option<Route> {
    match url.split_once('#') {
        Some((path, fragment)) => Route::parse(path, Some(fragment)),
        None => Route::parse(url, None),
    }
}
