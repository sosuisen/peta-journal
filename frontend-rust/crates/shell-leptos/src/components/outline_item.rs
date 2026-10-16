//! One row of the outline, ported from `outline-item.ts`: the controls, the bullet, the
//! editor or the rendered body, the task times and the item menu.

use crate::components::item_editor::ItemEditor;
use crate::components::item_view::ItemView;
use crate::core::CoreHandle;
use crate::dom;
use gloo_events::EventListener;
use leptos::prelude::*;
use peta_core::view::Row;
use peta_core::Event;
use wasm_bindgen::JsCast;

#[component]
pub fn OutlineItem(row: RwSignal<Row>) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let item_id = row.with_untracked(|r| r.item.id.clone());
    let host = NodeRef::<leptos::html::Div>::new();

    let menu_open = RwSignal::new(false);
    let menu_pos = RwSignal::new((0.0, 0.0));
    // The document listeners of the open menu; they go away with the row
    let menu_listeners: StoredValue<Vec<EventListener>, LocalStorage> =
        StoredValue::new_local(Vec::new());

    let send = move |event: Event| core.send(event);

    let close_menu = move || {
        menu_open.set(false);
        // The listeners are dropped after the event that closes the menu, not inside it
        dom::after_render(move || {
            menu_listeners.try_update_value(|l| l.clear());
        });
    };

    let on_menu_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        let Some(btn) = ev
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        else {
            return;
        };
        let btn_rect = btn.get_bounding_client_rect();
        let host_rect = host.get().map(|h| h.get_bounding_client_rect());
        let (hl, ht) = host_rect.map(|r| (r.left(), r.top())).unwrap_or((0.0, 0.0));
        menu_pos.set((btn_rect.left() - hl, btn_rect.bottom() - ht));
        menu_open.set(true);
        let document = dom::document();
        dom::after_render(move || {
            let close = EventListener::new_with_options(
                &document,
                "mousedown",
                gloo_events::EventListenerOptions::enable_prevent_default(),
                move |ev| {
                    if let Some(t) = ev
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    {
                        if t.closest(".item-popup-menu").ok().flatten().is_some() {
                            return;
                        }
                    }
                    close_menu();
                },
            );
            let on_key = EventListener::new(&dom::document(), "keydown", move |ev| {
                let ev: &web_sys::KeyboardEvent = ev.unchecked_ref();
                if ev.key() == "Escape" {
                    ev.prevent_default();
                    close_menu();
                }
            });
            menu_listeners.try_update_value(|l| {
                l.push(close);
                l.push(on_key);
            });
        });
    };

    let copy_text = |text: String| {
        let _ = dom::window().navigator().clipboard().write_text(&text);
    };

    let indent_px = move || row.with(|r| format!("{}px", r.indent_px));
    let is_selected = move || row.with(|r| r.is_selected);
    let is_heading = move || row.with(|r| r.is_heading);
    let heading_level =
        move || row.with(|r| (r.heading_level > 0).then(|| r.heading_level.to_string()));
    let hide_bullet = move || row.with(|r| !r.show_bullet);
    let has_children = move || row.with(|r| r.has_children);
    let is_collapsed = move || row.with(|r| r.is_collapsed);
    let is_transclusion = move || row.with(|r| r.is_transclusion);
    // Which body to show. A memo, so typing (which changes the row) does not rebuild the editor
    let body_mode = Memo::new(move |_| row.with(|r| (r.is_broken_transclusion, r.is_editing)));
    let zoom = move || {
        row.with(|r| {
            if (r.effective_zoom - 1.0).abs() < f64::EPSILON {
                String::new()
            } else {
                format!("{}", r.effective_zoom)
            }
        })
    };
    let has_zoom = move || row.with(|r| (r.item.zoom() - 1.0).abs() > f64::EPSILON);
    let task_time = move || row.with(|r| task_time_info(&r.item));

    let id_for_view = item_id.clone();
    view! {
        <app-outline-item>
            <div
                node_ref=host
                id=item_id.clone()
                class="outline-item"
                class:selected=is_selected
                class:heading=is_heading
                class:hide-bullet=hide_bullet
                data-heading-level=heading_level
                style:padding-left=indent_px
            >
                <div class="item-controls">
                    <div class="slot-menu">
                        <Show when=has_children>
                            <button class="item-menu" on:click=on_menu_click>"⋯"</button>
                        </Show>
                    </div>
                    <div class="slot-toggle">
                        <Show when=has_children fallback=move || view! { <button class="item-menu" on:click=on_menu_click>"⋯"</button> }>
                            <button
                                class="collapse-toggle"
                                class:collapsed=is_collapsed
                                on:click={
                                    let send = send;
                                    let id = id_for_view.clone();
                                    move |_| send(Event::CollapseToggle { item_id: id.clone() })
                                }
                            >
                                <svg class="triangle" viewBox="0 0 10 6" width="10" height="6">
                                    <polygon points="0,0 10,0 5,6" fill="currentColor" />
                                </svg>
                            </button>
                        </Show>
                    </div>
                </div>
                <div class="item-content" style:zoom=zoom>
                    <div class="slot-bullet">
                        <span class="bullet">
                            <svg viewBox="0 0 14 14" width="14" height="14">
                                {move || {
                                    if is_transclusion() {
                                        if is_collapsed() {
                                            view! { <rect x="1" y="1" width="12" height="12" rx="1" fill="#ddd" stroke="none" /> }.into_any()
                                        } else {
                                            view! { <rect x="1" y="1" width="12" height="12" rx="1" fill="none" stroke="#ccc" stroke-width="1" /> }.into_any()
                                        }
                                    } else if is_collapsed() {
                                        view! { <circle cx="7" cy="7" r="6.5" fill="#ddd" stroke="none" /> }.into_any()
                                    } else {
                                        ().into_any()
                                    }
                                }}
                                <circle cx="7" cy="7" r="2.5" fill="#888" stroke="none" />
                            </svg>
                        </span>
                    </div>
                    <div class="item-body">
                        {move || {
                            match body_mode.get() {
                                (true, _) => {
                                    let body = move || row.with(|r| r.item.body.clone());
                                    view! { <div class="broken-transclusion">"source not found: " {body}</div> }.into_any()
                                }
                                (false, true) => view! { <ItemEditor row=row /> }.into_any(),
                                (false, false) => view! { <ItemView row=row /> }.into_any(),
                            }
                        }}
                    </div>
                    {move || task_time().map(|(label, tooltip)| view! {
                        <div class="task-timestamps" title=tooltip>{label}</div>
                    })}
                </div>
            </div>
            <Show when=move || menu_open.get()>
                {
                    let id = item_id.clone();
                    move || {
                        let (x, y) = menu_pos.get();
                        let body = row.with_untracked(|r| r.item.body.clone());
                        let id = id.clone();
                        let close = close_menu;
                        view! {
                            <div class="item-popup-menu" style:left=format!("{x}px") style:top=format!("{y}px")>
                                <button on:click={ let b = body.clone(); move |_| { copy_text(b.clone()); close(); } }>"Copy"</button>
                                <button on:click={ let b = body.clone(); let id = id.clone(); move |_| { copy_text(b.clone()); send(Event::BodyChanged { item_id: id.clone(), body: String::new(), now_ms: core.now_ms() }); close(); } }>"Cut"</button>
                                <button on:click={ let id = id.clone(); move |_| { copy_text(id.clone()); close(); } }>"Copy Reference"</button>
                                <button on:click={ let id = id.clone(); move |_| { send(Event::PasteAsTransclusion { item_id: id.clone() }); close(); } }>"Paste as Transclusion"</button>
                                {is_transclusion().then(|| { let id = id.clone(); view! {
                                    <button on:click=move |_| { send(Event::RemoveTransclusion { item_id: id.clone() }); close(); }>"Remove Transclusion"</button>
                                }})}
                                <button on:click={ let id = id.clone(); move |_| { send(Event::SendToArchive { item_id: id.clone() }); close(); } }>"Send to Archive"</button>
                                {has_zoom().then(|| { let id = id.clone(); view! {
                                    <button on:click=move |_| { send(Event::ResetZoom { item_id: id.clone() }); close(); }>"Reset Zoom"</button>
                                }})}
                            </div>
                        }
                    }
                }
            </Show>
        </app-outline-item>
    }
}

/// The label and tooltip of a task's times: `HH:MM-` while running, the duration when done.
fn task_time_info(item: &peta_core::model::Item) -> Option<(String, String)> {
    if !item.is_task() {
        return None;
    }
    let begin = item.prop_str("begin")?;
    let begin_label = dom::local_time(begin);
    let Some(end) = item.prop_str("end") else {
        return Some((format!("{begin_label}-"), format!("{begin_label}-")));
    };
    let total_sec = ((dom::epoch_ms(end) - dom::epoch_ms(begin)) / 1000.0)
        .floor()
        .max(0.0) as i64;
    let (h, m, s) = (total_sec / 3600, (total_sec % 3600) / 60, total_sec % 60);
    let mut label = String::new();
    if h > 0 {
        label.push_str(&format!("{h}h"));
    }
    if m > 0 {
        label.push_str(&format!("{m}m"));
    }
    if h == 0 && m == 0 {
        label = format!("{s}s");
    }
    Some((label, format!("{begin_label} - {}", dom::local_time(end))))
}
