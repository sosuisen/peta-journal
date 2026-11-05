//! The PetaJournal frontend core: the outline, its operations and the app logic, with no
//! dependency on the browser. Built on Crux: the shell sends [`Event`]s, the core answers
//! with [`Effect`]s and a [`ViewModel`].

pub mod app;
pub mod effects;
pub mod model;
pub mod ops;
pub mod outline;
pub mod rest;
pub mod screens;
pub mod testing;
pub mod util;
pub mod view;

#[cfg(test)]
pub mod harness;
#[cfg(test)]
mod tests;

pub use app::{Event, Model, PetaApp, Route};
pub use effects::Effect;
pub use view::ViewModel;

pub use model::*;
