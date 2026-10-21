//! The settings screen, ported from `settings.ts`.

use crate::core::CoreHandle;
use leptos::prelude::*;
use peta_core::model::{PropertyDefinition, WeekNumbering, USER_VALUE_TYPES};
use peta_core::Event;

#[component]
pub fn Settings() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let config = core.select(|v| v.config.clone());
    let definitions = core.select(|v| v.config.property_definitions.clone());
    let definition_error = core.select(|v| v.config.definition_error.clone());

    let new_key = RwSignal::new(String::new());
    let new_type = RwSignal::new(USER_VALUE_TYPES[0].to_string());
    // The form is cleared once the list holds the key, i.e. when the server accepted it
    let submitted = RwSignal::new(Option::<String>::None);
    Effect::new(move |_| {
        let Some(key) = submitted.get() else { return };
        if definitions.with(|d| d.iter().any(|def| def.key == key)) {
            new_key.set(String::new());
            submitted.set(None);
        }
    });
    let add = move || {
        let key = new_key.get_untracked().trim().to_string();
        submitted.set(Some(key.clone()));
        core.send(Event::AddPropertyDefinition {
            key,
            value_type: new_type.get_untracked(),
        });
    };

    view! {
        <app-settings>
            <div class="settings">
                <h2>"Settings"</h2>
                <div class="setting-group">
                    <label class="setting-label">"Markdown working directory"</label>
                    <div class="input-with-button">
                        <input type="text" class="setting-input" prop:value=move || config.with(|c| c.markdown_dir.clone()) readonly />
                        <button class="browse-btn" on:click=move |_| core.send(Event::BrowseFolder)>"Browse..."</button>
                    </div>
                    <p class="setting-description">"Changes require a server restart to take effect."</p>
                </div>
                <div class="setting-group">
                    <label class="setting-label">"Week numbering"</label>
                    <select
                        class="setting-select"
                        prop:value=move || config.with(|c| c.week_numbering.as_str().to_string())
                        on:change=move |ev| core.send(Event::SetWeekNumbering(WeekNumbering::parse(&event_target_value(&ev))))
                    >
                        <option value="us">"US (Sunday start)"</option>
                        <option value="iso">"ISO (Monday start)"</option>
                    </select>
                </div>
                <div class="setting-group">
                    <label class="setting-checkbox">
                        <input
                            type="checkbox"
                            prop:checked=move || config.with(|c| c.logical_outdent)
                            on:change=move |ev| core.send(Event::SetLogicalOutdent(event_target_checked(&ev)))
                        />
                        " Logical outdent"
                    </label>
                    <p class="setting-description">"When outdenting, do not break parent-child relationships."</p>
                </div>
                <div class="setting-group">
                    <label class="setting-label">"Tag pages directory"</label>
                    <input
                        type="text"
                        class="setting-input"
                        placeholder="tags/"
                        prop:value=move || config.with(|c| c.tags_dir.clone())
                        on:change=move |ev| core.send(Event::SetTagsDir(event_target_value(&ev)))
                    />
                    <p class="setting-description">"A hashtag such as #topic links to the page topic under this directory. Leave it empty to link to the pages root."</p>
                </div>
                <div class="setting-group">
                    <label class="setting-label">"Properties"</label>
                    <table class="property-definitions">
                        <thead>
                            <tr><th>"Key"</th><th>"Type"</th><th>"Page"</th><th></th></tr>
                        </thead>
                        <tbody>
                            <For each=move || definitions.get() key=|d| d.key.clone() children=definition_row />
                        </tbody>
                    </table>
                    <form class="property-definition-form" on:submit=move |ev| { ev.prevent_default(); add(); }>
                        <input
                            type="text"
                            class="setting-input"
                            placeholder="new key"
                            prop:value=move || new_key.get()
                            on:input=move |ev| new_key.set(event_target_value(&ev))
                        />
                        <select
                            class="setting-select"
                            prop:value=move || new_type.get()
                            on:change=move |ev| new_type.set(event_target_value(&ev))
                        >
                            {USER_VALUE_TYPES.iter().map(|t| view! { <option value=*t>{*t}</option> }).collect_view()}
                        </select>
                        <button type="submit" class="browse-btn">"Add"</button>
                    </form>
                    {move || definition_error.get().map(|e| view! { <p class="setting-error">{e}</p> })}
                    <p class="setting-description">"System keys are managed by the app and cannot be defined again. A key can be used on items and, when marked, on pages."</p>
                </div>
            </div>
        </app-settings>
    }
}

/// One definition: a system key is marked as such instead of offering any control.
fn definition_row(d: PropertyDefinition) -> impl IntoView {
    view! {
        <tr class:system-key=d.system>
            <td>{d.key.clone()}</td>
            <td>{d.value_type.clone()}</td>
            <td>{if d.page { "yes" } else { "" }}</td>
            <td class="definition-note">{if d.system { "system" } else { "" }}</td>
        </tr>
    }
}
