//! Splits a Flat Markdown page into items by the rules of spec v0.2.
//! The split itself is done by the flatmarkdown crate; this maps its page JSON to plain
//! records and drops the per-item AST, which callers do not need.

use std::collections::BTreeMap;

use serde_json::Value;

use super::markdown_parser::MarkdownParser;

/// One item of a page as written in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitItem {
    /// The indent level; 1 when the metadata line is omitted.
    pub level: i32,
    /// The HMTID from the metadata line, or None when absent.
    pub id: Option<String>,
    /// The key-value pairs from the metadata line, in source order; empty when absent.
    pub properties: Vec<(String, String)>,
    /// The Markdown source of the item, without leading and trailing blank lines.
    pub body: String,
}

impl SplitItem {
    pub fn properties_map(&self) -> BTreeMap<String, String> {
        self.properties.iter().cloned().collect()
    }
}

/// A page split into front matter and items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPage {
    /// The raw YAML front matter including the `---` lines, or None when absent.
    pub front_matter: Option<String>,
    pub items: Vec<SplitItem>,
}

#[derive(Debug, Clone, Default)]
pub struct FlatMarkdownSplitter {
    parser: MarkdownParser,
}

impl FlatMarkdownSplitter {
    pub fn new(parser: MarkdownParser) -> FlatMarkdownSplitter {
        FlatMarkdownSplitter { parser }
    }

    /// Splits a page. An empty source gives a page without items.
    pub fn split(&self, markdown: &str) -> anyhow::Result<SplitPage> {
        if markdown.is_empty() {
            return Ok(SplitPage {
                front_matter: None,
                items: Vec::new(),
            });
        }
        let page: Value = serde_json::from_str(&self.parser.parse_page(markdown))
            .map_err(|e| anyhow::anyhow!("flatmarkdown returned invalid page JSON: {e}"))?;
        let front_matter = page
            .get("front_matter")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let items = page
            .get("items")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(to_split_item).collect())
            .unwrap_or_default();
        Ok(SplitPage { front_matter, items })
    }
}

fn to_split_item(node: &Value) -> SplitItem {
    let id = node.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let properties = node
        .get("properties")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| {
                    let value = match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    (k.clone(), value)
                })
                .collect()
        })
        .unwrap_or_default();
    SplitItem {
        level: node.get("level").and_then(|v| v.as_i64()).unwrap_or(1) as i32,
        id,
        properties,
        body: node
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_items_with_metadata() {
        let md = "---\nid: 2026-09-29-00-00-00-AAAAAAA\n---\n\n<!-- @ 2026-09-29-00-00-01-AAAAAAA idx=\"z0\" -->\nfirst\n\n<!-- @@ 2026-09-29-00-00-02-AAAAAAA idx=\"z1\" collapsed=\"true\" -->\nsecond\n";
        let page = FlatMarkdownSplitter::default().split(md).unwrap();
        assert!(page.front_matter.is_some());
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].level, 1);
        assert_eq!(page.items[0].body, "first");
        assert_eq!(page.items[1].level, 2);
        assert_eq!(page.items[1].properties_map().get("collapsed").map(String::as_str), Some("true"));
    }

    #[test]
    fn plain_text_is_level_one() {
        let page = FlatMarkdownSplitter::default().split("hello\n\nworld\n").unwrap();
        assert_eq!(page.items.len(), 2);
        assert!(page.items[0].id.is_none());
    }

    #[test]
    fn empty_gives_no_items() {
        let page = FlatMarkdownSplitter::default().split("").unwrap();
        assert!(page.items.is_empty());
    }
}
