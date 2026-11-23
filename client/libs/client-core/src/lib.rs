//! The PetaJournal frontend core: the outline, its operations and the app logic, with no
//! dependency on the browser. Built on Crux: the shell sends [`Event`]s, the core answers
//! with [`Effect`]s and a [`ViewModel`].

pub mod app;
pub mod asts;
pub mod body_model;
pub mod changes;
pub mod drag;
pub mod effects;
mod file_body;
pub mod keys;
pub mod model;
pub mod ops;
pub mod outline;
pub mod policy;
pub mod rest;
pub mod rich_text;
pub mod screens;
pub mod testing;
pub mod typing;
pub mod util;
pub mod view;

#[cfg(test)]
pub mod harness;
#[cfg(test)]
mod tests;

pub use app::{EditPlace, Event, Model, PetaApp, Route};
pub use body_model::BodyModel;
pub use effects::Effect;
pub use rich_text::RichText;
pub use view::ViewModel;

pub use model::*;
