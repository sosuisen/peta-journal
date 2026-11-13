//! The AST of each body that the outline shows (ADR 041). The core parses a body itself, with
//! the options that the journal parses it with, so that every shell draws the same typed AST.

use flatmarkdown::{parse_body_with, BodyOptions};
use flatmarkdown_ast::Node;
use std::collections::HashMap;
use std::sync::Mutex;

/// The ASTs of the bodies that the last view showed. The view is made after every event, and
/// most bodies are the same as the time before, so a body is parsed again only when it is new.
#[derive(Debug, Default)]
pub struct Asts {
    last: Mutex<HashMap<String, Node>>,
}

impl Asts {
    /// The AST of each of these bodies, in their order. The ASTs of bodies that are not among
    /// them are forgotten, so that what is kept stays as large as what is shown.
    pub fn of<'a>(&self, bodies: impl IntoIterator<Item = &'a str>) -> Vec<Node> {
        // A view that panicked while it held the lock left only whole entries behind
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut kept = HashMap::with_capacity(last.len());
        let asts = bodies
            .into_iter()
            .map(|body| {
                let ast = kept
                    .get(body)
                    .or_else(|| last.get(body))
                    .cloned()
                    .unwrap_or_else(|| parse(body));
                kept.insert(body.to_string(), ast.clone());
                ast
            })
            .collect();
        *last = kept;
        asts
    }
}

/// The AST of one body, as the journal makes it.
pub fn parse(body: &str) -> Node {
    parse_body_with(body, &BodyOptions::relative_page_links())
}
