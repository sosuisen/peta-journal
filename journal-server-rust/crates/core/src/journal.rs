//! The entrance of journal-core (ADR 029). REST, MCP and an app that embeds the core all
//! call an operation here, so the three give the same answer.
//!
//! An operation does all of the work of one request: it checks the values, calls the
//! services, and builds the answer. The answers are the JSON shapes of the REST API.
//!
//! The operations come here one area at a time. The search is the first. The other areas
//! are still in the handlers of the journal server.

mod search;

pub use search::SearchAnswer;

use std::sync::Arc;

use crate::application::combined_search::CombinedSearchService;
use crate::infrastructure::persistence::PropertyRepository;

pub struct Journal {
    combined_search: Arc<CombinedSearchService>,
    properties: PropertyRepository,
}

impl Journal {
    pub fn new(
        combined_search: Arc<CombinedSearchService>,
        properties: PropertyRepository,
    ) -> Journal {
        Journal {
            combined_search,
            properties,
        }
    }
}
