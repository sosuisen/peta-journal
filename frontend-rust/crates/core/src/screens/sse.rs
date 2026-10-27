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
    if json.get("type").and_then(|t| t.as_str()) != Some("pagesUpdated") {
        return done();
    }
    let page_id = json
        .get("pageId")
        .and_then(|p| p.as_str())
        .unwrap_or_default()
        .to_string();
    let from_file = json.get("source").and_then(|s| s.as_str()) == Some("file");
    let (reload, groups) = if from_file {
        (
            file_reload::reload_page(model, &page_id),
            atlas::reload_groups(model, &page_id),
        )
    } else {
        (done(), done())
    };
    Command::all([
        reload,
        groups,
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
