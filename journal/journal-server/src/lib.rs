//! PetaJournal journal server: a local REST, SSE and MCP server over a SQLite database and a
//! markdown folder.

pub mod application;
pub mod config;
pub mod error;
pub mod infrastructure;
pub mod listen;
pub mod presentation;
pub mod state;

use axum::http::{header, HeaderName, HeaderValue, Method};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::state::SharedState;

/// The complete HTTP application: REST, SSE and MCP with the CORS settings of the Java server,
/// and the web frontend when the settings name its folder.
pub fn build_router(state: SharedState) -> Router {
    let origins: Vec<HeaderValue> = state
        .settings
        .cors_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static(presentation::client_id::HEADER),
        ])
        .expose_headers([header::CONTENT_DISPOSITION]);

    let mcp = presentation::mcp::service(state.clone());
    let frontend_dir = state.settings.frontend_dir.clone();
    let config_dir = state.settings.config_dir.clone();
    let router = presentation::rest::router()
        .nest_service("/mcp", mcp)
        .with_state(state);
    let router = presentation::apps::serve(router, config_dir);
    let router = match frontend_dir {
        Some(dir) => presentation::frontend::serve(router, &dir),
        None => router,
    };
    router.layer(cors)
}
