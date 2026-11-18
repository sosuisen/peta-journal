//! Timers for the core: a request that resolves when the delay is over, or as cancelled.

use crate::core::CoreHandle;
use client_core::effects::{TimerOp, TimerOutput};
use crux_core::Request;
use gloo_timers::callback::Timeout;

pub struct Running {
    request: Request<TimerOp>,
    timeout: Option<Timeout>,
}

pub fn start(handle: CoreHandle, id: u64, ms: u32, request: Request<TimerOp>) {
    let fire = move || {
        let running = handle.inner().timers.borrow_mut().end(id);
        if let Some(mut running) = running {
            running.timeout.take().map(Timeout::forget);
            handle.resolve(&mut running.request, TimerOutput::Fired);
        }
    };
    let timeout = Timeout::new(ms, fire);
    handle.inner().timers.borrow_mut().start(
        id,
        Running {
            request,
            timeout: Some(timeout),
        },
    );
}

pub fn cancel(handle: &CoreHandle, id: u64) {
    let running = handle.inner().timers.borrow_mut().end(id);
    if let Some(mut running) = running {
        if let Some(timeout) = running.timeout.take() {
            timeout.cancel();
        }
        handle.resolve(&mut running.request, TimerOutput::Cancelled);
    }
}
