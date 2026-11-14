//! PetaJournal's web frontend: a Leptos shell around the Crux core in `client-core`.

mod body;
mod components;
mod core;
mod dom;
mod effects;
mod listeners;

use crate::components::layout::MainLayout;
use crate::core::CoreHandle;
use client_core::{Event, Route};
use leptos::prelude::*;
use leptos_router::components::Router;
use leptos_router::hooks::{use_location, use_navigate};
use leptos_router::NavigateOptions;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <Router><App /></Router> });
}

/// The route of the location, whose parts the browser keeps encoded.
fn parse_route(pathname: &str, hash: &str) -> Option<Route> {
    let path = dom::decode_uri(pathname);
    let fragment = hash.strip_prefix('#').map(dom::decode_uri);
    Route::parse(&path, fragment.as_deref())
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

    core.send(Event::IndexSortsRestored(
        components::sidebar::load_index_sorts(),
    ));
    core.send(Event::AtlasDirsRestored(
        components::atlas::load_atlas_dirs(),
    ));
    core.send(Event::Start {
        client_id: core.inner().client_id.clone(),
    });

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
