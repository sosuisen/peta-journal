//! The core for a shell that asks a journal server over HTTP: the SwiftUI and the WinUI shell
//! (ADR 024, ADR 028). It gives the core of `client-native-bridge` its way to the journal, and
//! it has the entrance for another language (ADR 036).

#[cfg(feature = "ffi")]
boltffi::scaffolding!();

#[cfg(feature = "ffi")]
pub mod ffi;
mod server;

pub use server::connect;
