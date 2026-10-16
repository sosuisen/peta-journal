//! The search box of the header, ported from `search-bar.ts`.

use crate::core::CoreHandle;
use crate::dom;
use leptos::prelude::*;
use peta_core::app::SearchKey;
use peta_core::Event;

#[component]
pub fn SearchBar() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let input = NodeRef::<leptos::html::Input>::new();
    let search = Memo::new(move |_| vm.with(|v| v.search.clone()));
    provide_context(SearchInput(input));

    // The query as the core has it (cleared after a navigation)
    Effect::new(move |_| {
        let q = search.with(|s| s.query.clone());
        if let Some(el) = input.get() {
            if el.value() != q {
                el.set_value(&q);
            }
        }
    });

    let has_results = move || {
        search.with(|s| !s.page_results.is_empty() || !s.item_results.is_empty() || s.show_create) && search.with(|s| s.show_dropdown)
    };

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
                    on:input={ let core = core; move |ev| core.send(Event::SearchQueryChanged(event_target_value(&ev))) }
                    on:keydown=on_keydown
                    on:focus={ let core = core; move |_| core.send(Event::SearchDropdown(true)) }
                    on:blur={ let core = core; move |_| { let c = core; dom::set_timeout(200, move || c.send(Event::SearchDropdown(false))); } }
                />
                <Show when=has_results>
                    <div class="search-dropdown">
                        {move || {
                            let s = search.get();
                                                        let selected = s.selected_index;
                            let pages = s.page_results.iter().enumerate().map(|(i, r)| {
                                                                view! {
                                    <div class="search-result" class:selected=selected == Some(i) on:mousedown=move |_| core.send(Event::SearchResultClicked { index: i })>
                                        <div class="result-page">
                                            <svg class="result-icon" width="14" height="14"><use href="#icon-page"/></svg>
                                            <span class="page-title">{r.page_title.clone()}</span>
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>();
                            let create = s.show_create.then(|| {
                                                                view! {
                                    <div class="search-create" class:selected=selected == Some(s.create_index) on:mousedown=move |_| core.send(Event::SearchCreateClicked)>
                                        {format!("+ Create \"{}\"", s.query)}
                                    </div>
                                }
                            });
                            let items = s.item_results.iter().enumerate().map(|(i, r)| {
                                                                let index = s.item_offset + i;
                                view! {
                                    <div class="search-result" class:selected=selected == Some(index) on:mousedown=move |_| core.send(Event::SearchResultClicked { index })>
                                        <div class="result-item">
                                            <svg class="result-icon" width="14" height="14"><use href="#icon-item"/></svg>
                                            <div class="item-content">
                                                <div class="item-page-path">{r.page_title.clone()}</div>
                                                <div class="item-snippet" inner_html=r.snippet.clone()></div>
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>();
                            view! { {pages} {create} {items} }
                        }}
                    </div>
                </Show>
            </div>
        </app-search-bar>
    }
}

/// The search input, so the layout can focus it on Ctrl+/.
#[derive(Clone, Copy)]
pub struct SearchInput(pub NodeRef<leptos::html::Input>);
