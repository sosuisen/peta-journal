//! The popup with page names that match the text typed inside a `[[wiki link]]`.

use crate::core::CoreHandle;
use leptos::prelude::*;
use peta_core::Event;

#[component]
pub fn WikiSuggestions() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let wiki = Memo::new(move |_| vm.with(|v| v.outline.wiki.clone()));
    let shown = move || wiki.with(|w| !w.suggestions.is_empty());

    view! {
        <app-wiki-suggestions>
            <Show when=shown>
                <div
                    class="wiki-suggestions"
                    style:left=move || format!("{}px", wiki.with(|w| w.x))
                    style:top=move || format!("{}px", wiki.with(|w| w.y))
                >
                    <For
                        each=move || wiki.with(|w| w.suggestions.iter().cloned().enumerate().collect::<Vec<_>>())
                        key=|(i, s)| (*i, s.clone())
                        children={
                                                        move |(i, title)| {
                                let selected = move || wiki.with(|w| w.index == i);
                                                                let t = title.clone();
                                view! {
                                    <div
                                        class="wiki-suggestion-item"
                                        class:selected=selected
                                        on:mousedown=move |ev: web_sys::MouseEvent| {
                                            ev.prevent_default();
                                            ev.stop_propagation();
                                            core.send(Event::WikiPick { title: t.clone() });
                                        }
                                    >
                                        {title}
                                    </div>
                                }
                            }
                        }
                    />
                </div>
            </Show>
        </app-wiki-suggestions>
    }
}
