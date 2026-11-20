//! The extension web apps: the folders in `apps/` of the config folder, each served at
//! `/apps/{url_fragment}/` (ADR 073).

use std::collections::HashMap;
use std::path::{Component, PathBuf};

use axum::body::Body;
use axum::extract::{Path, Request};
use axum::http::header::CACHE_CONTROL;
use axum::http::{HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::Router;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

/// Adds the routes of the apps in `apps/` of `config_dir` to `router`.
pub fn serve(router: Router, config_dir: PathBuf) -> Router {
    let apps = config_dir.join("apps");
    let file = get(
        move |Path(params): Path<HashMap<String, String>>, request: Request| {
            app_file(apps.clone(), params["app"].clone(), request)
        },
    );
    router
        .route("/apps/{app}", get(to_the_folder))
        .route("/apps/{app}/", file.clone())
        .route("/apps/{app}/{*path}", file)
}

/// Serves the file of the app named `app` in `apps`, or 404 when `app` would lead out of it.
async fn app_file(apps: PathBuf, app: String, request: Request) -> Response {
    if is_one_name(&app) {
        file(apps.join(app), request).await
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

/// Whether `app` is the name of one folder. `..`, `.` and a name with a separator, decoded
/// from `%2e%2e` or `%2f`, would lead out of `apps/`.
fn is_one_name(app: &str) -> bool {
    let mut parts = std::path::Path::new(app).components();
    matches!(
        (parts.next(), parts.next()),
        (Some(Component::Normal(_)), None)
    )
}

/// Sends `/apps/{app}` on to `/apps/{app}/`, so that the relative URLs of the app point into
/// its folder.
async fn to_the_folder(uri: Uri) -> Redirect {
    let with_slash = match uri.query() {
        Some(query) => format!("{}/?{query}", uri.path()),
        None => format!("{}/", uri.path()),
    };
    Redirect::temporary(&with_slash)
}

/// Serves the file that `request` asks for from the folder of its app.
///
/// The browser asks on every load whether the file changed. Each build tool puts a hash into
/// the names of its own way, and the server does not want to know them all to cache for long.
async fn file(folder: PathBuf, mut request: Request) -> Response {
    *request.uri_mut() = within_the_app(request.uri());
    let files = ServeDir::new(&folder);
    let Ok(mut response) = if has_an_extension(request.uri().path()) {
        files.oneshot(request).await
    } else {
        let index = ServeFile::new(folder.join("index.html"));
        files.fallback(index).oneshot(request).await
    };
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response.map(Body::new)
}

/// Whether the last segment of `path` has an extension, as `app.js` does. The app shows a
/// screen at a path without one, such as `/tasks/1`.
fn has_an_extension(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.contains('.'))
}

/// The URI of a request to `/apps/{app}/...` as the folder of the app sees it: `/...`.
///
/// The path is taken from the URI as it came, still percent-encoded, as `ServeDir` decodes
/// it itself.
fn within_the_app(uri: &Uri) -> Uri {
    let path_and_query = uri.path_and_query().map_or("/", |p| p.as_str());
    let after_apps = &path_and_query["/apps/".len()..];
    let slash = after_apps
        .find('/')
        .expect("the routes end the name of the app with a slash");
    Uri::try_from(&after_apps[slash..]).expect("a part of a URI is a URI")
}
