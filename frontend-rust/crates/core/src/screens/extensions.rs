//! Extension functions, ported from `extension-function.service.ts`: hidden iframes the
//! shell hosts, which create items and ask the user through `postMessage`.

use crate::app::{apply, done, log_error, run, Cmd, Event, Model};
use crate::effects::{Api, ApiError, UiOp};
use crate::model::{ItemPosition, Page};
use crate::ops::file_reload::reload_shown_page;
use crate::outline::tree::can_place;
use crux_core::Command;
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
/// With `transclude`, the new item then transcludes that item. When the item would become
/// the last line after a lone empty one, it overwrites that empty item instead.
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
    transclude: Option<String>,
) -> Cmd {
    // Support the `position` field, fall back to the legacy `afterItemId`. An empty string
    // counts as absent: auto-template sends `afterItemId: ''` for its first item, and the
    // Angular service treated that as `first` (an empty string is falsy in JavaScript).
    let non_empty = |s: Option<String>| s.filter(|s| !s.is_empty());
    let position = match (
        non_empty(position).and_then(|p| ItemPosition::parse(&p)),
        non_empty(after_item_id),
    ) {
        (Some(p), _) => p,
        (None, Some(after)) => ItemPosition::after(after),
        (None, None) => ItemPosition::First,
    };
    let transclude = non_empty(transclude);
    if let Some(empty_id) = lone_empty_item_before(model, &page_id, &position) {
        // The overwritten item is the first one, where only the top level fits
        return run(move |api| async move {
            let result = api
                .update_item(&empty_id, Some(body), Some(1), None)
                .await
                .map(|updated| {
                    apply(&api, "extension item over the empty one", move |m| {
                        m.state.replace_item(&updated);
                        done()
                    });
                    empty_id
                });
            answer_created(&api, source, request_id, result, transclude).await;
        });
    }
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
        let result = api
            .create_item(&page_id, position.clone(), level, &body, None)
            .await
            .map(|new_item| {
                let new_id = new_item.id.clone();
                apply(&api, "extension item", move |m| {
                    m.state.add_item(new_item, &position);
                    done()
                });
                new_id
            });
        answer_created(&api, source, request_id, result, transclude).await;
    })
}

/// The id of the only item of the shown page when it is empty and a new item goes last.
/// A new page starts with one empty item, and a template placed after it would leave an
/// empty first line, so the template's first item takes its place instead.
/// An item placed after a given item does not take its place. The next items of a template
/// go after the first one, which stays empty when its transclusion fails; they would take
/// its place one after another.
fn lone_empty_item_before(model: &Model, page_id: &str, position: &ItemPosition) -> Option<String> {
    if model.shown_page_id().as_deref() != Some(page_id) {
        return None;
    }
    let items = model.state.items();
    let [only] = items else {
        return None;
    };
    // The body as shown: an edit not saved yet makes the item not empty
    let item = model.current_item(&only.id)?;
    let empty = item.body.trim().is_empty() && item.transclude_item_id.is_none();
    (*position == ItemPosition::Last && empty).then(|| only.id.clone())
}

/// Transcludes into the created item when asked, then answers the extension with
/// `CreateItemResult` or `CreateItemError`.
async fn answer_created(
    api: &Api<Event>,
    source: u64,
    request_id: Option<String>,
    result: Result<String, ApiError>,
    transclude: Option<String>,
) {
    let reply = match result {
        Ok(item_id) => {
            if let Some(source_id) = transclude {
                set_transclusion(api, &item_id, &source_id).await;
            }
            json!({ "type": "CreateItemResult", "itemId": item_id, "requestId": request_id })
        }
        Err(e) => {
            log_error(format!("Failed to create item for extension: {e}"));
            json!({ "type": "CreateItemError", "error": e.0, "requestId": request_id })
        }
    };
    api.notify(UiOp::ExtensionReply {
        source,
        message: reply,
    });
}

/// The item stays a plain one when the transclusion fails: the extension has its item
/// either way, and only the link to the source is missing.
async fn set_transclusion(api: &Api<Event>, item_id: &str, source_id: &str) {
    match api.set_transclusion(item_id, source_id).await {
        Ok(updated) => {
            apply(api, "extension transclusion", move |m| {
                m.state.replace_item(&updated);
                done()
            });
            // The source tree of the transclusion comes with the page only
            reload_shown_page(api).await;
        }
        // Told, not only logged: a template with a wrong id would leave an empty item unseen
        Err(e) => api.notify(UiOp::Alert(format!(
            "Failed to transclude the item {source_id}: {e}"
        ))),
    }
}

/// A `Confirm` message from an extension. The extension cannot ask by itself: it runs in a
/// hidden iframe of another origin, where browsers block `window.confirm`.
pub fn confirm(source: u64, request_id: Option<String>, message: String) -> Cmd {
    run(move |api| async move {
        let ok = api.confirm(&message).await;
        api.notify(UiOp::ExtensionReply {
            source,
            message: json!({ "type": "ConfirmResult", "ok": ok, "requestId": request_id }),
        });
    })
}

/// The extensions hear of the tags added to the tags property of a page. Nothing is sent
/// when none was added.
pub fn tags_added(page: &Page, tags: Vec<String>) -> Cmd {
    if tags.is_empty() {
        return done();
    }
    Command::notify_shell(UiOp::ExtensionPageTagAdd {
        page_id: page.id.clone(),
        file_name: page.name.clone(),
        dir_name: page.dir_path.clone(),
        page_type: page.page_type.clone(),
        tags,
    })
    .into()
}
