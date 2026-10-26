//! Extracts the target paths of wikilinks from a parsed AST JSON string.

use serde_json::Value;

/// Parses the AST JSON and collects the page path of every wikilink, deduplicated and in
/// order. `[[a/b#id|label]]` gives `a/b`: the fragment names an item, not the page. Invalid
/// JSON gives an empty list.
pub fn extract_wikilink_paths(ast_json: &str) -> Vec<String> {
    if ast_json.trim().is_empty() {
        return Vec::new();
    }
    let Ok(root) = serde_json::from_str::<Value>(ast_json) else {
        return Vec::new();
    };
    let mut paths: Vec<String> = Vec::new();
    collect(&root, &mut paths);
    paths
}

fn collect(node: &Value, paths: &mut Vec<String>) {
    match node {
        Value::Array(items) => {
            for item in items {
                collect(item, paths);
            }
        }
        Value::Object(map) => {
            if map.get("type").and_then(Value::as_str) == Some("wikilink") {
                if let Some(url) = map.get("url").and_then(Value::as_str) {
                    let path = url.split('#').next().unwrap_or("").trim();
                    if !path.is_empty() && !paths.iter().any(|p| p == path) {
                        paths.push(path.to_string());
                    }
                }
            }
            for val in map.values() {
                if val.is_array() || val.is_object() {
                    collect(val, paths);
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
    fn collects_the_paths_without_the_fragment() {
        let ast = r#"{"type":"document","children":[{"type":"paragraph","children":[{"type":"wikilink","url":"Wiki","children":[]},{"type":"wikilink","url":"a/b#id","children":[{"type":"text","value":"label"}]},{"type":"wikilink","url":"Wiki","children":[]}]}]}"#;
        assert_eq!(extract_wikilink_paths(ast), vec!["Wiki", "a/b"]);
    }

    #[test]
    fn empty_and_invalid() {
        assert!(extract_wikilink_paths("").is_empty());
        assert!(extract_wikilink_paths("not json").is_empty());
    }
}
