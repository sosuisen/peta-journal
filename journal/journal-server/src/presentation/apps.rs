//! The extension web apps: the folders in `apps/` of the config folder, each served at
//! `/apps/{url_fragment}/` (ADR 073).

use std::path::PathBuf;

use axum::body::Body;
use axum::extract::{Path, Request};
use axum::http::Uri;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use tower::ServiceExt;
use tower_http::services::ServeDir;

/// Adds the routes of the apps in `apps/` of `config_dir` to `router`.
pub fn serve(router: Router, config_dir: PathBuf) -> Router {
    let apps = config_dir.join("apps");
    router.route(
        "/apps/{app}/{*path}",
        get(
            move |Path((app, path)): Path<(String, String)>, request: Request| {
                file(apps.join(app), path, request)
            },
        ),
    )
}

async fn file(folder: PathBuf, path: String, mut request: Request) -> Response {
    *request.uri_mut() = Uri::try_from(format!("/{path}")).unwrap();
    let Ok(response) = ServeDir::new(folder).oneshot(request).await;
    response.map(Body::new)
}
