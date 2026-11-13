//! Converts Flat Markdown text to AST JSON with the flatmarkdown crate.

use flatmarkdown::{BodyOptions, LinkResolution};

#[derive(Debug, Clone, Default)]
pub struct MarkdownParser;

fn options() -> BodyOptions {
    // The same options as the Java wrapper (com.sosuisha:flatmarkdown): link resolution turns
    // `#tag` text into hashtag nodes, and relative Markdown links to `./x.md` or `../x.md`
    // into wikilink nodes (`[#x](../x.md)` becomes a hashtag).
    BodyOptions {
        link_resolution: LinkResolution::On {
            prefixes: vec!["./".to_string(), "../".to_string()],
        },
    }
}

impl MarkdownParser {
    /// Parses the body of one item and returns the AST JSON. An empty body gives an empty string.
    pub fn parse_to_ast(&self, markdown: &str) -> String {
        if markdown.is_empty() {
            return String::new();
        }
        flatmarkdown::parse_body_with(markdown, &options()).to_json()
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
