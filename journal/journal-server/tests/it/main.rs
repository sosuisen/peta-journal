//! The integration tests in one test binary, apart from the MCP tests in `mcp`. Each file used
//! to be a binary of its own, and every change to the server linked each of them again. One
//! binary is linked once.

mod api;
mod asset_names;
mod atlas_sync;
mod client_id;
#[path = "../common/mod.rs"]
mod common;
mod find_doc;
mod frontend;
mod item_merge;
mod page_files;
mod page_properties;
mod property_repository;
mod watch;
