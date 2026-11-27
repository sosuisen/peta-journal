//! The table of the REST routes and the MCP tools (ADR 060). REST and MCP offer the same
//! operations, so every route has its tool here, or is marked as one of the exceptions of the
//! ADR. A route or a tool that is added without a line here fails a test.

use crate::connect;
use journal_api_types::routes::{self, Route};
use std::collections::BTreeSet;

/// What MCP has for a REST route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mcp {
    /// The tool of the same operation.
    Tool(&'static str),
    /// Nothing: the route is only for a screen or a browser. ADR 060 lists these exceptions.
    OnlyForScreens,
}

use Mcp::{OnlyForScreens, Tool};

const TABLE: &[(Route, Mcp)] = &[
    (routes::HEALTH, OnlyForScreens),
    (routes::EVENTS, OnlyForScreens),
    // pages
    (routes::GET_OR_CREATE_DOC, Tool("get_or_create_doc")),
    (routes::FIND_PAGE, OnlyForScreens),
    (routes::RECENT_PAGES, Tool("get_recent_pages")),
    (routes::GET_PAGE, Tool("get_page_by_id")),
    (routes::UPDATE_PAGE, Tool("update_page")),
    (routes::DELETE_PAGE, Tool("delete_page")),
    (routes::BACKLINKS, Tool("get_backlinks")),
    (routes::DEMOTE_PAGE, Tool("demote_page")),
    (routes::CHILD_PAGES, Tool("get_child_pages")),
    (routes::OPEN_EXPORT_FOLDER, OnlyForScreens),
    (routes::UPSERT_PAGE_PROPERTY, Tool("upsert_page_property")),
    (routes::DELETE_PAGE_PROPERTY, Tool("delete_page_property")),
    // items
    (routes::CREATE_ITEM, Tool("create_item")),
    (routes::ITEMS_BY_TYPES, Tool("get_items_by_type")),
    (routes::MOVE_ITEMS, Tool("move_items")),
    (routes::MERGE_ITEMS, Tool("merge_items")),
    (routes::DELETE_ITEMS, Tool("delete_items")),
    (routes::APPLY_BATCH, Tool("apply_batch")),
    (routes::TRANSCLUDED_ITEMS, Tool("get_transcluded_items")),
    (routes::SHIFT_ITEM_LEVELS, Tool("shift_item_levels")),
    (routes::EXPORT_TREE_MARKDOWN, Tool("export_tree_markdown")),
    (routes::EXPORT_PAGE_MARKDOWN, Tool("export_page_markdown")),
    (
        routes::EXPORT_SUBTREE_MARKDOWN,
        Tool("export_subtree_markdown"),
    ),
    (routes::IMPORT_TREE_MARKDOWN, Tool("import_tree_markdown")),
    (routes::PARSE_FLAT_MARKDOWN, Tool("parse_flat_markdown")),
    (routes::GET_ITEM, Tool("get_item")),
    (routes::UPDATE_ITEM, Tool("update_item")),
    (routes::DELETE_ITEM, Tool("delete_item")),
    (routes::SET_TRANSCLUSION, Tool("set_transclusion")),
    (routes::REMOVE_TRANSCLUSION, Tool("remove_transclusion")),
    (routes::PASTE_TEXT, Tool("paste_text")),
    (routes::SPLIT_ITEM, Tool("split_item")),
    (routes::STASH_ITEM, Tool("stash_item")),
    (routes::CYCLE_TASK, Tool("cycle_task")),
    (routes::UPSERT_ITEM_PROPERTY, Tool("upsert_property")),
    (routes::DELETE_ITEM_PROPERTY, Tool("delete_property")),
    // journals
    (
        routes::JOURNAL_BEFORE,
        Tool("get_most_recent_journal_before"),
    ),
    (
        routes::WEEKLY_JOURNAL_BEFORE,
        Tool("get_most_recent_weekly_journal_before"),
    ),
    (routes::GET_OR_CREATE_JOURNAL, Tool("get_or_create_journal")),
    (routes::MOVE_ITEMS_TO_JOURNAL, Tool("move_items_to_journal")),
    // search
    (routes::SEARCH_PAGE_NAMES, Tool("search_page_names")),
    (routes::SEARCH_COMBINED, Tool("search_combined")),
    (routes::SEARCH_BY_TAG, Tool("search_by_tag")),
    // config
    (routes::GET_CONFIG, Tool("get_config")),
    (routes::UPDATE_CONFIG, Tool("update_config")),
    (
        routes::PROPERTY_DEFINITIONS,
        Tool("get_property_definitions"),
    ),
    (
        routes::ADD_PROPERTY_DEFINITION,
        Tool("add_property_definition"),
    ),
    (
        routes::DELETE_PROPERTY_DEFINITION,
        Tool("delete_property_definition"),
    ),
    (routes::BROWSE_FOLDER, OnlyForScreens),
    (routes::ANSWER_CONFIRM, OnlyForScreens),
    // the markdown folder
    (routes::SYNC_STATUS, Tool("get_sync_status")),
    (routes::SCAN_FOLDER, Tool("scan_folder")),
    (routes::PENDING_DELETES, OnlyForScreens),
    (routes::SUBMIT_PENDING_DELETES, OnlyForScreens),
    // assets
    (routes::UPLOAD_ASSET, Tool("upload_asset")),
];

/// The routes the table of the API names, read from its source: `(method, pattern)` of every
/// `pub const NAME: Route = Route::method("pattern");`.
fn routes_of_the_api() -> BTreeSet<(String, String)> {
    let source = include_str!("../../../journal-api-types/src/routes.rs");
    source
        .split("pub const ")
        .skip(1)
        .filter_map(|constant| {
            let (_, made) = constant.split_once("Route::")?;
            let (method, rest) = made.split_once('(')?;
            let pattern = rest.split('"').nth(1)?;
            Some((method.to_uppercase(), pattern.to_string()))
        })
        .collect()
}

// APIのどのルートも、対応表に1行を持つ。ルートを足して表に書かないと、ここで落ちる。
#[test]
fn every_route_of_the_api_has_a_line_in_the_table() {
    let in_table: BTreeSet<(String, String)> = TABLE
        .iter()
        .map(|(route, _)| (route.method.as_str().to_string(), route.pattern.to_string()))
        .collect();

    assert_eq!(in_table, routes_of_the_api());
    assert_eq!(in_table.len(), TABLE.len(), "a route has two lines");
}

// MCPのツールは、対応表のツールとちょうど同じである。どのツールも、1つのルートの操作である。
#[tokio::test]
async fn the_tools_are_those_of_the_table_one_for_each_route() {
    let mcp = connect().await;

    let tools = mcp.list_all_tools().await.expect("tools");
    let listed: BTreeSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    let in_table: Vec<&str> = TABLE
        .iter()
        .filter_map(|(_, mcp)| match mcp {
            Tool(name) => Some(*name),
            OnlyForScreens => None,
        })
        .collect();

    assert_eq!(listed, in_table.iter().copied().collect::<BTreeSet<_>>());
    assert_eq!(listed.len(), in_table.len(), "two routes name one tool");
}
