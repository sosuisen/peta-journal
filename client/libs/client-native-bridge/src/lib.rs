//! The core for a shell that is not written in Rust (ADR 024). Events, the view model and the
//! effects of the screen cross as bincode. The timers and the clock stay on this side.
//!
//! This crate has no way to the journal and no entrance for another language. A crate above
//! it has both: it gives [`NativeShell`] a [`Host`] (ADR 036).

mod clock;
#[cfg(test)]
mod examples;
#[cfg(feature = "facet_typegen")]
pub mod codegen;
mod html;
mod links;
mod shell;

pub use client_core::changes::{ListChange, OutlinePanel, Panel};
/// The changes of the screen of a native shell, whose body model is [`RichText`](client_core::RichText).
pub type Changes = client_core::changes::Changes<client_core::RichText>;
pub use clock::{Clock, SystemClock};
pub use html::copied_html;
pub use links::asset_address;
pub use shell::{BorderError, Host, NativeShell, NothingThere, ShellEffect, ShellEffectFfi};

/// The core under the border. A row gives a native shell the parts of its body to draw
/// (`RichText`, ADR 042).
pub type BorderApp = client_core::PetaApp<client_core::RichText>;

#[cfg(test)]
mod tests;
