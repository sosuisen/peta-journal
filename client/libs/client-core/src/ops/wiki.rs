//! The popup with page names that match the text typed inside a `[[wiki link]]`, ported
//! from `wiki-suggestions.ts`. The outline feeds it the query from the editor and inserts
//! the pick into the body.

use crate::app::{cancel_timer, done, log_error, run, Cmd, Event, Model, WikiLinkContext};
use crate::effects::TimerOutput;
use crate::ops::edit::record_typing;
use crate::screens::navigate::{open_page, to_page};
use crate::util::{char_len, link_at, Link};
use crux_core::Command;

/// How long typing may pause before the page names are searched.
pub const SEARCH_DELAY_MS: u32 = 150;

/// The text typed so far inside the `[[` the cursor is in, which the popup searches for.
/// `cursor` counts characters (ADR 043). None when the cursor is in no link: no `[[` before
/// it, a `]]` between that `[[` and it, or no `]]` after it.
pub fn query_at(body: &str, cursor: usize) -> Option<String> {
    let at = body
        .char_indices()
        .nth(cursor)
        .map_or(body.len(), |(byte, _)| byte);
    let (before, after) = body.split_at(at);
    let open = before.rfind("[[")?;
    let between = &before[open + 2..];
    (!between.contains("]]") && after.contains("]]")).then(|| between.to_string())
}

/// Opens the page of the wiki link or the tag at the cursor. Off a link, nothing happens.
pub fn open_link_at(model: &Model, item_id: &str, cursor: usize) -> Cmd {
    let link = model
        .state
        .get_item_by_id(item_id)
        .and_then(|item| link_at(&item.body, cursor));
    match link {
        Some(Link::Wiki(content)) => match page_urls::split_wikilink(&content) {
            (name, None) => open_page(name),
            (name, item) => to_page(name, false, item),
        },
        Some(Link::Tag(tag)) => open_page(&page_urls::tag_page_name(&model.config.tags_dir, &tag)),
        None => done(),
    }
}

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
        api.send_event(Event::WikiLinksSuggested { timer_id, titles });
    });
    Command::all([cancel, search])
}

/// The page names the search the timer `timer_id` started found. They are not shown when a
/// later search started or the popup closed.
pub fn show_suggestions(model: &mut Model, timer_id: u64, titles: Vec<String>) -> Cmd {
    if model.wiki.timer_id == timer_id && model.wiki.item_id.is_some() {
        model.wiki.suggestions = titles;
        model.wiki.index = 0;
    }
    done()
}

/// ArrowUp, ArrowDown or Enter while the popup is open.
pub fn key_nav(model: &mut Model, key: &str) -> Cmd {
    if key == "Escape" {
        return close(model);
    }
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

/// True for a key that the open popup takes from the editor: the arrows up and down pick a
/// name, Enter puts it in, and Escape closes the popup. The key is named as the browser
/// names it.
pub fn takes_key(key: &str) -> bool {
    matches!(key, "ArrowUp" | "ArrowDown" | "Enter" | "Escape")
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
    let close_cmd = close(model);
    // The query sits right before `]]`: find the `[[` that opens it
    let Some((open_idx, close_idx)) = link_being_typed(&body) else {
        return close_cmd;
    };
    let new_body = format!("{}{}{}", &body[..open_idx + 2], title, &body[close_idx..]);
    let new_cursor = char_len(&body[..open_idx + 2]) + char_len(title);
    let item: crate::model::Item = (*item).clone();
    let typing = record_typing(model, &item, &new_body);
    model.state.set_focused_item(&item.id, Some(new_cursor));
    Command::all([close_cmd, typing])
}

/// The byte indexes of the `[[` and the `]]` of the link being typed: the first `]]` that
/// follows an unclosed `[[`.
fn link_being_typed(body: &str) -> Option<(usize, usize)> {
    let mut search_from = 0;
    while let Some(open_rel) = body[search_from..].find("[[") {
        let open = search_from + open_rel;
        let close = open + 2 + body[open + 2..].find("]]")?;
        // A link with another `[[` inside it belongs to the inner `[[`
        if body[open + 2..close].contains("[[") {
            search_from = open + 2;
            continue;
        }
        return Some((open, close));
    }
    None
}

pub fn close(model: &mut Model) -> Cmd {
    model.wiki.suggestions.clear();
    model.wiki.item_id = None;
    model.wiki.query.clear();
    cancel_timer(&mut model.wiki.timer_id)
}
