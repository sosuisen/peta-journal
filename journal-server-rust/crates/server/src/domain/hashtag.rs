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
}
