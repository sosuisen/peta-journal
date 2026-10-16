//! PetaJournal's web frontend: a Leptos shell around the Crux core in `peta-core`.

mod components;
mod core;
mod dom;
mod effects;
mod listeners;

use crate::components::layout::MainLayout;
use crate::core::CoreHandle;
use leptos::prelude::*;
use leptos_router::components::Router;
use leptos_router::hooks::{use_location, use_navigate};
use leptos_router::NavigateOptions;
use peta_core::{Event, Route};

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <Router><App /></Router> });
}

/// The URL as the core understands it: `/today`, `/pages/<name>`, `/docs/<name>`,
/// `/journals/<name>` or `/settings`.
fn parse_route(pathname: &str, hash: &str) -> Option<Route> {
    let path = dom::decode_uri(pathname.trim_start_matches('/'));
    let fragment = hash
        .strip_prefix('#')
        .filter(|f| !f.is_empty())
        .map(dom::decode_uri);
    if path == "settings" {
        return Some(Route::Settings);
    }
    if path.is_empty() {
        return None;
    }
    if path == "today" {
        return Some(Route::Page {
            name: "today".into(),
            fragment,
        });
    }
    for prefix in ["pages/", "docs/", "journals/"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            let name = rest.trim_end_matches('/').to_string();
            return Some(Route::Page {
                name: if name.is_empty() {
                    "today".into()
                } else {
                    name
                },
                fragment,
            });
        }
    }
    Some(Route::Page {
        name: path,
        fragment,
    })
}

#[component]
fn App() -> impl IntoView {
    let core = CoreHandle::init();
    provide_context(core);

    let navigate = use_navigate();
    *core.inner().navigate.borrow_mut() = Some(Box::new(move |url: &str, replace: bool| {
        let url = dom::encode_uri(url);
        navigate(
            &url,
            NavigateOptions {
                replace,
                scroll: false,
                ..Default::default()
            },
        );
    }));

    core.send(Event::Start);

    let location = use_location();
    {
        Effect::new(move |_| {
            let pathname = location.pathname.get();
            let hash = location.hash.get();
            match parse_route(&pathname, &hash) {
                Some(route) => core.send(Event::Route {
                    route,
                    today: dom::today_iso(),
                }),
                None => core.navigate("/today", true),
            }
        });
    }

    view! { <MainLayout /> }
}
