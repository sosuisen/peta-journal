//! The server's event stream, ported from `sse.service.ts`: connection state, reconnection
//! with exponential backoff, and what a `pagesUpdated` event triggers.

use crate::app::{apply, done, run, Cmd, ConnectionState, Event, Model};
use crate::effects::{SseEvent, SseOp, TimerOutput};
use crate::ops::file_reload;
use crate::screens::{atlas, page, sidebar};
use crux_core::Command;

const MAX_RECONNECT_ATTEMPTS: u32 = 10;

/// Opens the stream; every event on it becomes [`Event::Sse`].
pub fn connect(model: &mut Model) -> Cmd {
    model.connection = ConnectionState::Connecting;
    Command::stream_from_shell(SseOp::Connect).then_send(Event::Sse)
}

pub fn on_event(model: &mut Model, event: SseEvent) -> Cmd {
    match event {
        SseEvent::Open => {
            model.connection = ConnectionState::Connected;
            model.sse_reconnect_attempts = 0;
            sidebar::refresh_recent_pages()
        }
        SseEvent::Message(data) => on_message(model, &data),
        SseEvent::Error => {
            model.connection = ConnectionState::Error;
            schedule_reconnect(model)
        }
    }
}

fn on_message(model: &mut Model, data: &str) -> Cmd {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(data) else {
        return done();
    };
    match json.get("type").and_then(|t| t.as_str()) {
        Some("pagesUpdated") => on_pages_updated(model, &json),
        Some("confirm") => confirm(&json),
        _ => done(),
    }
}

/// An extension on the server asks the user. The answer goes back with the id of the question.
fn confirm(json: &serde_json::Value) -> Cmd {
    let field = |name: &str| {
        json.get(name)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let (request_id, message) = (field("requestId"), field("message"));
    run(move |api| async move {
        let ok = api.confirm(&message).await;
        // A late answer is refused by the server, and nothing is left to do about it
        let _ = api.answer_confirm(&request_id, ok).await;
    })
}

fn on_pages_updated(model: &mut Model, json: &serde_json::Value) -> Cmd {
    let page_id = json
        .get("pageId")
        .and_then(|p| p.as_str())
        .unwrap_or_default()
        .to_string();
    // A change that this client did not make is read again. An extension writes items after
    // a tag add, and the server writes the outline of an atlas page by itself, as when a page
    // of its directory is made. Both tell so with their own source
    let reload = match json.get("source").and_then(|s| s.as_str()) {
        Some("file" | "extension") => file_reload::reload_page(model, &page_id),
        Some("server") => atlas::reload_atlas(model, &page_id),
        _ => done(),
    };
    Command::all([
        reload,
        sidebar::refresh_recent_pages(),
        sidebar::load_index_pages(model),
        page::reload_backlinks(model),
    ])
}

fn schedule_reconnect(model: &mut Model) -> Cmd {
    if model.sse_reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
        return done();
    }
    let delay = (1000u32 * 2u32.pow(model.sse_reconnect_attempts)).min(30_000);
    model.sse_reconnect_attempts += 1;
    let timer_id = model.next_timer_id();
    run(move |api| async move {
        if api.sleep(timer_id, delay).await == TimerOutput::Fired {
            apply(&api, "sse reconnect", connect);
        }
    })
}
