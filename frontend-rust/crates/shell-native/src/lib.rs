//! The core for a shell that is not written in Rust (ADR 024). Events, the view model and the
//! effects of the screen cross as bincode. The server, the timers and the clock stay on this
//! side.

#[cfg(feature = "facet_typegen")]
pub mod codegen;
#[cfg(feature = "ffi")]
boltffi::scaffolding!();

#[cfg(feature = "ffi")]
pub mod ffi;
mod html;
mod server;
mod shell;

pub use html::item_html;
pub use server::connect;
pub use shell::{BorderError, Host, NativeShell, NothingThere, ShellEffect, ShellEffectFfi};

#[cfg(test)]
mod tests;
