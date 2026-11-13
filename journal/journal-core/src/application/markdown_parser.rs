//! Parses Flat Markdown with the flatmarkdown crate.

use flatmarkdown::BodyOptions;
use flatmarkdown_ast::Node;

#[derive(Debug, Clone, Default)]
pub struct MarkdownParser;

/// Pages are `.md` files linked by relative paths: `#tag` text becomes a hashtag node, and a
/// link to `./x.md` or `../x.md` a wikilink node (`[#x](../x.md)` becomes a hashtag). The
/// clients parse bodies with the same options (ADR 041).
fn options() -> BodyOptions {
    BodyOptions::relative_page_links()
}

impl MarkdownParser {
    /// The AST of the body of one item. The journal keeps no AST: it parses a body when it
    /// needs what is in it (ADR 041).
    pub fn parse(&self, markdown: &str) -> Node {
        flatmarkdown::parse_body_with(markdown, &options())
    }

    /// The AST of the body of one item as JSON, for the apps that read it from the REST API.
    /// An empty body gives an empty string.
    pub fn parse_to_ast(&self, markdown: &str) -> String {
        if markdown.is_empty() {
            return String::new();
        }
        self.parse(markdown).to_json()
    }

    /// Splits a whole Flat Markdown page into items; returns the flatmarkdown page JSON.
    pub fn parse_page(&self, markdown: &str) -> String {
        flatmarkdown::parse_page_with(markdown, &options()).to_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ハッシュタグはASTのノードになる。
    #[test]
    fn hashtags_become_nodes() {
        let ast = MarkdownParser.parse_to_ast("remember #urgent");
        assert!(ast.contains("\"hashtag\""), "{ast}");
        assert_eq!(MarkdownParser.parse_to_ast(""), "");
    }

    // 相対リンクは、Javaのラッパーと同じようにウィキリンクになる。
    #[test]
    fn relative_links_become_wikilinks_like_the_java_wrapper() {
        let ast =
            MarkdownParser.parse_to_ast("[Page](../docs/Page.md) [#f](../f.md) [x](docs/x.md)");
        assert!(
            ast.contains(r#""type":"wikilink","url":"docs/Page""#),
            "{ast}"
        );
        assert!(ast.contains(r#"{"type":"hashtag","value":"f"}"#), "{ast}");
        assert!(ast.contains(r#""type":"link","url":"docs/x.md""#), "{ast}");
    }

    // ページのJSONには、項目とフロントマターが入る。
    #[test]
    fn page_json_has_items() {
        let json = MarkdownParser.parse_page(
            "---\nid: x\n---\n\n<!-- @ 2026-09-29-00-00-00-AAAAAAA idx=\"z0\" -->\nhello\n",
        );
        assert!(json.contains("\"items\""));
        assert!(json.contains("front_matter"));
    }
}
