//! Parses a Flat Markdown page (spec v0.2) into its front matter and its items.
//! The item split is done by the splitter; this maps the result: YAML front matter to key-value
//! pairs, and the metadata line to item fields.

use std::collections::BTreeMap;

use crate::application::flat_markdown_serializer::{COLLAPSED_KEY, IDX_KEY, TRANSCLUDE_KEY};
use crate::application::flat_markdown_splitter::{FlatMarkdownSplitter, SplitItem};
use crate::domain::item_body::{ItemBody, TASK_TYPE};

/// Front matter and items of one parsed page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    pub frontmatter: BTreeMap<String, String>,
    pub items: Vec<ParsedItem>,
}

/// One item as written in the file. `id`, `transclude_item_id` and `idx` are None when absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedItem {
    pub id: Option<String>,
    pub level: i32,
    pub body: String,
    pub collapsed: bool,
    pub types: Vec<String>,
    /// Properties in file order, without the reserved keys.
    pub properties: Vec<(String, String)>,
    pub transclude_item_id: Option<String>,
    pub idx: Option<String>,
}

impl ParsedItem {
    /// An item with an empty body, used for a file without any item.
    pub fn empty() -> ParsedItem {
        ParsedItem {
            id: None,
            level: 1,
            body: String::new(),
            collapsed: false,
            types: Vec::new(),
            properties: Vec::new(),
            transclude_item_id: None,
            idx: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OutlineMarkdownParser {
    splitter: FlatMarkdownSplitter,
}

impl OutlineMarkdownParser {
    pub fn new(splitter: FlatMarkdownSplitter) -> OutlineMarkdownParser {
        OutlineMarkdownParser { splitter }
    }

    /// Parses a whole page. An empty source gives an empty result.
    pub fn parse(&self, markdown: &str) -> anyhow::Result<ParseResult> {
        let page = self.splitter.split(markdown)?;
        let frontmatter = match page.front_matter {
            None => BTreeMap::new(),
            Some(raw) => parse_frontmatter(&strip_frontmatter_delimiters(&raw)),
        };
        let items = page.items.iter().map(to_parsed_item).collect();
        Ok(ParseResult { frontmatter, items })
    }
}

/// Removes the `---` lines around the front matter body.
fn strip_frontmatter_delimiters(raw: &str) -> String {
    let mut s = raw.trim();
    if let Some(rest) = s.strip_prefix("---") {
        s = rest;
    }
    if let Some(rest) = s.strip_suffix("---") {
        s = rest;
    }
    s.trim().to_string()
}

fn parse_frontmatter(frontmatter: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if frontmatter.is_empty() {
        return map;
    }
    let mut list_key: Option<String> = None;
    let mut list_values: Vec<String> = Vec::new();
    for line in frontmatter.split('\n') {
        if let (Some(_), Some(rest)) = (&list_key, line.strip_prefix("  - ")) {
            list_values.push(rest.trim().to_string());
            continue;
        }
        if let Some(key) = list_key.take() {
            map.insert(key, list_values.join(","));
            list_values.clear();
        }
        if let Some(colon) = line.find(':') {
            if colon > 0 {
                let key = line[..colon].trim().to_string();
                let mut value = line[colon + 1..].trim().to_string();
                if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                    value = value[1..value.len() - 1].replace("\\\"", "\"");
                }
                if value.is_empty() {
                    list_key = Some(key);
                    list_values.clear();
                } else {
                    map.insert(key, value);
                }
            }
        }
    }
    if let Some(key) = list_key {
        map.insert(key, list_values.join(","));
    }
    map
}

fn to_parsed_item(item: &SplitItem) -> ParsedItem {
    let mut idx = None;
    let mut collapsed = false;
    let mut transclude = None;
    let mut properties = Vec::new();
    for (k, v) in &item.properties {
        match k.as_str() {
            IDX_KEY => idx = Some(v.clone()),
            COLLAPSED_KEY => collapsed = v == "true",
            TRANSCLUDE_KEY => transclude = Some(v.clone()),
            _ => properties.push((k.clone(), v.clone())),
        }
    }
    let body = ItemBody::parse(Some(&item.body));
    let mut types = Vec::new();
    if body.task {
        types.push(TASK_TYPE.to_string());
    }
    types.extend(body.text_types());
    ParsedItem {
        id: item.id.clone(),
        level: item.level,
        body: body.text,
        collapsed,
        types,
        properties,
        transclude_item_id: transclude,
        idx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // フロントマターと項目が解析される。
    #[test]
    fn parses_frontmatter_and_items() {
        let md = "---\nid: 2026-09-29-00-00-00-AAAAAAA\ncreated: 2026-09-29T00:00:00Z\ntags:\n  - a\n  - b\n---\n\n<!-- @ 2026-09-29-00-00-01-AAAAAAA idx=\"z0\" plan=\"2026-09-30\" -->\n- [ ] task\n";
        let r = OutlineMarkdownParser::default().parse(md).unwrap();
        assert_eq!(
            r.frontmatter.get("id").map(String::as_str),
            Some("2026-09-29-00-00-00-AAAAAAA")
        );
        assert_eq!(r.frontmatter.get("tags").map(String::as_str), Some("a,b"));
        assert_eq!(r.items.len(), 1);
        let it = &r.items[0];
        assert_eq!(it.body, "task");
        assert_eq!(it.types, vec!["task"]);
        assert_eq!(it.idx.as_deref(), Some("z0"));
        assert_eq!(
            it.properties,
            vec![("plan".to_string(), "2026-09-30".to_string())]
        );
    }
}
