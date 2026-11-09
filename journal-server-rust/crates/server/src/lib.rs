//! PetaJournal journal server: a local REST, SSE and MCP server over a SQLite database and a
//! markdown folder.

pub mod application;
pub mod config;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod presentation;
pub mod state;

use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::routing::any;
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::state::SharedState;

/// The complete HTTP application: REST, SSE and MCP with the CORS settings of the Java server.
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
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .expose_headers([header::CONTENT_DISPOSITION]);

    let mcp = presentation::mcp::service(state.clone());
    let frontend_dir = state.settings.frontend_dir.clone();
    let mut router = presentation::rest::router()
        .nest_service("/mcp", mcp)
        .with_state(state);
    if let Some(dir) = frontend_dir {
        let index = ServeFile::new(dir.join("index.html"));
        router = router
            .route("/api/{*path}", any(|| async { StatusCode::NOT_FOUND }))
            .fallback_service(ServeDir::new(dir).fallback(index));
    }
    router.layer(cors)
}
