//! The property rows between the title and the body of a doc page: one row per property,
//! `+` adds a row whose key is picked from the definitions, `×` removes a property.
//! Date-times are shown and typed in local time to the minute and sent as UTC.

use crate::core::CoreHandle;
use crate::dom;
use client_page_links::hashtag_path;
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
                let shown: AnyView = if p.is_list {
                    let remove_last = { let key = key.clone(); move || core.send(Event::DeletePageProperty { key: key.clone() }) };
                    view! { <TagList value=value.into() save=send remove_last=remove_last /> }.into_any()
                } else if p.editable {
                    value_input(&value_type, value.into(), send, false, NodeRef::new())
                } else {
                    // The same input as an editable date-time, so the browser shows both dates alike
                    view! { <input type="datetime-local" class="property-value property-readonly" readonly prop:value=move || dom::local_minutes(&value.get()) /> }.into_any()
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
                            if t == "tags" || t == "list" {
                                // The row goes with its last tag, as a saved list does
                                let discard = move || drafts.update(|ds| ds.retain(|x| x.id != id));
                                return Some(view! { <TagList value=Signal::derive(String::new) save=commit remove_last=discard open=true /> }.into_any());
                            }
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

/// A list value as tag chips: a chip opens its tag page, its `×` removes the tag, and `+`
/// opens an input that adds the typed tags on Enter or when the focus leaves. A new row
/// (`open`) starts with that input open. A chip dragged onto another one moves the tag
/// before or after it.
#[component]
fn TagList(
    value: Signal<String>,
    save: impl Fn(String) + Clone + Send + Sync + 'static,
    remove_last: impl Fn() + Clone + Send + Sync + 'static,
    #[prop(optional)] open: bool,
) -> impl IntoView {
    let core: CoreHandle = expect_context();
    // Tag pages live where hashtags link to, so both reach the same page
    let tags_dir = core.select(|v| v.config.tags_dir.clone());
    let tags = Memo::new(move |_| {
        value.with(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .collect::<Vec<String>>()
        })
    });
    let adding = RwSignal::new(false);
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let open_input = move || {
        adding.set(true);
        dom::after_render(move || {
            if let Some(input) = input_ref.try_get_untracked().flatten() {
                let _ = input.focus();
            }
        });
    };
    if open {
        open_input();
    }

    let remove = {
        let save = save.clone();
        move |tag: String| {
            let rest: Vec<String> = tags
                .get_untracked()
                .into_iter()
                .filter(|t| *t != tag)
                .collect();
            // An empty list would stay as a row with no tags, so the property goes instead
            if rest.is_empty() {
                remove_last();
            } else {
                save(rest.join(", "));
            }
        }
    };
    // The tag being dragged, and the chip it would land at with whether it goes after it
    let dragged = RwSignal::new(None::<String>);
    let drop_at = RwSignal::new(None::<(String, bool)>);
    let end_drag = move || {
        dragged.set(None);
        drop_at.set(None);
    };
    let reorder = {
        let save = save.clone();
        move |target: String, after: bool| {
            let Some(moved) = dragged.get_untracked() else {
                return;
            };
            let old = tags.get_untracked();
            let mut new: Vec<String> = old.iter().filter(|t| **t != moved).cloned().collect();
            let Some(at) = new.iter().position(|t| *t == target) else {
                return;
            };
            new.insert(if after { at + 1 } else { at }, moved);
            // A drop back where the tag was would build the page again for nothing
            if new != old {
                save(new.join(", "));
            }
        }
    };
    let add = move |typed: String| {
        // Enter and Escape take the input away, and its blur then comes too late to add again
        if !adding.get_untracked() {
            return;
        }
        adding.set(false);
        // The core drops a leading `#` and a tag that is already there
        if !typed.trim().is_empty() {
            save(format!("{}, {typed}", value.get_untracked()));
        }
    };

    view! {
        <div class="property-tags">
            <For each=move || tags.get() key=|t| t.clone() children=move |tag| {
                let href = hashtag_path(&tags_dir.get_untracked(), &tag);
                let open = {
                    let href = href.clone();
                    move |ev: web_sys::MouseEvent| {
                        // A modified click is left to the browser, so it opens a new tab or window
                        if ev.ctrl_key() || ev.meta_key() || ev.shift_key() {
                            return;
                        }
                        ev.prevent_default();
                        core.navigate(&href, false);
                    }
                };
                let remove = remove.clone();
                let removed = tag.clone();
                let reorder = reorder.clone();
                let start_tag = tag.clone();
                let over_tag = tag.clone();
                let before_tag = tag.clone();
                let after_tag = tag.clone();
                view! {
                    <span
                        class="property-tag"
                        draggable="true"
                        class:dragging={
                            let tag = tag.clone();
                            move || dragged.with(|d| d.as_ref() == Some(&tag))
                        }
                        class:drop-before=move || drop_at.with(|d| d.as_ref() == Some(&(before_tag.clone(), false)))
                        class:drop-after=move || drop_at.with(|d| d.as_ref() == Some(&(after_tag.clone(), true)))
                        on:dragstart=move |ev: web_sys::DragEvent| {
                            if let Some(dt) = ev.data_transfer() {
                                // Firefox starts no drag without data
                                let _ = dt.set_data("text/plain", &format!("#{start_tag}"));
                                dt.set_effect_allowed("move");
                            }
                            dragged.set(Some(start_tag.clone()));
                        }
                        on:dragover=move |ev: web_sys::DragEvent| {
                            // Text or a card dragged from elsewhere is not a tag of this list
                            if dragged.with_untracked(|d| d.is_none()) {
                                return;
                            }
                            ev.prevent_default();
                            if let Some(dt) = ev.data_transfer() {
                                dt.set_drop_effect("move");
                            }
                            let after = drop_side(&ev);
                            let at = Some((over_tag.clone(), after));
                            if drop_at.with_untracked(|d| *d != at) {
                                drop_at.set(at);
                            }
                        }
                        on:drop={
                            let tag = tag.clone();
                            move |ev: web_sys::DragEvent| {
                                if dragged.with_untracked(|d| d.is_none()) {
                                    return;
                                }
                                ev.prevent_default();
                                reorder(tag.clone(), drop_side(&ev));
                                end_drag();
                            }
                        }
                        // The next dragover shows the mark again when the pointer only moved onto the link
                        on:dragleave=move |_| drop_at.set(None)
                        on:dragend=move |_| end_drag()
                    >
                        // The link would drag its URL instead of the chip
                        <a class="property-tag-link" href=href draggable="false" on:click=open>{format!("#{tag}")}</a>
                        <button class="property-tag-remove" title="Remove tag" on:click=move |_| remove(removed.clone())>"×"</button>
                    </span>
                }
            } />
            {move || if adding.get() {
                let add_on_enter = add.clone();
                let add_on_blur = add.clone();
                view! {
                    <input
                        node_ref=input_ref
                        type="text"
                        class="property-tag-input"
                        placeholder="tag"
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if dom::is_ime_composing(&ev) {
                                return;
                            }
                            match ev.key().as_str() {
                                "Enter" => {
                                    ev.prevent_default();
                                    add_on_enter(event_target_value(&ev));
                                }
                                "Escape" => adding.set(false),
                                _ => {}
                            }
                        }
                        on:blur=move |ev| add_on_blur(event_target_value(&ev))
                    />
                }.into_any()
            } else {
                view! { <button class="property-tag-add" title="Add tag" on:click=move |_| open_input()>"+"</button> }.into_any()
            }}
        </div>
    }
}

/// Whether a tag dropped at the pointer goes after the chip under it: it does on the right
/// half of the chip.
fn drop_side(ev: &web_sys::DragEvent) -> bool {
    let Some(chip) = dom::target_closest(ev, ".property-tag") else {
        return false;
    };
    let rect = chip.get_bounding_client_rect();
    ev.client_x() as f64 > rect.left() + rect.width() / 2.0
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
        // Shown in local time to the minute; sent as UTC. It is saved only when the focus
        // leaves or on Enter, not on change. A save builds the page again, and that would
        // cut off the picker after its first click.
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
                    let changed = local != dom::local_minutes(&value.get_untracked());
                    if !local.is_empty() && (is_draft || changed) {
                        to_utc(local);
                    }
                }
            };
            let on_enter_utc = {
                let on_leave_utc = on_leave_utc.clone();
                move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" && !dom::is_ime_composing(&ev) {
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
                "bounds" => ("text", "x, y, width, height"),
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
