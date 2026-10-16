//! The settings screen, ported from `settings.ts`.

use crate::core::CoreHandle;
use leptos::prelude::*;
use peta_core::model::WeekNumbering;
use peta_core::Event;

#[component]
pub fn Settings() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let config = core.select(|v| v.config.clone());

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
            </div>
        </app-settings>
    }
}
