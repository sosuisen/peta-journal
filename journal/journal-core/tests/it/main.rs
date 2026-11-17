//! The behaviour of the journal core, tested through its own entrance and parts rather than
//! through the server (ADR 058). One binary holds all the files, so a change links one test
//! program, not one per file.

#[path = "../common/mod.rs"]
mod common;

mod atlas_sync;
mod definitions_sync;
mod export_queue;
mod find_doc;
