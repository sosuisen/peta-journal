//! The confirmation dialog of `UiOp::ConfirmDestructive`: the browser's `confirm` answers OK
//! to Enter, so a deletion that is hard to undo gets a dialog whose Cancel has the focus.

use crate::core::CoreHandle;
use crate::effects::ui::answer_confirm;
use leptos::prelude::*;

/// Whether an event comes from inside the dialog. The outline's selection listens on the
/// document and leaves such events alone: Escape or a click there answers the dialog, and must
/// not also end the selection the dialog asks about. Leptos hands its own handlers the events
/// at the document too, so stopping them in the dialog would not keep them from the outline.
pub fn is_inside(ev: &web_sys::Event) -> bool {
    crate::dom::target_closest(ev, ".confirm-overlay").is_some()
}

#[component]
pub fn ConfirmDialog() -> impl IntoView {
    let core: CoreHandle = expect_context();
    let message = core.inner().confirm_message;
    let cancel = NodeRef::<leptos::html::Button>::new();
    Effect::new(move |_| {
        if message.get().is_some() {
            if let Some(button) = cancel.get() {
                let _ = button.focus();
            }
        }
    });
    let on_key = move |e: web_sys::KeyboardEvent| {
        if e.key() == "Escape" {
            e.prevent_default();
            answer_confirm(core, false);
        }
    };
    view! {
        <Show when=move || message.get().is_some()>
            <div class="confirm-overlay" on:keydown=on_key>
                <div class="confirm-dialog" role="alertdialog" aria-modal="true">
                    <p class="confirm-message">{move || message.get().unwrap_or_default()}</p>
                    <div class="confirm-buttons">
                        <button type="button" node_ref=cancel on:click=move |_| answer_confirm(core, false)>"Cancel"</button>
                        <button type="button" class="confirm-ok" on:click=move |_| answer_confirm(core, true)>"OK"</button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
