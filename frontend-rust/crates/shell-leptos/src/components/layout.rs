//! The frame of the app, ported from `main-layout.ts` and `app.ts`: header, sidebar with its
//! resize handle, the screen, the footer, and the global key and click listeners.

use crate::components::page::PageDispatcher;
use crate::components::search_bar::{SearchBar, SearchInput};
use crate::components::sidebar::Sidebar;
use crate::core::CoreHandle;
use crate::dom;
use crate::listeners::DocumentListeners;
use leptos::prelude::*;
use peta_core::view::ScreenView;
use peta_core::Event;
use wasm_bindgen::JsCast;

const CONFIG_KEY: &str = "peta-journal-config";
const STANDARD_PROTOCOLS: [&str; 9] = [
    "http://",
    "https://",
    "mailto:",
    "tel:",
    "sms:",
    "file:",
    "data:",
    "javascript:",
    "about:",
];

fn load_sidebar_width() -> f64 {
    dom::local_storage_get(CONFIG_KEY)
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("sidebarWidth").and_then(|w| w.as_f64()))
        .unwrap_or(260.0)
}

fn save_sidebar_width(width: f64) {
    let mut config = dom::local_storage_get(CONFIG_KEY)
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    config["sidebarWidth"] = serde_json::json!(width);
    dom::local_storage_set(CONFIG_KEY, &config.to_string());
}

/// True for a link with a custom URL scheme (`peta-journal://...`), which the server opens.
fn has_custom_protocol(href: &str) -> bool {
    if href.starts_with('/')
        || href.starts_with("./")
        || href.starts_with("../")
        || href.starts_with('#')
    {
        return false;
    }
    let lower = href.to_lowercase();
    if STANDARD_PROTOCOLS.iter().any(|p| lower.starts_with(p)) {
        return false;
    }
    href.contains("://")
}

#[component]
pub fn MainLayout() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let sidebar_collapsed = RwSignal::new(false);
    let page_menu_open = RwSignal::new(false);
    let sidebar_width = RwSignal::new(load_sidebar_width());
    // The mouse listeners of a sidebar resize in progress
    let resize_listeners = DocumentListeners::new();
    // The search input lives in a child, which cannot provide context to this layout, and a
    // document listener runs with no reactive owner, so the ref is made and provided here
    let search_input = SearchInput(NodeRef::<leptos::html::Input>::new());
    provide_context(search_input);

    let connection = move || vm.with(|v| v.connection.as_str());
    let focused_item = move || vm.with(|v| v.outline.focused_item_id.clone());
    let search_query = move || vm.with_untracked(|v| v.search.query.clone());
    let on_doc_page = move || vm.with(|v| matches!(v.screen, ScreenView::Page(Some(_))));

    // Ctrl+/ or a bare / focuses the search box; custom URL schemes go to the launcher
    {
        let global = DocumentListeners::new();
        global.on_cancelable("keydown", move |ev: &web_sys::KeyboardEvent| {
            if ev.key() != "/" {
                return;
            }
            let is_editing = core
                .view()
                .with_untracked(|v| v.outline.focused_item_id.is_some());
            let focus = || {
                if let Some(el) = search_input.0.get_untracked() {
                    let _ = el.focus();
                }
            };
            if ev.ctrl_key() {
                ev.prevent_default();
                focus();
            } else if !is_editing && !ev.alt_key() && !ev.meta_key() {
                let tag = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .map(|e| e.tag_name())
                    .unwrap_or_default();
                if tag == "INPUT" || tag == "TEXTAREA" {
                    return;
                }
                ev.prevent_default();
                focus();
            }
        });
        global.on_capture("click", move |ev: &web_sys::Event| {
            let Some(target) = ev
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            else {
                return;
            };
            let Some(anchor) = target.closest("a").ok().flatten() else {
                return;
            };
            let Some(href) = anchor.get_attribute("href") else {
                return;
            };
            if !has_custom_protocol(&href) {
                return;
            }
            ev.prevent_default();
            ev.stop_propagation();
            core.send(Event::LaunchUrl { url: href });
        });
    }

    let start_resize = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        let start_x = ev.client_x() as f64;
        let start_width = sidebar_width.get_untracked();
        resize_listeners.clear();
        resize_listeners.on("mousemove", move |ev: &web_sys::MouseEvent| {
            let delta = ev.client_x() as f64 - start_x;
            let width = (start_width + delta).clamp(200.0, 600.0);
            sidebar_width.set(width);
            save_sidebar_width(width);
        });
        resize_listeners.on("mouseup", move |_: &web_sys::Event| {
            resize_listeners.clear_later()
        });
    };

    view! {
        <svg xmlns="http://www.w3.org/2000/svg" style="display:none">
            <symbol id="icon-page" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/>
            </symbol>
            <symbol id="icon-page-filled" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8" fill="none" stroke="#fff"/>
            </symbol>
            <symbol id="icon-item" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="5" cy="7" r="2.5" fill="currentColor" stroke="none"/><line x1="11" y1="7" x2="21" y2="7"/><line x1="11" y1="13" x2="19" y2="13"/><line x1="11" y1="19" x2="17" y2="19"/>
            </symbol>
        </svg>
        <app-main-layout>
        <div class="layout" class:sidebar-collapsed=move || sidebar_collapsed.get()>
            <header class="header">
                <div class="header-left">
                    <button class="menu-btn" on:click=move |_| sidebar_collapsed.update(|c| *c = !*c)>"☰"</button>
                    <button class="journal-btn" title="Today" on:click=move |_| core.send(Event::NavigateToToday)>
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <rect x="3" y="1" width="16" height="20" rx="1.5" ry="1.5"/>
                            <line x1="3" y1="13" x2="19" y2="13" stroke="#666" stroke-width="1"/>
                            <line x1="3" y1="17" x2="19" y2="17" stroke="#666" stroke-width="1"/>
                            <line x1="14" y1="2" x2="14" y2="20" stroke="#666" stroke-width="3"/>
                            <rect x="5" y="21" width="3" height="2" fill="#666" stroke="none"/>
                        </svg>
                    </button>
                    <button class="root-btn" title="Root" on:click=move |_| core.send(Event::NavigateToRoot)>
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M3 7v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-6l-2-2H5a2 2 0 0 0-2 2z"/>
                        </svg>
                    </button>
                </div>
                <div class="header-center">
                    <button class="new-page-btn" title="New page" on:click=move |_| core.send(Event::CreateNewPage { search_query: search_query() })>"+"</button>
                    <SearchBar />
                </div>
                <div class="header-right">
                    <button class="menu-btn home-btn" title="Home" on:click=move |_| core.send(Event::NavigateToToday)>
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
                    </button>
                    <div class="page-menu">
                        <button class="page-menu-btn" on:click=move |_| page_menu_open.update(|o| *o = !*o)>"⋮"</button>
                        <Show when=move || page_menu_open.get()>
                            <div class="page-menu-dropdown" on:click=move |_| page_menu_open.set(false)>
                                <Show when=on_doc_page>
                                    <button class="page-menu-item" on:click=move |_| core.send(Event::EditPageProperties)>"Edit properties"</button>
                                </Show>
                                <button class="page-menu-item" on:click=move |_| core.send(Event::DeleteCurrentPage)>"Delete page"</button>
                            </div>
                        </Show>
                    </div>
                </div>
            </header>
            <div class="body">
                <aside class="sidebar" class:collapsed=move || sidebar_collapsed.get()
                       style:width=move || if sidebar_collapsed.get() { "0px".to_string() } else { format!("{}px", sidebar_width.get()) }>
                    <Sidebar />
                </aside>
                <Show when=move || !sidebar_collapsed.get()>
                    <div class="resize-handle" on:mousedown=start_resize></div>
                </Show>
                <main class="content">
                    <PageDispatcher />
                </main>
            </div>
            <footer class="footer">
                <span class=move || format!("connection-status {}", connection())>{connection}</span>
                {move || focused_item().map(|id| view! { <span class="item-id">"Item: " {id}</span> })}
            </footer>
        </div>
        </app-main-layout>
    }
}
