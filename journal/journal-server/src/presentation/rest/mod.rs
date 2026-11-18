//! The REST API, route for route the same as the Java server.

pub mod assets;
pub mod atlas;
pub mod config;
pub mod confirm;
pub mod items;
pub mod journals;
pub mod launcher;
pub mod pages;
pub mod search;
pub mod sync;

use axum::extract::Request;
use axum::handler::Handler;
use axum::http::Method;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{on, MethodFilter, MethodRouter};
use axum::Router;
use journal_api_types::routes::{self, Route};

use crate::error::HttpError;
use crate::presentation::client_id;
use crate::state::SharedState;

/// The form of the page about missing files posts here. A form of a browser cannot add a
/// header, so this one path needs no client ID.
const FORM_PATH: &str = routes::SUBMIT_PENDING_DELETES.pattern;

/// The handler of a route of the API, under the method of the route. The routes are the ones
/// of `journal_api_types::routes`, which the frontend core makes its requests from.
pub(crate) fn handling<H, T>(route: Route, handler: H) -> MethodRouter<SharedState>
where
    H: Handler<T, SharedState>,
    T: 'static,
{
    let method = match route.method {
        routes::Method::Get => MethodFilter::GET,
        routes::Method::Post => MethodFilter::POST,
        routes::Method::Put => MethodFilter::PUT,
        routes::Method::Delete => MethodFilter::DELETE,
    };
    on(method, handler)
}

/// Puts a handler at a route of the API.
pub(crate) trait At {
    fn at<H, T>(self, route: Route, handler: H) -> Self
    where
        H: Handler<T, SharedState>,
        T: 'static;
}

impl At for Router<SharedState> {
    fn at<H, T>(self, route: Route, handler: H) -> Self
    where
        H: Handler<T, SharedState>,
        T: 'static,
    {
        self.route(route.pattern, handling(route, handler))
    }
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .at(routes::HEALTH, health)
        .at(routes::EVENTS, events)
        .merge(pages::routes())
        .merge(atlas::routes())
        .merge(items::routes())
        .merge(journals::routes())
        .merge(search::routes())
        .merge(config::routes())
        .merge(confirm::routes())
        .merge(sync::routes())
        .merge(assets::routes())
        .merge(launcher::routes())
        .layer(axum::middleware::from_fn(require_client_id))
}

/// Refuses a request that may change something when its client does not name itself
/// (ADR-015). The method alone decides, so a POST that only reads needs the ID too.
///
/// The header is read here and nowhere else. A handler takes the ID as `Extension<ClientId>`.
async fn require_client_id(mut request: Request, next: Next) -> Response {
    let may_change = [Method::POST, Method::PUT, Method::DELETE].contains(request.method());
    if may_change && request.uri().path() != FORM_PATH {
        match client_id::of(request.headers()) {
            Ok(client) => request.extensions_mut().insert(client),
            Err(refusal) => return HttpError(refusal).into_response(),
        };
    }
    next.run(request).await
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
