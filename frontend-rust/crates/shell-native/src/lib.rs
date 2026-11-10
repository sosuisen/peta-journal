//! The core for a shell that is not written in Rust (ADR 024). Events, the view model and the
//! effects of the screen cross as bincode. The server, the timers and the clock stay on this
//! side.

mod shell;

pub use shell::{BorderError, Host, NativeShell, ShellEffect, ShellEffectFfi};

#[cfg(test)]
mod tests;
