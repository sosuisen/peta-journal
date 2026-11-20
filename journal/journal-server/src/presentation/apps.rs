//! The extension web apps: the folders in `apps/` of the config folder, each served at
//! `/apps/{url_fragment}/` (ADR 073).

use std::collections::HashMap;
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
    let file = get(
        move |Path(params): Path<HashMap<String, String>>, request: Request| {
            file(apps.join(&params["app"]), request)
        },
    );
    router
        .route("/apps/{app}/", file.clone())
        .route("/apps/{app}/{*path}", file)
}

/// Serves the file that `request` asks for from the folder of its app.
async fn file(folder: PathBuf, mut request: Request) -> Response {
    *request.uri_mut() = within_the_app(request.uri());
    let Ok(response) = ServeDir::new(folder).oneshot(request).await;
    response.map(Body::new)
}

/// The URI of a request to `/apps/{app}/...` as the folder of the app sees it: `/...`.
///
/// The path is taken from the URI as it came, still percent-encoded, as `ServeDir` decodes
/// it itself.
fn within_the_app(uri: &Uri) -> Uri {
    let path_and_query = uri.path_and_query().map_or("/", |p| p.as_str());
    let after_apps = &path_and_query["/apps/".len()..];
    let within = &after_apps[after_apps.find('/').unwrap_or(after_apps.len())..];
    Uri::try_from(within).unwrap()
}
