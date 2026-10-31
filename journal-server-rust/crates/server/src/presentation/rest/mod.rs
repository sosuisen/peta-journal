//! The REST API, route for route the same as the Java server.

pub mod assets;
pub mod atlas;
pub mod config;
pub mod functions;
pub mod items;
pub mod journals;
pub mod launcher;
pub mod pages;
pub mod search;
pub mod sync;

use axum::routing::get;
use axum::Router;

use crate::state::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/events", get(events))
        .merge(pages::routes())
        .merge(atlas::routes())
        .merge(items::routes())
        .merge(journals::routes())
        .merge(search::routes())
        .merge(config::routes())
        .merge(sync::routes())
        .merge(assets::routes())
        .merge(functions::routes())
        .merge(launcher::routes())
}

async fn health() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        "{\"status\": \"ok\", \"service\": \"peta-journal-server\"}\n",
    )
}

async fn events(
    axum::extract::State(state): axum::extract::State<SharedState>,
) -> impl axum::response::IntoResponse {
    state.sse.event_stream()
}
