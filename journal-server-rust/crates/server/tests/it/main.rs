//! The integration tests in one test binary, apart from the MCP tests in `mcp`. Each file used
//! to be a binary of its own, and every change to the server linked each of them again. One
//! binary is linked once.

mod api;
mod atlas_sync;
#[path = "../common/mod.rs"]
mod common;
mod item_merge;
mod property_repository;
mod watch;
