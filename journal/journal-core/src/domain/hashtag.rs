//! Extracts hashtag values from a parsed AST.

use flatmarkdown_ast::Node;

/// Collects all hashtag values under `ast`, deduplicated and in the order of the text
/// (without the `#` prefix).
pub fn extract_hashtags(ast: &Node) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    collect(ast, &mut tags);
    tags
}

fn collect(node: &Node, tags: &mut Vec<String>) {
    if let Node::Hashtag { value } = node {
        if !value.is_empty() && !tags.iter().any(|t| t == value) {
            tags.push(value.clone());
        }
    }
    for child in node.children() {
        collect(child, tags);
    }
}

/// Whether `tag` can be written as a hashtag: letters, digits, `_`, `-` and `/` as the parser
/// takes them, with no `/` at either end.
pub fn is_valid(tag: &str) -> bool {
    !tag.is_empty()
        && !tag.starts_with('/')
        && !tag.ends_with('/')
        && tag
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/'))
}

/// The tag as it reads once the tag `old` is named `new`: `old` itself, or a tag under it like
/// `old/x`. Letters are matched ignoring ASCII case, as tags that differ only in case share
/// one tag page. None when the tag is neither.
pub fn renamed(tag: &str, old: &str, new: &str) -> Option<String> {
    let head = tag.get(..old.len())?;
    let rest = &tag[old.len()..];
    (head.eq_ignore_ascii_case(old) && (rest.is_empty() || rest.starts_with('/')))
        .then(|| format!("{new}{rest}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::application::markdown_parser::MarkdownParser;

    // タグは現れた順に、重複なく集められる。リストの中のタグも集められる。
    #[test]
    fn collects_in_order_without_duplicates() {
        let ast = MarkdownParser.parse("#b and #a\n\n- #b and #c");
        assert_eq!(extract_hashtags(&ast), vec!["b", "a", "c"]);
    }

    // 空の本文からはタグが取れない。
    #[test]
    fn an_empty_body_gives_no_tags() {
        assert!(extract_hashtags(&MarkdownParser.parse("")).is_empty());
    }

    // 正しいタグは、空白を含まず、両端がスラッシュでない。
    #[test]
    fn a_tag_has_no_space_and_no_slash_at_either_end() {
        assert!(is_valid("a/b-c_1"));
        assert!(is_valid("日本語"));
        assert!(!is_valid("a b"));
        assert!(!is_valid("/a"));
        assert!(!is_valid("a/"));
        assert!(!is_valid(""));
    }

    // タグとその下のタグが改名される。
    #[test]
    fn renames_the_tag_and_the_tags_under_it() {
        assert_eq!(renamed("old", "old", "new").as_deref(), Some("new"));
        assert_eq!(renamed("Old", "old", "new").as_deref(), Some("new"));
        assert_eq!(renamed("old/x", "old", "new").as_deref(), Some("new/x"));
        assert_eq!(renamed("older", "old", "new"), None);
        assert_eq!(renamed("ol", "old", "new"), None);
        assert_eq!(renamed("x/old", "old", "new"), None);
    }
}
