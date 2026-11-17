//! The behaviour of the journal core, tested through its own entrance and parts rather than
//! through the server (ADR 058). One binary holds all the files, so a change links one test
//! program, not one per file.

#[path = "../common/mod.rs"]
mod common;

mod atlas_sync;
#[cfg(feature = "extensions")]
mod auto_template;
mod changes_told;
mod definitions_sync;
mod export_queue;
#[cfg(feature = "extensions")]
mod extensions;
mod find_doc;
mod item_merge;
mod page_properties;
