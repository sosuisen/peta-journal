//! The server's event stream, ported from `sse.service.ts`: connection state, reconnection
//! with exponential backoff, and what a `pagesUpdated` or a `propertyDefinitionsUpdated`
//! event triggers.

use crate::app::{done, run, Cmd, ConnectionState, Event, Model};
use crate::effects::{SseEvent, SseOp, TimerOutput};
use crate::ops::file_reload;
use crate::screens::{atlas, page, settings, sidebar};
use crux_core::Command;
use journal_api_types::{ChangeSource, ClientMessage};

const MAX_DELAY_MS: u32 = 30_000;

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
            // What changed while the stream was down was told to no one
            Command::all([
                sidebar::refresh_recent_pages(),
                settings::load_property_definitions(),
            ])
        }
        SseEvent::Message(data) => on_message(model, &data),
        SseEvent::Error => {
            model.connection = ConnectionState::Error;
            schedule_reconnect(model)
        }
    }
}

fn on_message(model: &mut Model, data: &str) -> Cmd {
    // The journal writes the message with the same type (`journal-api-types`). A message of
    // a kind that a newer journal sends and this client does not know is passed over
    match serde_json::from_str::<ClientMessage>(data) {
        Ok(ClientMessage::PagesUpdated {
            page_id,
            source,
            client_id,
        }) => on_pages_updated(model, page_id, source, client_id),
        Ok(ClientMessage::Confirm {
            request_id,
            message,
        }) => confirm(request_id, message),
        Ok(ClientMessage::PropertyDefinitionsUpdated) => settings::load_property_definitions(),
        Err(_) => done(),
    }
}

/// An extension on the server asks the user. The answer goes back with the id of the question.
fn confirm(request_id: String, message: String) -> Cmd {
    run(move |api| async move {
        let ok = api.confirm(&message).await;
        // A late answer is refused by the server, and nothing is left to do about it
        let _ = api.answer_confirm(&request_id, ok).await;
    })
}

fn on_pages_updated(
    model: &mut Model,
    page_id: String,
    source: ChangeSource,
    client_id: Option<String>,
) -> Cmd {
    let by_this_client = client_id.as_deref() == Some(model.client_id.as_str());
    // A change that this client did not make is read again. Another client names itself with
    // a change through the API (ADR-015). An MCP tool is never this client. An extension writes
    // items after a tag add, and the server writes the outline of an atlas page by itself, as
    // when a page of its directory is made. Each tells so with its own source
    let reload = match source {
        ChangeSource::App if by_this_client => done(),
        ChangeSource::App | ChangeSource::Mcp => reload_for_another_client(model, &page_id),
        ChangeSource::File | ChangeSource::Extension => file_reload::reload_page(model, &page_id),
        ChangeSource::Server => atlas::reload_atlas(model, &page_id),
        ChangeSource::Other => done(),
    };
    Command::all([
        reload,
        sidebar::refresh_recent_pages(),
        sidebar::load_index_pages(model),
        page::reload_backlinks(model),
    ])
}

/// Reads again the page that another client changed. An atlas that shows no item yet has
/// nothing for `reload_page` to replace, so it is read as the server's own changes to it are.
fn reload_for_another_client(model: &mut Model, page_id: &str) -> Cmd {
    if model.state.items().iter().any(|i| i.page_id == page_id) {
        file_reload::reload_page(model, page_id)
    } else {
        atlas::reload_atlas(model, page_id)
    }
}

/// Tries again until the stream opens: the server may start long after the shell (#756).
fn schedule_reconnect(model: &mut Model) -> Cmd {
    let delay = 2u32
        .saturating_pow(model.sse_reconnect_attempts)
        .saturating_mul(1000)
        .min(MAX_DELAY_MS);
    model.sse_reconnect_attempts += 1;
    let timer_id = model.next_timer_id();
    run(move |api| async move {
        if api.sleep(timer_id, delay).await == TimerOutput::Fired {
            api.send_event(Event::SseReconnect);
        }
    })
}
