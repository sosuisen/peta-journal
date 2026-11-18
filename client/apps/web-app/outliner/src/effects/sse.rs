//! The server's event stream (`/api/events`), delivered to the core as a stream request.

use crate::core::CoreHandle;
use client_core::effects::{SseEvent, SseOp};
use client_core::rest::routes;
use crux_core::Request;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub struct Connection {
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

pub fn connect(handle: CoreHandle, request: Request<SseOp>) {
    // One connection at a time; a new request replaces the old stream
    handle.inner().sse.borrow_mut().take();
    let Ok(source) = web_sys::EventSource::new(routes::EVENTS.pattern) else {
        let mut request = request;
        handle.resolve(&mut request, SseEvent::Error);
        return;
    };
    let request = std::rc::Rc::new(std::cell::RefCell::new(request));

    let on_open = {
        let (handle, request) = (handle, request.clone());
        Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            handle.resolve(&mut request.borrow_mut(), SseEvent::Open);
        })
    };
    let on_message = {
        let (handle, request) = (handle, request.clone());
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |ev: web_sys::MessageEvent| {
            if let Some(data) = ev.data().as_string() {
                handle.resolve(&mut request.borrow_mut(), SseEvent::Message(data));
            }
        })
    };
    let on_error = {
        let (handle, request) = (handle, request.clone());
        Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            // Close first, so the core's reconnect opens a fresh stream
            handle.inner().sse.borrow_mut().take();
            handle.resolve(&mut request.borrow_mut(), SseEvent::Error);
        })
    };
    source.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    source.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    *handle.inner().sse.borrow_mut() = Some(Connection {
        source,
        _on_open: on_open,
        _on_message: on_message,
        _on_error: on_error,
    });
}
