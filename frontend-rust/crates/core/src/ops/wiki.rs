//! The popup with page names that match the text typed inside a `[[wiki link]]`, ported
//! from `wiki-suggestions.ts`. The outline feeds it the query from the editor and inserts
//! the pick into the body.

use crate::app::{apply, cancel_timer, done, log_error, run, Cmd, Model, WikiLinkContext};
use crate::effects::TimerOutput;
use crate::ops::edit::record_typing;
use crate::util::char_len;
use crux_core::Command;

/// How long typing may pause before the page names are searched.
pub const SEARCH_DELAY_MS: u32 = 150;

/// True when the popup is open for this item.
pub fn is_shown_for(model: &Model, item_id: &str) -> bool {
    model.wiki.item_id.as_deref() == Some(item_id) && !model.wiki.suggestions.is_empty()
}

/// Typing inside `[[ ]]`: positions the popup and searches after a pause. None closes it.
pub fn query(model: &mut Model, item_id: &str, context: Option<WikiLinkContext>) -> Cmd {
    let Some(ctx) = context else {
        return close(model);
    };
    model.wiki.item_id = Some(item_id.to_string());
    model.wiki.x = ctx.x;
    model.wiki.y = ctx.y + 4.0;
    model.wiki.query = ctx.query.clone();
    if ctx.query.is_empty() {
        model.wiki.suggestions.clear();
        return done();
    }
    let cancel = cancel_timer(&mut model.wiki.timer_id);
    let timer_id = model.next_timer_id();
    model.wiki.timer_id = timer_id;
    let q = ctx.query;
    let search = run(move |api| async move {
        if api.sleep(timer_id, SEARCH_DELAY_MS).await != TimerOutput::Fired {
            return;
        }
        let titles = match api.search_page_names(&q, 10).await {
            Ok(results) => results.into_iter().map(|r| r.page_title).collect(),
            Err(e) => {
                log_error(format!("Wiki link search failed: {e}"));
                Vec::new()
            }
        };
        apply(&api, "wiki suggestions", move |m| {
            if m.wiki.timer_id == timer_id && m.wiki.item_id.is_some() {
                m.wiki.suggestions = titles;
                m.wiki.index = 0;
            }
            done()
        });
    });
    Command::all([cancel, search])
}

/// ArrowUp, ArrowDown or Enter while the popup is open.
pub fn key_nav(model: &mut Model, key: &str) -> Cmd {
    let n = model.wiki.suggestions.len();
    if n == 0 {
        return done();
    }
    match key {
        "ArrowDown" => {
            model.wiki.index = (model.wiki.index + 1) % n;
            done()
        }
        "ArrowUp" => {
            model.wiki.index = (model.wiki.index + n - 1) % n;
            done()
        }
        "Enter" => {
            let title = model.wiki.suggestions[model.wiki.index].clone();
            pick(model, &title)
        }
        _ => done(),
    }
}

/// Replaces the wiki link query text (between `[[` and `]]`) with the picked title and
/// puts the caret after it.
pub fn pick(model: &mut Model, title: &str) -> Cmd {
    let Some(item_id) = model.wiki.item_id.clone() else {
        return done();
    };
    let Some(item) = model.current_item(&item_id) else {
        return close(model);
    };
    let body = item.body.clone();
    let query_len = char_len(&model.wiki.query);
    let close_cmd = close(model);
    // The query sits right before `]]`: find the `[[` that opens it
    let Some(close_idx) = find_close_after_query(&body, &model_query_hint(query_len)) else {
        return close_cmd;
    };
    let open_idx = match body[..close_idx].rfind("[[") {
        Some(i) => i,
        None => return close_cmd,
    };
    let new_body = format!("{}{}{}", &body[..open_idx + 2], title, &body[close_idx..]);
    let new_cursor = char_len(&body[..open_idx + 2]) + char_len(title);
    let item: crate::model::Item = (*item).clone();
    let typing = record_typing(model, &item, &new_body);
    model.state.set_focused_item(&item.id, Some(new_cursor));
    Command::all([close_cmd, typing])
}

/// The byte index of the `]]` that closes the link being typed: the first `]]` that follows
/// an unclosed `[[`.
fn find_close_after_query(body: &str, _hint: &usize) -> Option<usize> {
    let mut search_from = 0;
    while let Some(open_rel) = body[search_from..].find("[[") {
        let open = search_from + open_rel;
        let close = open + 2 + body[open + 2..].find("]]")?;
        // A link with another `[[` inside it belongs to the inner `[[`
        if body[open + 2..close].contains("[[") {
            search_from = open + 2;
            continue;
        }
        return Some(close);
    }
    None
}

fn model_query_hint(query_len: usize) -> usize {
    query_len
}

pub fn close(model: &mut Model) -> Cmd {
    model.wiki.suggestions.clear();
    model.wiki.item_id = None;
    model.wiki.query.clear();
    cancel_timer(&mut model.wiki.timer_id)
}
