//! The sidebar, ported from `sidebar.ts`: calendar, the index pages, the table of contents,
//! the recent pages and the settings button.

use crate::components::calendar::Calendar;
use crate::core::CoreHandle;
use crate::dom;
use crate::listeners::DocumentListeners;
use leptos::prelude::*;
use peta_core::model::{IndexSort, IndexSortKey, SortOrder};
use peta_core::view::SidebarPage;
use peta_core::Event;
use std::collections::{HashMap, HashSet};
use wasm_bindgen::JsCast;

const COLLAPSED_KEY: &str = "sidebar-collapsed";
const INDEX_SORT_KEY: &str = "sidebar-index-sort";

/// The sort of each directory's index list, as the browser stored it.
pub fn load_index_sorts() -> HashMap<String, IndexSort> {
    dom::local_storage_get(INDEX_SORT_KEY)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_index_sort(dir: &str, sort: IndexSort) {
    let mut sorts = load_index_sorts();
    sorts.insert(dir.to_string(), sort);
    dom::local_storage_set(INDEX_SORT_KEY, &serde_json::to_string(&sorts).unwrap_or_default());
}

fn sort_key_label(key: IndexSortKey) -> &'static str {
    match key {
        IndexSortKey::Name => "Name",
        IndexSortKey::Modified => "Modified",
        IndexSortKey::Published => "Published",
    }
}

/// The sort control of the index list: one small button that opens a popup menu with the
/// six sorts, the current one marked.
#[component]
fn IndexSortControl(dir: Memo<String>, sort: Memo<IndexSort>) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let menu_open = RwSignal::new(false);
    // The click outside and the Escape that close the open menu
    let menu_listeners = DocumentListeners::new();
    let close_menu = move || {
        menu_open.set(false);
        menu_listeners.clear_later();
    };
    let open_menu = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        if menu_open.get() {
            close_menu();
            return;
        }
        menu_open.set(true);
        // Listening starts after this click, so the click itself does not close the menu
        dom::after_render(move || {
            menu_listeners.on_cancelable("mousedown", move |ev: &web_sys::MouseEvent| {
                let inside_menu = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .is_some_and(|t| t.closest(".index-sort-menu").ok().flatten().is_some());
                if !inside_menu {
                    close_menu();
                }
            });
            menu_listeners.on("keydown", move |ev: &web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    ev.prevent_default();
                    close_menu();
                }
            });
        });
    };
    let choose = move |chosen: IndexSort| {
        let dir = dir.get();
        save_index_sort(&dir, chosen);
        core.send(Event::SetIndexSort { dir, sort: chosen });
        close_menu();
    };
    let sorts = [IndexSortKey::Name, IndexSortKey::Modified, IndexSortKey::Published]
        .into_iter()
        .flat_map(|key| [SortOrder::Asc, SortOrder::Desc].map(|order| IndexSort { key, order }));
    view! {
        <div class="index-sort">
            <button class="collapse-btn index-sort-btn" title="Sort" on:click=open_menu>
                // Three bars of falling length: the usual sort glyph, unlike the collapse chevrons
                <svg width="12" height="12" viewBox="0 0 12 12" fill="#888">
                    <rect x="1" y="2" width="10" height="1.6"/>
                    <rect x="1" y="5.2" width="7" height="1.6"/>
                    <rect x="1" y="8.4" width="4" height="1.6"/>
                </svg>
            </button>
            <Show when=move || menu_open.get()>
                <div class="index-sort-menu">
                    {sorts.clone().map(|item| {
                        let arrow = match item.order { SortOrder::Asc => "↑", SortOrder::Desc => "↓" };
                        view! {
                            <button class="index-sort-item" class:current=move || sort.get() == item
                                on:click=move |_| choose(item)>
                                <span class="index-sort-check">{move || if sort.get() == item { "✓" } else { "" }}</span>
                                {format!("{} {arrow}", sort_key_label(item.key))}
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </Show>
        </div>
    }
}

fn load_collapsed() -> HashSet<String> {
    dom::local_storage_get(COLLAPSED_KEY)
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .map(|v| v.into_iter().collect())
        .unwrap_or_default()
}

#[component]
fn SectionToggle(collapsed: Signal<bool>, on_toggle: Callback<()>) -> impl IntoView {
    view! {
        <button class="collapse-btn" on:click=move |_| on_toggle.run(()) aria-label=move || if collapsed.get() { "Expand" } else { "Collapse" }>
            <svg width="10" height="12" viewBox="0 0 10 12">
                {move || if collapsed.get() {
                    view! {
                        <polygon points="2,4.5 5,1.5 8,4.5" fill="#888"/>
                        <polygon points="2,7.5 5,10.5 8,7.5" fill="#888"/>
                    }.into_any()
                } else {
                    view! {
                        <polygon points="2,2.5 5,5.5 8,2.5" fill="#888"/>
                        <polygon points="2,9.5 5,6.5 8,9.5" fill="#888"/>
                    }.into_any()
                }}
            </svg>
        </button>
    }
}

#[component]
pub fn Sidebar() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let today = dom::today_iso();
    let collapsed = RwSignal::new(load_collapsed());
    let is_collapsed = move |section: &'static str| {
        Signal::derive(move || collapsed.with(|c| c.contains(section)))
    };
    let toggle = move |section: &'static str| {
        Callback::new(move |_| {
            collapsed.update(|c| {
                if !c.remove(section) {
                    c.insert(section.to_string());
                }
                let list: Vec<&String> = c.iter().collect();
                dom::local_storage_set(
                    COLLAPSED_KEY,
                    &serde_json::to_string(&list).unwrap_or_default(),
                );
            });
        })
    };

    let index_header = core.select(|v| v.sidebar.index_header.clone());
    let index_pages = core.select(|v| v.sidebar.index_pages.clone());
    let index_sort = core.select(|v| v.sidebar.index_sort);
    let index_parent_path = core.select(|v| v.sidebar.index_parent_path.clone());
    let headings = core.select(|v| v.sidebar.headings.clone());
    let recent = core.select(|v| v.sidebar.recent_pages.clone());
    let current_page_id = core.select(|v| v.sidebar.current_page_id.clone());

    let go_to_page = {
        move |page: &SidebarPage, ev: &web_sys::MouseEvent| {
            core.send(Event::GoToPage {
                path: page.path.clone(),
                new_tab: ev.ctrl_key() || ev.meta_key(),
                new_window: ev.shift_key(),
            });
        }
    };

    let page_list = move |pages: Vec<SidebarPage>, active: bool| {
        let go_to_page = go_to_page;
        pages
            .into_iter()
            .map(|p| {
                let go = go_to_page;
                let is_active = active && current_page_id.with(|c| c.as_deref() == Some(p.page.id.as_str()));
                let label = p.label.clone();
                let dir_label = p.dir_label.clone();
                let is_dir = p.is_dir;
                let icon = if p.is_dir {
                    "#icon-folder"
                } else if p.published {
                    "#icon-page-filled"
                } else {
                    "#icon-page"
                };
                view! {
                    <div class="sidebar-item recent-page-item" class:folder-item=is_dir class:active=is_active on:click=move |ev| go(&p, &ev)>
                        <svg class="page-icon" width=if is_dir { "14" } else { "11" } height=if is_dir { "12" } else { "11" }><use href=icon/></svg>
                        <span class="page-label">
                            {dir_label.map(|d| view! { <span class="page-dir">{d}</span> })}
                            <span class="page-name">{label}</span>
                        </span>
                    </div>
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <app-sidebar>
            <nav class="sidebar">
                <div class="sidebar-content">
                    <div class="sidebar-section">
                        <Calendar selected_date=today.clone() />
                    </div>

                    <Show when=move || !index_pages.with(|p| p.is_empty()) || index_parent_path.with(|p| p.is_some())>
                        <div class="sidebar-section">
                            <div class="section-header">
                                <h3 class="section-title section-title-link index-header-title" on:click={
                                                                        move |ev: web_sys::MouseEvent| {
                                        let header = index_header.get();
                                        let path = if header == "(root)/" { "index".to_string() } else { format!("{header}index") };
                                        core.send(Event::GoToPage { path, new_tab: ev.ctrl_key() || ev.meta_key(), new_window: ev.shift_key() });
                                    }
                                }>
                                    <svg class="page-icon" width="14" height="12"><use href="#icon-folder"/></svg>
                                    <span class="index-header-name">{move || index_header.get()}</span>
                                </h3>
                                <IndexSortControl dir=index_header sort=index_sort />
                                <SectionToggle collapsed=is_collapsed("index") on_toggle=toggle("index") />
                            </div>
                            <Show when=move || !is_collapsed("index").get()>
                                <div class="index-page-list">
                                    {move || index_parent_path.get().map(|path| view! {
                                        <div class="sidebar-item recent-page-item folder-item" on:click=move |ev: web_sys::MouseEvent| {
                                            core.send(Event::GoToPage { path: path.clone(), new_tab: ev.ctrl_key() || ev.meta_key(), new_window: ev.shift_key() });
                                        }>
                                            <svg class="page-icon" width="14" height="12"><use href="#icon-folder"/></svg>
                                            <span class="page-label"><span class="page-name">"../"</span></span>
                                        </div>
                                    })}
                                    {move || page_list(index_pages.get(), true)}
                                </div>
                            </Show>
                        </div>
                    </Show>

                    <Show when=move || !headings.with(|h| h.is_empty())>
                        <div class="sidebar-section">
                            <div class="section-header">
                                <h3 class="section-title">"Contents"</h3>
                                <SectionToggle collapsed=is_collapsed("contents") on_toggle=toggle("contents") />
                            </div>
                            <Show when=move || !is_collapsed("contents").get()>
                                <div class="contents-list">
                                    {move || headings.get().into_iter().map(|h| {
                                                                                let id = h.id.clone();
                                        view! {
                                            <div class=format!("contents-item contents-h{}", h.level)
                                                 on:click=move |_| core.send(Event::ScrollToItemRequested { item_id: id.clone() })>
                                                {h.text.clone()}
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </Show>
                        </div>
                    </Show>

                    <div class="sidebar-section">
                        <div class="section-header">
                            <h3 class="section-title">"Recent Pages"</h3>
                            <SectionToggle collapsed=is_collapsed("recent") on_toggle=toggle("recent") />
                        </div>
                        <Show when=move || !is_collapsed("recent").get()>
                            {move || page_list(recent.get(), false)}
                        </Show>
                    </div>
                </div>

                <div class="sidebar-footer">
                    <img src="/peta-journal-gray.png" alt="PetaJournal" class="footer-logo" />
                    <span class="footer-text">"PetaJournal"</span>
                    <button class="settings-btn" title="Settings" on:click=move |_| core.navigate("/settings", false)>
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#888" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <circle cx="12" cy="12" r="3"/>
                            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
                        </svg>
                    </button>
                </div>
            </nav>
        </app-sidebar>
    }
}
