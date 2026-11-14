//! The search box of the header, ported from `search-bar.ts`.

use crate::core::CoreHandle;
use crate::dom;
use client_core::app::SearchKey;
use client_core::view::{SearchEntry, SearchKind};
use client_core::Event;
use leptos::prelude::*;

#[component]
pub fn SearchBar() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let SearchInput(input) = expect_context();
    let search = core.select(|v| v.search.clone());

    // The query as the core has it (cleared after a navigation)
    Effect::new(move |_| {
        let q = search.with(|s| s.query.clone());
        if let Some(el) = input.get() {
            if el.value() != q {
                el.set_value(&q);
            }
        }
    });

    let has_results = move || search.with(|s| !s.results.is_empty() && s.show_dropdown);

    let on_keydown = {
        move |ev: web_sys::KeyboardEvent| {
            if dom::is_ime_composing(&ev) {
                return;
            }
            let key = match ev.key().as_str() {
                "ArrowDown" => SearchKey::Down,
                "ArrowUp" => SearchKey::Up,
                "Enter" => {
                    if search.with(|s| s.selected_index.is_none()) {
                        return;
                    }
                    SearchKey::Enter
                }
                "Escape" => SearchKey::Escape,
                _ => return,
            };
            ev.prevent_default();
            core.send(Event::SearchKey(key));
        }
    };

    view! {
        <app-search-bar>
            <div class="search-container">
                <input
                    node_ref=input
                    type="text"
                    class="search-input"
                    placeholder="Type Ctrl+/ to search"
                    on:input=move |ev| core.send(Event::SearchQueryChanged(event_target_value(&ev)))
                    on:keydown=on_keydown
                    on:focus=move |_| core.send(Event::SearchDropdown(true))
                    on:blur=move |_| { let c = core; dom::set_timeout(200, move || c.send(Event::SearchDropdown(false))); }
                />
                <Show when=has_results>
                    <div class="search-dropdown">
                        {move || {
                            let s = search.get();
                            let selected = s.selected_index;
                            s.results.into_iter().map(|entry| {
                                let SearchEntry { position, kind, title, label, snippet } = entry;
                                let is_selected = selected == Some(position);
                                let click = move |_: web_sys::MouseEvent| core.send(Event::SearchResultClicked { position });
                                match kind {
                                    SearchKind::Today => view! {
                                        <div class="search-today" class:selected=is_selected on:mousedown=click>
                                            {label}
                                        </div>
                                    }.into_any(),
                                    SearchKind::Page => view! {
                                        <div class="search-result" class:selected=is_selected on:mousedown=click>
                                            <div class="result-page">
                                                <svg class="result-icon" width="14" height="14"><use href="#icon-page"/></svg>
                                                <span class="page-title">{title}</span>
                                            </div>
                                        </div>
                                    }.into_any(),
                                    SearchKind::Create => view! {
                                        <div class="search-create" class:selected=is_selected on:mousedown=click>
                                            {label}
                                        </div>
                                    }.into_any(),
                                    SearchKind::Item => view! {
                                        <div class="search-result" class:selected=is_selected on:mousedown=click>
                                            <div class="result-item">
                                                <svg class="result-icon" width="14" height="14"><use href="#icon-item"/></svg>
                                                <div class="item-content">
                                                    <div class="item-page-path">{title}</div>
                                                    <div class="item-snippet" inner_html=snippet></div>
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any(),
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </div>
                </Show>
            </div>
        </app-search-bar>
    }
}

/// The search input, made and provided by the layout so it can focus it on Ctrl+/.
#[derive(Clone, Copy)]
pub struct SearchInput(pub NodeRef<leptos::html::Input>);
