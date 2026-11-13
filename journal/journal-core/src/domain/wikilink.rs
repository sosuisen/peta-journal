//! Extracts the target paths of wikilinks from a parsed AST.

use flatmarkdown_ast::Node;

/// Collects the page path of every wikilink under `ast`, deduplicated and in order.
/// `[[a/b#id|label]]` gives `a/b`: the fragment names an item, not the page.
pub fn extract_wikilink_paths(ast: &Node) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    collect(ast, &mut paths);
    paths
}

fn collect(node: &Node, paths: &mut Vec<String>) {
    if let Node::Wikilink { url, .. } = node {
        let path = url.split('#').next().unwrap_or("").trim();
        if !path.is_empty() && !paths.iter().any(|p| p == path) {
            paths.push(path.to_string());
        }
    }
    for child in node.children() {
        collect(child, paths);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::application::markdown_parser::MarkdownParser;

    // パスはフラグメントを除いて、重複なく集められる。
    #[test]
    fn collects_the_paths_without_the_fragment() {
        let ast = MarkdownParser.parse("[[Wiki]] [[a/b#id|label]] [[Wiki]]");
        assert_eq!(extract_wikilink_paths(&ast), vec!["Wiki", "a/b"]);
    }

    // 空の本文からはパスが取れない。
    #[test]
    fn an_empty_body_gives_no_paths() {
        assert!(extract_wikilink_paths(&MarkdownParser.parse("")).is_empty());
    }
}
