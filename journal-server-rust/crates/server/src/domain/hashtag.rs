//! Extracts hashtag values from a parsed AST JSON string.

use serde_json::Value;

/// Parses the AST JSON and collects all hashtag values, deduplicated and in order
/// (without the `#` prefix). Invalid JSON gives an empty list.
pub fn extract_hashtags(ast_json: &str) -> Vec<String> {
    if ast_json.trim().is_empty() {
        return Vec::new();
    }
    let Ok(root) = serde_json::from_str::<Value>(ast_json) else {
        return Vec::new();
    };
    let mut tags: Vec<String> = Vec::new();
    collect(&root, &mut tags);
    tags
}

fn collect(node: &Value, tags: &mut Vec<String>) {
    match node {
        Value::Array(items) => {
            for item in items {
                collect(item, tags);
            }
        }
        Value::Object(map) => {
            if map.get("type").and_then(Value::as_str) == Some("hashtag") {
                if let Some(value) = map.get("value").and_then(Value::as_str) {
                    if !value.is_empty() && !tags.iter().any(|t| t == value) {
                        tags.push(value.to_string());
                    }
                }
            }
            // Children first, so tags keep their order in the text; then every other
            // nested field in case the AST nests differently
            if let Some(children) = map.get("children") {
                collect(children, tags);
            }
            for (key, val) in map {
                if key != "children" && (val.is_array() || val.is_object()) {
                    collect(val, tags);
                }
            }
        }
        _ => {}
    }
}

/// Whether `tag` can be written as a hashtag: letters, digits, `_`, `-` and `/` as the parser
/// takes them, with no `/` at either end.
pub fn is_valid(tag: &str) -> bool {
    !tag.is_empty()
        && !tag.starts_with('/')
        && !tag.ends_with('/')
        && tag.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/'))
}

/// The tag as it reads once the tag `old` is named `new`: `old` itself, or a tag under it like
/// `old/x`. Letters are matched ignoring ASCII case, as tags that differ only in case share
/// one tag page. None when the tag is neither.
pub fn renamed(tag: &str, old: &str, new: &str) -> Option<String> {
    let head = tag.get(..old.len())?;
    let rest = &tag[old.len()..];
    (head.eq_ignore_ascii_case(old) && (rest.is_empty() || rest.starts_with('/'))).then(|| format!("{new}{rest}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_in_order_without_duplicates() {
        let ast = r#"{"type":"root","children":[{"type":"hashtag","value":"b"},{"type":"paragraph","children":[{"type":"hashtag","value":"a"},{"type":"hashtag","value":"b"}]}]}"#;
        assert_eq!(extract_hashtags(ast), vec!["b", "a"]);
    }

    #[test]
    fn empty_and_invalid() {
        assert!(extract_hashtags("").is_empty());
        assert!(extract_hashtags("not json").is_empty());
    }

    #[test]
    fn valid_tags() {
        assert!(is_valid("a/b-c_1"));
        assert!(is_valid("日本語"));
        assert!(!is_valid("a b"));
        assert!(!is_valid("/a"));
        assert!(!is_valid("a/"));
        assert!(!is_valid(""));
    }

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
