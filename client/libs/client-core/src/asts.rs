//! The AST of a body (ADR 041). The core parses a body itself, with the options that the
//! journal parses it with, so that every shell draws the same body.

use flatmarkdown::{parse_body_with, BodyOptions};
use flatmarkdown_ast::Node;

/// The AST of one body, as the journal makes it.
pub fn parse(body: &str) -> Node {
    parse_body_with(body, &BodyOptions::relative_page_links())
}
