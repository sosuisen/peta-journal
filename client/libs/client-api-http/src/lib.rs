//! Talks to the journal server for a shell that runs natively: the Emacs shell and the native
//! shells. The core knows the requests and the answers (`client_core::rest`); this crate sends
//! them.

mod api;
mod transport;

pub use api::{call, upload, Http};
pub use transport::{multipart_body, read_events, sse_data, UreqHttp};

#[cfg(test)]
mod tests;
