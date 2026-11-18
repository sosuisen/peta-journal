//! PetaJournal journal core: the pages and items in a SQLite database, their markdown folder,
//! and the search. It knows no HTTP and runs no tasks of its own: the journal server and an
//! app that embeds it call it (ADR 029).

pub mod application;
pub mod assembly;
pub mod clock;
pub mod domain;
pub mod dto;
pub mod error;
pub mod events;
pub mod infrastructure;
pub mod journal;
