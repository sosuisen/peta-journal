//! The property rows between the title and the body of a doc page: one row per property,
//! `+` adds a row whose key is picked from the definitions, `×` removes a property.
//! Date-times are shown and typed in local time to the minute and sent as UTC.

use crate::core::CoreHandle;
use crate::dom;
use leptos::prelude::*;
use peta_core::model::PropertyDefinition;
use peta_core::view::ScreenView;
use peta_core::Event;

/// A row added with `+` and not yet saved: it becomes a property once it has a key and a
/// value. Kept by the page, so a re-render of the header does not lose it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub id: u32,
    pub key: String,
}

#[component]
pub fn PageProperties(drafts: RwSignal<Vec<Draft>>) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let properties = core.select(|v| match &v.screen {
        ScreenView::Page(Some(p)) => p.properties.clone(),
        _ => Vec::new(),
    });
    let definitions = core.select(|v| v.config.property_definitions.clone());
    let type_of = move |key: &str| {
        definitions.with(|ds| {
            ds.iter()
                .find(|d| d.key == key)
                .map(|d| d.value_type.clone())
                .unwrap_or_else(|| "text".to_string())
        })
    };
    // The keys a new row may take: usable on pages and not on the page yet
    let free_keys = Memo::new(move |_| {
        let used: Vec<String> = properties.with(|ps| ps.iter().map(|p| p.key.clone()).collect());
        definitions.with(|ds| {
            ds.iter()
                .filter(|d| d.page && !used.contains(&d.key))
                .cloned()
                .collect::<Vec<PropertyDefinition>>()
        })
    });
    let next_id = StoredValue::new(1u32);
    let add_draft = move |_| {
        let id = next_id.get_value();
        next_id.set_value(id + 1);
        drafts.update(|ds| {
            ds.push(Draft {
                id,
                key: String::new(),
            })
        });
    };

    view! {
        <div class="page-properties">
            <For each=move || properties.get() key=|p| p.key.clone() children=move |p| {
                let key = p.key.clone();
                let value = Memo::new({
                    let key = key.clone();
                    move |_| properties.with(|ps| ps.iter().find(|q| q.key == key).map(|q| q.value.clone()).unwrap_or_default())
                });
                let value_type = if p.is_list { "tags".to_string() } else { type_of(&p.key) };
                let send = {
                    let key = key.clone();
                    let value_type = value_type.clone();
                    move |value: String| core.send(Event::SetPageProperty { key: key.clone(), value_type: value_type.clone(), value })
                };
                let remove = { let key = key.clone(); move |_| core.send(Event::DeletePageProperty { key: key.clone() }) };
                let shown: AnyView = if p.editable {
                    value_input(&value_type, value.into(), send, false, NodeRef::new())
                } else {
                    view! { <span class="property-value property-readonly">{move || dom::local_minutes(&value.get())}</span> }.into_any()
                };
                view! {
                    <div class="property-row">
                        <span class="property-key">{key.clone()}</span>
                        {shown}
                        {p.removable.then(|| view! { <button class="property-remove" title="Remove" on:click=remove>"×"</button> })}
                    </div>
                }
            } />
            <For each=move || drafts.get() key=|d| d.id children=move |d| {
                let id = d.id;
                let key = Memo::new(move |_| drafts.with(|ds| ds.iter().find(|x| x.id == id).map(|x| x.key.clone()).unwrap_or_default()));
                let value_type = Memo::new(move |_| {
                    let k = key.get();
                    if k.is_empty() { String::new() } else { type_of(&k) }
                });
                let pick = move |ev: web_sys::Event| {
                    let chosen = event_target_value(&ev);
                    drafts.update(|ds| if let Some(x) = ds.iter_mut().find(|x| x.id == id) { x.key = chosen });
                };
                // A value with a key becomes the property; the draft row is then gone
                let commit = move |value: String| {
                    let k = key.get_untracked();
                    if k.is_empty() {
                        return;
                    }
                    core.send(Event::SetPageProperty { key: k, value_type: value_type.get_untracked(), value });
                    drafts.update(|ds| ds.retain(|x| x.id != id));
                };
                let discard = move |_| drafts.update(|ds| ds.retain(|x| x.id != id));
                view! {
                    <div class="property-row draft">
                        <select class="property-key-select" prop:value=move || key.get() on:change=pick>
                            <option value="">"key"</option>
                            {move || free_keys.get().into_iter().map(|d| view! { <option value=d.key.clone()>{d.key.clone()}</option> }).collect_view()}
                        </select>
                        {move || {
                            let t = value_type.get();
                            // A new date-time starts at now, so Enter alone accepts it
                            let initial = if t == "datetime" { dom::local_minutes_to_utc(&dom::now_local_minutes()) } else { String::new() };
                            // The new input takes the focus, so a click elsewhere leaves it and saves
                            let input_ref = NodeRef::<leptos::html::Input>::new();
                            dom::after_render(move || {
                                if let Some(input) = input_ref.try_get_untracked().flatten() {
                                    let _ = input.focus();
                                }
                            });
                            (!t.is_empty()).then(|| value_input(&t, Signal::derive(move || initial.clone()), commit, true, input_ref))
                        }}
                        // mousedown is stopped, so the input keeps the focus and its blur does
                        // not save the row that is being discarded
                        <button class="property-remove" title="Remove" on:mousedown=|ev| ev.prevent_default() on:click=discard>"×"</button>
                    </div>
                }
            } />
            <button class="property-add" title="Add property" on:click=add_draft>"+"</button>
        </div>
    }
}

/// The input of a value, by the type of its key. Text-like types save on change (blur or
/// Enter); a checkbox saves on every click. A new row (`is_draft`) also saves a value that
/// was not changed, on Enter and when the focus leaves, so an initial value can be accepted
/// as it is; an empty value is not saved.
fn value_input(
    value_type: &str,
    value: Signal<String>,
    save: impl Fn(String) + Clone + 'static,
    is_draft: bool,
    input_ref: NodeRef<leptos::html::Input>,
) -> AnyView {
    // Enter and blur of a draft: the value as typed, when there is one
    let on_leave = {
        let save = save.clone();
        move |typed: String| {
            if is_draft && !typed.is_empty() {
                save(typed);
            }
        }
    };
    let on_enter = {
        let on_leave = on_leave.clone();
        move |ev: web_sys::KeyboardEvent| {
            if is_draft && ev.key() == "Enter" && !dom::is_ime_composing(&ev) {
                ev.prevent_default();
                on_leave(event_target_value(&ev));
            }
        }
    };
    let on_blur = {
        let on_leave = on_leave.clone();
        move |ev: web_sys::FocusEvent| on_leave(event_target_value(&ev))
    };
    match value_type {
        "checkbox" => view! {
            <input
                node_ref=input_ref
                type="checkbox"
                class="property-checkbox"
                prop:checked=move || value.get() == "true"
                on:change=move |ev| save(if event_target_checked(&ev) { "true".into() } else { "false".into() })
            />
        }
        .into_any(),
        // Shown in local time to the minute; sent as UTC
        "datetime" => {
            let to_utc = {
                let save = save.clone();
                move |local: String| {
                    let utc = dom::local_minutes_to_utc(&local);
                    if !utc.is_empty() {
                        save(utc);
                    }
                }
            };
            let on_leave_utc = {
                let to_utc = to_utc.clone();
                move |local: String| {
                    if is_draft && !local.is_empty() {
                        to_utc(local);
                    }
                }
            };
            let on_enter_utc = {
                let on_leave_utc = on_leave_utc.clone();
                move |ev: web_sys::KeyboardEvent| {
                    if is_draft && ev.key() == "Enter" && !dom::is_ime_composing(&ev) {
                        ev.prevent_default();
                        on_leave_utc(event_target_value(&ev));
                    }
                }
            };
            let on_blur_utc = {
                let on_leave_utc = on_leave_utc.clone();
                move |ev: web_sys::FocusEvent| on_leave_utc(event_target_value(&ev))
            };
            view! {
                <input
                    node_ref=input_ref
                    type="datetime-local"
                    class="property-value"
                    prop:value=move || dom::local_minutes(&value.get())
                    on:change=move |ev| to_utc(event_target_value(&ev))
                    on:keydown=on_enter_utc
                    on:blur=on_blur_utc
                />
            }
            .into_any()
        }
        other => {
            let (input_type, placeholder) = match other {
                "number" => ("number", ""),
                "date" => ("date", ""),
                "tags" | "list" => ("text", "a, b"),
                _ => ("text", ""),
            };
            view! {
                <input
                    node_ref=input_ref
                    type=input_type
                    class="property-value"
                    placeholder=placeholder
                    prop:value=move || value.get()
                    on:change=move |ev| save(event_target_value(&ev))
                    on:keydown=on_enter
                    on:blur=on_blur
                />
            }
            .into_any()
        }
    }
}
