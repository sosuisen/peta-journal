//! The history of the pages that were opened, for a shell without a history of its own
//! (ADR 035).

use crate::app::{done, Cmd, Model, Route};
use crate::screens::route;

/// Opens the URL.
pub fn open(model: &mut Model, url: &str, today: String) -> Cmd {
    match Route::parse_url(url) {
        Some(to) => route::route(model, to, today),
        None => done(),
    }
}
