//! Extracts the target paths of wikilinks from a parsed AST.

use flatmarkdown_ast::Node;

/// Collects the page path of every wikilink under `ast`, deduplicated and in order.
/// `[[a/b#<item id>|label]]` gives `a/b`: the item id names an item, not the page. Any other
/// `#` is a part of the name, as in `[[C#]]`.
pub fn extract_wikilink_paths(ast: &Node) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    collect(ast, &mut paths);
    paths
}

fn collect(node: &Node, paths: &mut Vec<String>) {
    if let Node::Wikilink { url, .. } = node {
        let path = page_urls::split_wikilink(url).0.trim();
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

    // パスは項目のidを除いて、重複なく集められる。
    #[test]
    fn collects_the_paths_without_the_item_id() {
        let ast = MarkdownParser.parse("[[Wiki]] [[a/b#20260426035700_QZ998PX|label]] [[Wiki]]");
        assert_eq!(extract_wikilink_paths(&ast), vec!["Wiki", "a/b"]);
    }

    // 名前の#はページ名の一部で、項目のidの前の#だけが区切りである。
    #[test]
    fn a_hash_is_a_part_of_the_name_unless_an_item_id_follows_it() {
        let ast = MarkdownParser.parse("[[C#]] [[C##20260426035700_QZ998PX]] [[F#入門]]");
        assert_eq!(extract_wikilink_paths(&ast), vec!["C#", "F#入門"]);
    }

    // 空の本文からはパスが取れない。
    #[test]
    fn an_empty_body_gives_no_paths() {
        assert!(extract_wikilink_paths(&MarkdownParser.parse("")).is_empty());
    }
}
