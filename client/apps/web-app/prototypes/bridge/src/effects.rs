//! The effects that stay in wasm: the requests to the server, its event stream, and the
//! timers. They do what `outliner-web` does, without Leptos.

use crate::{ApiRequest, Inner};
use client_core::effects::{
    ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, SseEvent, SseOp, TimerOp, TimerOutput,
};
use client_core::rest::{rest_call, routes, ClientId, HttpReply, HttpRequest};
use crux_core::Request;
use gloo_net::http::Request as Http;
use gloo_timers::callback::Timeout;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

/// The local time of the browser.
pub(crate) fn local_now() -> LocalDateTimeDto {
    let date = js_sys::Date::new_0();
    LocalDateTimeDto {
        year: date.get_full_year() as i32,
        month: date.get_month() + 1,
        day: date.get_date(),
        hour: date.get_hours(),
        minute: date.get_minutes(),
    }
}

fn api_err(error: impl std::fmt::Display) -> ApiError {
    ApiError(error.to_string())
}

/// Asks the server, one request at a time and in the order the core made them
/// (`client_core::policy`).
pub(crate) fn call(inner: Rc<Inner>, request: Request<ApiRequest>) {
    if !inner.requests.borrow_mut().push(request) {
        return;
    }
    spawn_local(async move {
        let next = || inner.requests.borrow_mut().take_next();
        while let Some(mut request) = next() {
            let output = perform(&inner.client_id, request.operation.clone()).await;
            inner.resolve(&mut request, output);
        }
    });
}

async fn perform(client_id: &ClientId, op: ApiOp) -> ApiResult<ApiOutput> {
    let rest = rest_call(op).ok_or_else(|| ApiError("not a REST call".into()))?;
    let reply = send(client_id, &rest.request).await?;
    rest.decode(&reply)
}

async fn send(client_id: &ClientId, request: &HttpRequest) -> ApiResult<HttpReply> {
    let builder = match request.method {
        "POST" => Http::post(&request.path),
        "PUT" => Http::put(&request.path),
        "DELETE" => Http::delete(&request.path),
        _ => Http::get(&request.path),
    }
    .header(ClientId::HEADER, client_id.as_str());
    let sent = match &request.body {
        Some(body) => builder
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .map_err(api_err)?,
        None => builder.build().map_err(api_err)?,
    };
    let response = sent.send().await.map_err(api_err)?;
    Ok(HttpReply {
        status: response.status(),
        body: response.text().await.map_err(api_err)?,
    })
}

/// A timer that runs, with the request that waits for it.
pub(crate) struct Running {
    request: Request<TimerOp>,
    timeout: Option<Timeout>,
}

pub(crate) fn time(inner: Rc<Inner>, request: Request<TimerOp>) {
    match request.operation {
        TimerOp::Start { id, ms } => {
            let fired = inner.clone();
            let timeout = Timeout::new(ms, move || {
                let running = fired.timers.borrow_mut().end(id);
                if let Some(mut running) = running {
                    running.timeout.take().map(Timeout::forget);
                    fired.resolve(&mut running.request, TimerOutput::Fired);
                }
            });
            let running = Running {
                request,
                timeout: Some(timeout),
            };
            inner.timers.borrow_mut().start(id, running);
        }
        TimerOp::Cancel { id } => {
            let running = inner.timers.borrow_mut().end(id);
            if let Some(mut running) = running {
                if let Some(timeout) = running.timeout.take() {
                    timeout.cancel();
                }
                inner.resolve(&mut running.request, TimerOutput::Cancelled);
            }
        }
    }
}

/// The event stream of the server, with the callbacks that keep it open.
pub(crate) struct Connection {
    source: web_sys::EventSource,
    _on_open: Closure<dyn FnMut(web_sys::Event)>,
    _on_message: Closure<dyn FnMut(web_sys::MessageEvent)>,
    _on_error: Closure<dyn FnMut(web_sys::Event)>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.source.close();
    }
}

pub(crate) fn listen(inner: Rc<Inner>, request: Request<SseOp>) {
    // One connection at a time: a new request replaces the old stream
    inner.sse.borrow_mut().take();
    let Ok(source) = web_sys::EventSource::new(routes::EVENTS.pattern) else {
        let mut request = request;
        inner.resolve(&mut request, SseEvent::Error);
        return;
    };
    let request = Rc::new(RefCell::new(request));
    let on_open = {
        let (inner, request) = (inner.clone(), request.clone());
        Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            inner.resolve(&mut request.borrow_mut(), SseEvent::Open);
        })
    };
    let on_message = {
        let (inner, request) = (inner.clone(), request.clone());
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            if let Some(data) = event.data().as_string() {
                inner.resolve(&mut request.borrow_mut(), SseEvent::Message(data));
            }
        })
    };
    let on_error = {
        let (inner, request) = (inner.clone(), request.clone());
        Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            // Closed first, so that the core opens a new stream when it connects again
            inner.sse.borrow_mut().take();
            inner.resolve(&mut request.borrow_mut(), SseEvent::Error);
        })
    };
    source.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    source.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    *inner.sse.borrow_mut() = Some(Connection {
        source,
        _on_open: on_open,
        _on_message: on_message,
        _on_error: on_error,
    });
}
