//! Extension functions, ported from `extension-function.service.ts`: hidden iframes the
//! shell hosts, which create items through `postMessage`.

use crate::app::{apply, done, log_error, run, Cmd, Model};
use crate::effects::UiOp;
use crate::model::ItemPosition;
use crate::outline::tree::can_place;
use serde_json::json;

/// Loads the registered functions and asks the shell to host them.
pub fn load(model: &mut Model) -> Cmd {
    if model.extension_functions_loaded {
        return done();
    }
    model.extension_functions_loaded = true;
    run(|api| async move {
        match api.get_functions().await {
            Ok(functions) => api.notify(UiOp::LoadExtensions { functions }),
            Err(e) => log_error(format!("Failed to load extension functions: {e}")),
        }
    })
}

/// A `CreateItem` message from an extension: creates the item where it asked, with the
/// level lowered until it fits, and answers with `CreateItemResult` or `CreateItemError`.
#[allow(clippy::too_many_arguments)]
pub fn create_item(
    model: &mut Model,
    source: u64,
    request_id: Option<String>,
    page_id: String,
    position: Option<String>,
    after_item_id: Option<String>,
    mut level: i32,
    body: String,
) -> Cmd {
    // Support the `position` field, fall back to the legacy `afterItemId`
    let position = match (position.and_then(|p| ItemPosition::parse(&p)), after_item_id) {
        (Some(p), _) => p,
        (None, Some(after)) => ItemPosition::after(after),
        (None, None) => ItemPosition::First,
    };
    let items = model.state.items();
    let prev = match &position {
        ItemPosition::After(after_id) => items.iter().find(|b| &b.id == after_id).cloned(),
        ItemPosition::Last => items.last().cloned(),
        ItemPosition::First => None,
    };
    while level > 1 && !can_place(level, prev.as_deref()) {
        level -= 1;
    }
    run(move |api| async move {
        match api.create_item(&page_id, position.clone(), level, &body, None).await {
            Ok(new_item) => {
                let reply = json!({ "type": "CreateItemResult", "itemId": new_item.id, "requestId": request_id });
                apply(&api, "extension item", move |m| {
                    m.state.add_item(new_item, &position);
                    done()
                });
                api.notify(UiOp::ExtensionReply { source, message: reply });
            }
            Err(e) => {
                log_error(format!("Failed to create item for extension: {e}"));
                let reply = json!({ "type": "CreateItemError", "error": e.0, "requestId": request_id });
                api.notify(UiOp::ExtensionReply { source, message: reply });
            }
        }
    })
}
