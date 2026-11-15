//! The core for a shell that is not written in Rust (ADR 024). Events, the view model and the
//! effects of the screen cross as bincode. The timers and the clock stay on this side.
//!
//! This crate has no way to the journal and no entrance for another language. A crate above
//! it has both: it gives [`NativeShell`] a [`Host`] (ADR 036).

mod clock;
#[cfg(feature = "facet_typegen")]
pub mod codegen;
mod html;
mod links;
mod shell;
mod typed;

pub use clock::{Clock, SystemClock};
pub use html::copied_html;
pub use links::{asset_address, page_address};
pub use shell::{BorderError, Host, NativeShell, NothingThere, ShellEffect, ShellEffectFfi};
pub use typed::closed_as_typed;

/// The core under the border. A row gives a native shell the parts of its body to draw
/// (`RichText`, ADR 042).
pub type BorderApp = client_core::PetaApp<client_core::RichText>;

#[cfg(test)]
mod tests;
