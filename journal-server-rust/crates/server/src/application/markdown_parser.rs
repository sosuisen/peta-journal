//! Converts Flat Markdown text to AST JSON with the flatmarkdown crate.

use flatmarkdown::{BodyOptions, LinkResolution};

#[derive(Debug, Clone, Default)]
pub struct MarkdownParser;

fn options() -> BodyOptions {
    // Link resolution turns `#tag` text into hashtag nodes; bodies in the app hold
    // wikilinks and hashtags, never relative Markdown links, so no prefixes are needed.
    BodyOptions {
        link_resolution: LinkResolution::On { prefixes: Vec::new() },
    }
}

impl MarkdownParser {
    /// Parses the body of one item and returns the AST JSON. An empty body gives an empty string.
    pub fn parse_to_ast(&self, markdown: &str) -> String {
        if markdown.is_empty() {
            return String::new();
        }
        flatmarkdown::body_to_ast_with(markdown, &options())
    }

    /// Splits a whole Flat Markdown page into items; returns the flatmarkdown page JSON.
    pub fn parse_page(&self, markdown: &str) -> String {
        flatmarkdown::page_to_ast_with(markdown, &options())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashtags_become_nodes() {
        let ast = MarkdownParser.parse_to_ast("remember #urgent");
        assert!(ast.contains("\"hashtag\""), "{ast}");
        assert_eq!(MarkdownParser.parse_to_ast(""), "");
    }

    #[test]
    fn page_json_has_items() {
        let json = MarkdownParser.parse_page("---\nid: x\n---\n\n<!-- @ 2026-09-29-00-00-00-AAAAAAA idx=\"z0\" -->\nhello\n");
        assert!(json.contains("\"items\""));
        assert!(json.contains("front_matter"));
    }
}
