//! The checkbox in front of a task item, shown by the item view and the editor alike.

use crate::body::Row;
use crate::core::CoreHandle;
use client_core::view::TaskState;
use client_core::Event;
use leptos::prelude::*;

#[component]
pub fn TaskCheckbox(row: RwSignal<Row>) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let item_id = StoredValue::new(row.with_untracked(|r| r.item.id.clone()));
    let task = Memo::new(move |_| row.with(|r| r.task));
    let on_click = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        core.send(Event::TaskCycle {
            item_id: item_id.get_value(),
        });
    };
    view! {
        <Show when=move || task.get().is_some()>
            <input
                type="checkbox"
                class="task-checkbox"
                prop:checked=move || task.get() == Some(TaskState::Done)
                prop:indeterminate=move || task.get() == Some(TaskState::Running)
                on:click=on_click
            />
        </Show>
    }
}
