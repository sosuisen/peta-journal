//! Pastes into an item editor, ported from `paste.service.ts`. Text is split into items at
//! the same block boundaries as a markdown file, the first piece joins the text before the
//! cursor and the last piece the text after it; the pieces in between become new items
//! below. An image is uploaded and linked at the cursor.

use crate::app::{apply, done, log_error, run, Cmd, Event, Model};
use crate::effects::{Api, ApiError};
use crate::model::{Item, ItemPosition};
use crate::ops::edit::{discard_pending_edit, focus_editor, mark_dirty, save_body};
use crate::outline::CursorState;
use crate::util::{has_unclosed_code_fence, split_at_char};

/// A piece of pasted text that becomes an item; the level is relative to the item pasted into.
#[derive(Debug, Clone)]
struct PastedItem {
    body: String,
    level: i32,
}

/// Pastes text into an item. Supports plain text, tree markdown, and flat markdown that
/// was copied from this app.
pub fn paste_text(model: &mut Model, item_id: &str, text: String, _html: String, cursor: usize) -> Cmd {
    let Some(item) = model.current_item(item_id) else { return done() };
    let item: Item = (*item).clone();
    // If we have stored flat markdown for this clipboard content, use it
    let stored_flat = match &model.clipboard {
        Some((flat, tree)) if *tree == text => Some(flat.clone()),
        _ => None,
    };
    run(move |api| async move {
        if let Err(e) = paste_text_now(&api, item, text, cursor, stored_flat).await {
            log_error(format!("Failed to process paste: {e}"));
            apply(&api, "paste: abort undo", |m| {
                m.undo.abort();
                done()
            });
        }
    })
}

async fn paste_text_now(
    api: &Api<Event>,
    item: Item,
    text: String,
    cursor: usize,
    stored_flat: Option<String>,
) -> Result<(), ApiError> {
    let mut text_to_parse = text.clone();
    let mut is_plain_text = true;
    if let Some(flat) = stored_flat {
        text_to_parse = flat;
        is_plain_text = false;
    } else if text.trim_start().starts_with("- ") {
        // Text that looks like tree markdown is converted
        match api.import_tree_markdown(&text).await {
            Ok(flat) => {
                text_to_parse = flat;
                is_plain_text = false;
            }
            Err(e) => log_error(format!("Failed to import tree markdown, using raw text: {e}")),
        }
    }

    let (before, after) = split_at_char(&item.body, cursor);
    // Plain text pasted inside an open code fence stays in this item as it is
    let skip_split = is_plain_text && has_unclosed_code_fence(before);
    let mut parsed: Vec<PastedItem> = if skip_split {
        vec![PastedItem { body: text_to_parse, level: 0 }]
    } else {
        split_text_into_items(api, &text_to_parse).await?
    };
    if parsed.is_empty() {
        return Ok(());
    }

    parsed[0].body = format!("{before}{}", parsed[0].body);
    let last = parsed.len() - 1;
    parsed[last].body = format!("{}{after}", parsed[last].body);

    let original = item.clone();
    let (first_body, rest): (String, Vec<PastedItem>) = (parsed[0].body.clone(), parsed[1..].to_vec());
    let it = item.clone();
    apply(api, "paste: begin", move |m| {
        m.undo.begin("paste", Some(CursorState::new(&it.id, cursor)));
        // The auto-save of what was typed before the paste is dropped, or it would
        // overwrite the pasted text a moment later
        discard_pending_edit(m, &it)
    });
    save_body(api, &item.id, &first_body).await?;
    let id = item.id.clone();
    apply(api, "paste: first item", move |m| {
        mark_dirty(m, &id);
        if let Some(after_first) = m.state.get_item_by_id(&id) {
            m.undo.push_update(&original, &after_first);
        }
        done()
    });

    let mut prev_id = item.id.clone();
    for pasted in rest {
        let position = ItemPosition::after(prev_id.clone());
        let new_item = api.create_item(&item.page_id, position.clone(), item.level + pasted.level, &pasted.body, None).await?;
        prev_id = new_item.id.clone();
        apply(api, "paste: new item", move |m| {
            m.state.add_items(vec![new_item.clone()], &position);
            let final_item = m.state.get_item_by_id(&new_item.id).map(|b| (*b).clone()).unwrap_or(new_item);
            m.undo.push_create(&final_item);
            done()
        });
    }
    apply(api, "paste: commit", |m| {
        m.undo.commit(None, m.clock_ms);
        done()
    });
    // Focus the last created item or the current item
    focus_editor(api, None).await;
    Ok(())
}

/// Splits text into items with the server-side Flat Markdown parser, so that pasted text
/// is cut at the same block boundaries as a markdown file. Levels are made relative to the
/// first item (0-based), because the caller places them under the current item.
async fn split_text_into_items(api: &Api<Event>, text: &str) -> Result<Vec<PastedItem>, ApiError> {
    let parsed = api.parse_flat_markdown(text).await?;
    let Some(first) = parsed.first() else { return Ok(Vec::new()) };
    let top_level = first.level;
    Ok(parsed.iter().map(|p| PastedItem { body: p.body.clone(), level: (p.level - top_level).max(0) }).collect())
}

/// Uploads a pasted image and puts a link to it at the cursor.
pub fn paste_image(model: &mut Model, item_id: &str, file: u64, cursor: usize) -> Cmd {
    let Some(item) = model.current_item(item_id) else { return done() };
    let item: Item = (*item).clone();
    run(move |api| async move {
        let file_name = match api.upload_asset(file).await {
            Ok(name) => name,
            Err(e) => return log_error(format!("Failed to paste image: {e}")),
        };
        let markdown = format!("![{file_name}](../assets/{file_name})");
        let (before, after) = split_at_char(&item.body, cursor);
        let new_body = format!("{before}{markdown}{after}");
        let it = item.clone();
        apply(&api, "paste image: discard pending", move |m| discard_pending_edit(m, &it));
        if let Err(e) = save_body(&api, &item.id, &new_body).await {
            return log_error(format!("Failed to paste image: {e}"));
        }
        let new_pos = cursor + crate::util::char_len(&markdown);
        let id = item.id.clone();
        apply(&api, "paste image: cursor", move |m| {
            mark_dirty(m, &id);
            m.state.set_focused_item(&id, Some(new_pos));
            done()
        });
        focus_editor(&api, Some(item.id.clone())).await;
    })
}

