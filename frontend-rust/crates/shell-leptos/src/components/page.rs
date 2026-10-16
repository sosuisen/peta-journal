//! The screen under the header: the journal with its date header, a doc page with its
//! editable title, or the settings. Ported from `page-dispatcher.ts`, `journal.ts` and
//! `page.ts`.

use crate::components::calendar::Calendar;
use crate::components::outline::Outline;
use crate::components::settings::Settings;
use crate::core::CoreHandle;
use crate::dom;
use leptos::prelude::*;
use peta_core::app::JournalNav;
use peta_core::util::filename::JournalType;
use peta_core::view::{PageView, ScreenView};
use peta_core::Event;
use wasm_bindgen::JsCast;

#[component]
pub fn PageDispatcher() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let kind = Memo::new(move |_| {
        vm.with(|v| match &v.screen {
            ScreenView::None => 0,
            ScreenView::Journal(_) => 1,
            ScreenView::Page(_) => 2,
            ScreenView::Settings => 3,
        })
    });
    view! {
        <app-page-dispatcher>
            {move || match kind.get() {
                1 => view! { <Journal /> }.into_any(),
                2 => view! { <DocPage /> }.into_any(),
                3 => view! { <Settings /> }.into_any(),
                _ => ().into_any(),
            }}
        </app-page-dispatcher>
    }
}

#[component]
fn Journal() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let journal = Memo::new(move |_| {
        vm.with(|v| match &v.screen {
            ScreenView::Journal(j) => Some(j.clone()),
            _ => None,
        })
    });
    let show_calendar = RwSignal::new(false);
    let scrolled = move || vm.with(|v| v.outline.scrolled_from_top);
    let display_date = move || {
        journal.with(|j| {
            j.as_ref()
                .map(|j| j.display_date.clone())
                .unwrap_or_default()
        })
    };
    let current_date = Signal::derive(move || {
        journal.with(|j| {
            j.as_ref()
                .map(|j| j.current_date.clone())
                .unwrap_or_default()
        })
    });
    let journal_type = move || journal.with(|j| j.as_ref().and_then(|j| j.journal_type));
    let has_page =
        move || journal.with(|j| j.as_ref().map(|j| j.page_id.is_some()).unwrap_or(false));
    let toggle = move |_| show_calendar.update(|s| *s = !*s);
    let nav = move |n: JournalNav| move |_| core.send(Event::JournalNav(n));
    let on_navigate = Callback::new(move |keep_open: bool| {
        if !keep_open {
            show_calendar.set(false);
        }
    });

    view! {
        <app-journal>
            <div class="journal-header" class:scrolled=scrolled>
                <h2 class="journal-date" on:click=toggle>{display_date}</h2>
                <button class="calendar-btn" on:click=toggle>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>
                </button>
                <Show when=move || journal_type() == Some(JournalType::Daily)>
                    <button class="nav-btn" on:click=nav(JournalNav::PrevDay)>"<"</button>
                    <button class="nav-btn" on:click=nav(JournalNav::NextDay)>">"</button>
                </Show>
                <Show when=move || journal_type() == Some(JournalType::Weekly)>
                    <button class="nav-btn" on:click=nav(JournalNav::PrevWeek)>"<"</button>
                    <button class="nav-btn" on:click=nav(JournalNav::NextWeek)>">"</button>
                </Show>
            </div>
            <Show when=move || show_calendar.get()>
                <Calendar selected_date=current_date on_navigate=on_navigate />
            </Show>
            <Show when=has_page>
                <Outline />
            </Show>
        </app-journal>
    }
}

#[component]
fn DocPage() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let page = Memo::new(move |_| {
        vm.with(|v| match &v.screen {
            ScreenView::Page(p) => p.clone(),
            _ => None,
        })
    });
    let scrolled = move || vm.with(|v| v.outline.scrolled_from_top);
    let editing = RwSignal::new(false);
    let edit_dir = RwSignal::new(String::new());
    let edit_name = RwSignal::new(String::new());
    let dir_input = NodeRef::<leptos::html::Input>::new();
    let name_input = NodeRef::<leptos::html::Input>::new();

    let start_edit = {
        move |ev: web_sys::MouseEvent, p: PageView| {
            edit_dir.set(p.page.dir_path.trim_end_matches('/').to_string());
            edit_name.set(p.page.name.clone());
            // The caret goes where the title was clicked
            let caret = dom::caret_point_from_point(ev.client_x() as f64, ev.client_y() as f64)
                .map(|(_, offset)| offset)
                .unwrap_or(p.page.name.encode_utf16().count() as u32);
            editing.set(true);
            let is_index = p.page.name == "index";
            dom::after_render(move || {
                if is_index {
                    if let Some(input) = dir_input.try_get_untracked().flatten() {
                        let _ = input.focus();
                        let len = input.value().encode_utf16().count() as u32;
                        let _ = input.set_selection_range(len, len);
                    }
                } else if let Some(input) = name_input.try_get_untracked().flatten() {
                    let _ = input.focus();
                    let _ = input.set_selection_range(caret, caret);
                }
            });
        }
    };
    let save = {
        move || {
            if !editing.get_untracked() {
                return;
            }
            editing.set(false);
            core.send(Event::PageTitleSave {
                dir_path: edit_dir.get_untracked(),
                file_name: edit_name.get_untracked(),
            });
        }
    };
    let on_title_key = {
        move |ev: web_sys::KeyboardEvent| {
            if dom::is_ime_composing(&ev) {
                return;
            }
            match ev.key().as_str() {
                "Enter" => save(),
                "Escape" => editing.set(false),
                _ => {}
            }
        }
    };
    let on_focus_out = {
        move |ev: web_sys::FocusEvent| {
            let row = ev
                .current_target()
                .and_then(|t| t.dyn_into::<web_sys::Node>().ok());
            let next = ev
                .related_target()
                .and_then(|t| t.dyn_into::<web_sys::Node>().ok());
            let stays = match (&row, &next) {
                (Some(row), Some(next)) => row.contains(Some(next)),
                _ => false,
            };
            if !stays {
                save();
            }
        }
    };

    view! {
        <app-page>
            {move || page.get().map(|p| {
                let p2 = p.clone();
                let start = start_edit;
                let on_title_key2 = on_title_key;
                view! {
                    <div class="page-header" class:scrolled=scrolled>
                        <Show when=move || editing.get() fallback={
                            let p = p2.clone();
                                                        move || {
                                let p = p.clone();
                                let crumbs = p.dir_segments.iter().enumerate().map(|(i, (name, path))| {
                                    let last = i + 1 == p.dir_segments.len();
                                                                        let path = path.clone();
                                    if p.is_index && last {
                                        let pp = p.clone();
                                        view! {
                                            <span class="dir-segment-title" on:click=move |ev| start(ev, pp.clone())>{name.clone()}</span>
                                            <span class="breadcrumb-slash">"/"</span>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <a class="dir-segment" on:click=move |_| core.send(Event::GoToPage { path: format!("{path}/index"), new_tab: false, new_window: false })>{name.clone()}</a>
                                            <span class="breadcrumb-slash">"/"</span>
                                        }.into_any()
                                    }
                                }).collect::<Vec<_>>();
                                let title = if p.is_root_index {
                                    view! { <h2 class="page-title root-index-title">"(root)/"</h2><div class="index-label">"index"</div> }.into_any()
                                } else if p.is_index {
                                                                        let demote = p.demote_name.clone();
                                    view! {
                                        <div class="index-label">
                                            "index "
                                            <Show when={ let can = p.can_demote; move || can }>
                                                <button class="demote-btn" on:click=move |_| core.send(Event::DemoteIndex)>
                                                    {format!("ページ名を {demote} に変更する")}
                                                </button>
                                            </Show>
                                        </div>
                                    }.into_any()
                                } else {
                                    let pp = p.clone();
                                    view! { <h2 class="page-title" on:click=move |ev| start(ev, pp.clone())>{p.title.clone()}</h2> }.into_any()
                                };
                                let breadcrumb = (!p.page.dir_path.is_empty()).then(|| view! {
                                    <div class="dir-path-breadcrumb" class:index-breadcrumb=p.is_index>{crumbs}</div>
                                });
                                view! { {breadcrumb} {title} }
                            }
                        }>
                            <div class="edit-row" on:focusout=on_focus_out>
                                <Show when={ let has = !p.page.dir_path.is_empty(); move || has }>
                                    <input node_ref=dir_input class="edit-dir-path" prop:value=move || edit_dir.get()
                                        on:input=move |ev| edit_dir.set(event_target_value(&ev)) on:keydown=on_title_key />
                                    <span class="edit-slash">"/"</span>
                                </Show>
                                <input node_ref=name_input class="edit-file-name" prop:value=move || edit_name.get() prop:disabled=p.is_index
                                    on:input=move |ev| edit_name.set(event_target_value(&ev)) on:keydown=on_title_key2 />
                            </div>
                        </Show>
                    </div>
                    <Outline />
                }
            })}
        </app-page>
    }
}
