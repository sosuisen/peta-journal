//! The core for a shell that is not written in Rust (ADR 024). Events, the view model and the
//! effects of the screen cross as bincode. The timers and the clock stay on this side.
//!
//! This crate has no way to the journal and no entrance for another language. A crate above
//! it has both: it gives [`NativeShell`] a [`Host`] (ADR 036).

mod clock;
#[cfg(feature = "facet_typegen")]
pub mod codegen;
mod html;
mod shell;

pub use clock::{Clock, SystemClock};
pub use html::{copied_html, hashtag_url, wikilink_url};
pub use shell::{BorderError, Host, NativeShell, NothingThere, ShellEffect, ShellEffectFfi};

#[cfg(test)]
mod tests;
