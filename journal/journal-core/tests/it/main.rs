//! The behaviour of the journal core, tested through its own entrance and parts rather than
//! through the server (ADR 058). One binary holds all the files, so a change links one test
//! program, not one per file.

#[path = "../common/mod.rs"]
mod common;

mod atlas;
mod atlas_migration;
mod atlas_sync;
#[cfg(feature = "extensions")]
mod auto_template;
mod batch;
mod changes_told;
mod config;
mod creating_items;
mod definitions_sync;
mod export_queue;
#[cfg(feature = "extensions")]
mod extensions;
mod find_doc;
mod folder_sync;
mod group_delete;
mod item_merge;
mod journals;
mod markdown_conversions;
mod merging_items;
mod moving_items;
mod page_properties;
mod pages;
mod paste;
mod plain_atlas;
mod properties;
mod search;
mod split;
mod stash;
mod tasks;
mod transclusion;
mod unclosed_blocks;
mod updating_items;
