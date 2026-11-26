//! The core for the web shell in TypeScript (ADR 079). The shell sends its events in and takes
//! the changes of the screen out, as the native shells do with `client-native-bridge`. Both
//! cross as the JSON of facet (ADR 080). The requests to the server, the event stream, the
//! timers and the APIs of `window` stay here, in wasm.

pub mod atlas;
pub mod body;
pub mod border;
pub mod bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod codegen;
pub mod examples;
pub mod lanes;
pub mod replay;
pub mod settings;
pub mod tags;
#[cfg(target_arch = "wasm32")]
pub mod web;
pub mod wiki;
