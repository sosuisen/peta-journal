//! Parses a Flat Markdown page (spec v0.2) into its front matter and its items.
//! The item split is done by the splitter; this maps the result: YAML front matter to key-value
//! pairs, and the metadata line to item fields.

use std::collections::BTreeMap;

use crate::application::flat_markdown_serializer::{
    COLLAPSED_KEY, IDX_KEY, MODIFIED_KEY, TRANSCLUDE_KEY,
};
use crate::application::flat_markdown_splitter::{FlatMarkdownSplitter, SplitItem};
use crate::domain::item_body::{ItemBody, TASK_TYPE};

use super::page_matcher::parse_instant;

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
    /// When the item last changed. A file written before items had dates has none.
    pub modified: Option<i64>,
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
            modified: None,
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
    let mut modified = None;
    let mut properties = Vec::new();
    for (k, v) in &item.properties {
        match k.as_str() {
            IDX_KEY => idx = Some(v.clone()),
            COLLAPSED_KEY => collapsed = v == "true",
            TRANSCLUDE_KEY => transclude = Some(v.clone()),
            MODIFIED_KEY if parse_instant(Some(v)).is_some() => {
                modified = parse_instant(Some(v));
            }
            _ => properties.push((k.clone(), v.clone())),
        }
    }
    let body = ItemBody::parse(&item.body);
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
        modified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 項目の`modified`は日時なら更新日になり、そうでなければプロパティのままである。
    #[test]
    fn modified_is_the_date_of_the_item_only_when_it_is_a_date() {
        let md = "<!-- @ 2026-09-29-00-00-01-AAAAAAA modified=\"1970-01-01T00:00:01Z\" -->\na\n\n<!-- @ 2026-09-29-00-00-02-AAAAAAA modified=\"soon\" -->\nb\n";
        let r = OutlineMarkdownParser::default().parse(md).unwrap();
        assert_eq!(r.items[0].modified, Some(1000));
        assert!(r.items[0].properties.is_empty());
        assert_eq!(r.items[1].modified, None);
        assert_eq!(
            r.items[1].properties,
            vec![("modified".to_string(), "soon".to_string())]
        );
    }

    // ファイルの項目の本文は、前後の空白だけの行を持たない。空白だけの本文は空になる。Markdownでは、ブロックの前後の空行に意味がないためである。1行目の行頭と最後の行の行末の空白は残る。行頭の4つの空白はコードブロックを表す。
    #[test]
    fn a_body_read_from_a_file_has_no_blank_lines_before_and_after() {
        for (written, read) in [
            (" ", ""),
            ("\t", ""),
            ("\n\ntext\n\n", "text"),
            (" \n  text\nmore  \n ", "  text\nmore  "),
            ("    code", "    code"),
        ] {
            let md = format!(
                "<!-- @ 2026-09-29-00-00-01-AAAAAAA -->\n{written}\n\n<!-- @ 2026-09-29-00-00-02-AAAAAAA -->\nnext\n"
            );
            let r = OutlineMarkdownParser::default().parse(&md).unwrap();
            assert_eq!(r.items[0].body, read, "{written:?}");
        }
    }

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
