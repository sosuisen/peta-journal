//! The places of the REST API of the journal server. The server makes its routes from these
//! and the frontend core makes its requests from the same ones. So the two cannot write a
//! path or a method differently.
//!
//! A pattern names a changing part of a path as `{name}`, the way axum writes it.

/// The HTTP method of a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

impl Method {
    /// The name HTTP writes the method with.
    pub const fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
        }
    }
}

/// One place of the API: a method and the pattern of a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub method: Method,
    pub pattern: &'static str,
}

impl Route {
    const fn get(pattern: &'static str) -> Route {
        Route {
            method: Method::Get,
            pattern,
        }
    }

    const fn post(pattern: &'static str) -> Route {
        Route {
            method: Method::Post,
            pattern,
        }
    }

    const fn put(pattern: &'static str) -> Route {
        Route {
            method: Method::Put,
            pattern,
        }
    }

    const fn delete(pattern: &'static str) -> Route {
        Route {
            method: Method::Delete,
            pattern,
        }
    }

    /// The path of a request: the pattern with each `{name}` replaced by the next of `args`.
    ///
    /// The caller encodes each value for a path first: this crate has no dependency for it.
    ///
    /// # Panics
    ///
    /// When the number of `args` is not the number of `{name}` parts of the pattern.
    pub fn path(&self, args: &[&str]) -> String {
        let mut args = args.iter();
        let mut path = String::new();
        let mut rest = self.pattern;
        while let Some((before, after)) = rest.split_once('{') {
            let (_name, after) = after.split_once('}').expect("a pattern closes each `{`");
            path.push_str(before);
            path.push_str(args.next().expect("a value for each part of the pattern"));
            rest = after;
        }
        assert!(
            args.next().is_none(),
            "more values than parts in {}",
            self.pattern
        );
        path.push_str(rest);
        path
    }
}

// ---- the server itself ----

pub const HEALTH: Route = Route::get("/api/health");
pub const EVENTS: Route = Route::get("/api/events");

// ---- pages ----

pub const GET_OR_CREATE_DOC: Route = Route::post("/api/pages/doc");
pub const FIND_PAGE: Route = Route::get("/api/pages/find");
pub const RECENT_PAGES: Route = Route::get("/api/pages/recent");
pub const GET_PAGE: Route = Route::get("/api/pages/{id}");
pub const UPDATE_PAGE: Route = Route::put("/api/pages/{id}");
pub const DELETE_PAGE: Route = Route::delete("/api/pages/{id}");
pub const BACKLINKS: Route = Route::get("/api/pages/{id}/backlinks");
pub const DEMOTE_PAGE: Route = Route::post("/api/pages/{id}/demote");
pub const CHILD_PAGES: Route = Route::get("/api/pages/{id}/children");
pub const OPEN_EXPORT_FOLDER: Route = Route::post("/api/pages/{id}/open-folder");
pub const UPSERT_PAGE_PROPERTY: Route = Route::put("/api/pages/{id}/properties/{key}");
pub const DELETE_PAGE_PROPERTY: Route = Route::delete("/api/pages/{id}/properties/{key}");

// ---- items ----

pub const CREATE_ITEM: Route = Route::post("/api/items");
pub const ITEMS_BY_TYPES: Route = Route::get("/api/items");
pub const MOVE_ITEMS: Route = Route::post("/api/items/move");
pub const MERGE_ITEMS: Route = Route::post("/api/items/merge");
pub const DELETE_ITEMS: Route = Route::post("/api/items/delete");
pub const APPLY_BATCH: Route = Route::post("/api/items/batch");
pub const TRANSCLUDED_ITEMS: Route = Route::post("/api/items/transcluded");
pub const SHIFT_ITEM_LEVELS: Route = Route::post("/api/items/shift-level");
pub const EXPORT_TREE_MARKDOWN: Route = Route::post("/api/items/export");
pub const EXPORT_PAGE_MARKDOWN: Route = Route::post("/api/items/export-page");
pub const EXPORT_SUBTREE_MARKDOWN: Route = Route::post("/api/items/export-subtree");
pub const IMPORT_TREE_MARKDOWN: Route = Route::post("/api/items/import");
pub const PARSE_FLAT_MARKDOWN: Route = Route::post("/api/items/parse");
pub const GET_ITEM: Route = Route::get("/api/items/{id}");
pub const UPDATE_ITEM: Route = Route::put("/api/items/{id}");
pub const DELETE_ITEM: Route = Route::delete("/api/items/{id}");
pub const SET_TRANSCLUSION: Route = Route::post("/api/items/{id}/transclude");
pub const REMOVE_TRANSCLUSION: Route = Route::delete("/api/items/{id}/transclude");
pub const PASTE_TEXT: Route = Route::post("/api/items/{id}/paste");
pub const SPLIT_ITEM: Route = Route::post("/api/items/{id}/split");
pub const STASH_ITEM: Route = Route::post("/api/items/{id}/stash");
pub const CYCLE_TASK: Route = Route::post("/api/items/{id}/cycle-task");
pub const UPSERT_ITEM_PROPERTY: Route = Route::put("/api/items/{id}/properties/{key}");
pub const DELETE_ITEM_PROPERTY: Route = Route::delete("/api/items/{id}/properties/{key}");

// ---- journals ----

pub const JOURNAL_BEFORE: Route = Route::get("/api/journals/before");
pub const WEEKLY_JOURNAL_BEFORE: Route = Route::get("/api/journals/weekly/before");
pub const GET_OR_CREATE_JOURNAL: Route = Route::post("/api/journals/{name}");
pub const MOVE_ITEMS_TO_JOURNAL: Route = Route::post("/api/journals/{name}/move");

// ---- search ----

pub const SEARCH_PAGE_NAMES: Route = Route::get("/api/search/pages");
pub const SEARCH_COMBINED: Route = Route::get("/api/search/combined");
pub const SEARCH_BY_TAG: Route = Route::get("/api/search/tags");

// ---- config ----

pub const GET_CONFIG: Route = Route::get("/api/config");
pub const UPDATE_CONFIG: Route = Route::put("/api/config");
pub const PROPERTY_DEFINITIONS: Route = Route::get("/api/config/property-definitions");
pub const ADD_PROPERTY_DEFINITION: Route = Route::post("/api/config/property-definitions");
pub const DELETE_PROPERTY_DEFINITION: Route =
    Route::delete("/api/config/property-definitions/{key}");
pub const BROWSE_FOLDER: Route = Route::post("/api/config/browse-folder");

// ---- questions to the user ----

pub const ANSWER_CONFIRM: Route = Route::post("/api/confirm/{request_id}");

// ---- the markdown folder ----

pub const SYNC_STATUS: Route = Route::get("/api/sync/status");
pub const SCAN_FOLDER: Route = Route::post("/api/sync/scan");
pub const PENDING_DELETES: Route = Route::get("/api/sync/pending-deletes");
pub const SUBMIT_PENDING_DELETES: Route = Route::post("/api/sync/pending-deletes");

// ---- assets ----

pub const UPLOAD_ASSET: Route = Route::post("/api/assets/upload");

#[cfg(test)]
mod tests {
    use super::*;

    // 経路のパスは、パターンの変わる部分を、渡した値で順に置き換えたものである。
    #[test]
    fn a_path_is_the_pattern_with_its_parts_replaced_in_order() {
        assert_eq!(
            UPSERT_ITEM_PROPERTY.path(&["a", "due"]),
            "/api/items/a/properties/due"
        );
        assert_eq!(
            GET_OR_CREATE_JOURNAL.path(&["2026-11-06"]),
            "/api/journals/2026-11-06"
        );
    }

    // 変わる部分のない経路のパスは、パターンそのままである。
    #[test]
    fn a_path_without_parts_is_the_pattern() {
        assert_eq!(CREATE_ITEM.path(&[]), "/api/items");
    }

    // 値が足りなければ、パスを作らずに止まる。
    #[test]
    #[should_panic]
    fn a_path_with_too_few_values_is_refused() {
        SPLIT_ITEM.path(&[]);
    }

    // 値が多すぎても、パスを作らずに止まる。
    #[test]
    #[should_panic]
    fn a_path_with_too_many_values_is_refused() {
        CREATE_ITEM.path(&["a"]);
    }

    // メソッドは、HTTPが書く名前になる。
    #[test]
    fn a_method_has_the_name_http_writes() {
        let names = [Method::Get, Method::Post, Method::Put, Method::Delete].map(Method::as_str);
        assert_eq!(names, ["GET", "POST", "PUT", "DELETE"]);
    }
}
