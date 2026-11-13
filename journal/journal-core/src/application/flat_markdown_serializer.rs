//! Serializes items to Flat Markdown (spec v0.2).
//! Every item is written as one metadata line followed by its body:
//! `<!-- @@ ID idx="a0" collapsed="true" transclude="ID" key="value" -->`.
//! The number of `@` characters is the item level. Items are separated by a blank line.

use std::collections::BTreeMap;

use crate::domain::model::Item;
use crate::infrastructure::persistence::property_repository::{ListProps, ScalarProps};

/// Metadata key that holds the fractional index of the item.
pub const IDX_KEY: &str = "idx";
/// Metadata key that marks a collapsed item.
pub const COLLAPSED_KEY: &str = "collapsed";
/// Metadata key that holds the source item ID of a transclusion.
pub const TRANSCLUDE_KEY: &str = "transclude";
/// Metadata key that holds when the item last changed, in the file of a page.
pub const MODIFIED_KEY: &str = "modified";

static EMPTY_SCALARS: BTreeMap<String, String> = BTreeMap::new();
static EMPTY_LISTS: BTreeMap<String, Vec<String>> = BTreeMap::new();

/// Serializes items in order, including their properties.
pub fn serialize(items: &[Item], scalar_props: &ScalarProps, list_props: &ListProps) -> String {
    serialize_with(items, scalar_props, list_props, &|s| s.to_string(), &|_| {
        None
    })
}

/// Serializes items in order, passing each body through `body_mapper` before it is written.
/// `date` gives the value of the `modified` key of an item, if it is to have one.
pub fn serialize_with(
    items: &[Item],
    scalar_props: &ScalarProps,
    list_props: &ListProps,
    body_mapper: &dyn Fn(&str) -> String,
    date: &dyn Fn(&Item) -> Option<String>,
) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        let id = item.id.to_string();
        if i > 0 {
            out.push('\n');
        }
        append_item(
            &mut out,
            item,
            scalar_props.get(&id).unwrap_or(&EMPTY_SCALARS),
            list_props.get(&id).unwrap_or(&EMPTY_LISTS),
            body_mapper,
            date(item).as_deref(),
        );
    }
    out
}

/// Appends one item as a metadata line and its body, each ending with a newline.
pub fn append_item(
    out: &mut String,
    item: &Item,
    scalars: &BTreeMap<String, String>,
    lists: &BTreeMap<String, Vec<String>>,
    body_mapper: &dyn Fn(&str) -> String,
    modified: Option<&str>,
) {
    out.push_str(&metadata_line(item, scalars, lists, modified));
    out.push('\n');
    let body = body_mapper(&body_with_task_prefix(item, scalars));
    // An empty body is expressed by the metadata line alone
    if !body.is_empty() {
        out.push_str(&body);
        out.push('\n');
    }
}

/// Builds the metadata line of an item. Properties are written in a fixed order: idx, collapsed,
/// transclude, modified, then the remaining keys sorted alphabetically.
pub fn metadata_line(
    item: &Item,
    scalars: &BTreeMap<String, String>,
    lists: &BTreeMap<String, Vec<String>>,
    modified: Option<&str>,
) -> String {
    let mut sb = String::from("<!-- ");
    sb.push_str(&"@".repeat(item.level.max(1) as usize));
    sb.push(' ');
    sb.push_str(item.id.as_str());
    append_property(&mut sb, IDX_KEY, &item.idx);
    if item.collapsed {
        append_property(&mut sb, COLLAPSED_KEY, "true");
    }
    if let Some(t) = &item.transclude_item_id {
        append_property(&mut sb, TRANSCLUDE_KEY, t.as_str());
    }
    if let Some(modified) = modified {
        append_property(&mut sb, MODIFIED_KEY, modified);
    }
    let mut others: BTreeMap<String, String> = scalars.clone();
    for (key, values) in lists {
        others.insert(key.clone(), values.join(","));
    }
    for (key, value) in &others {
        if modified.is_some() && key == MODIFIED_KEY {
            continue;
        }
        append_property(&mut sb, key, value);
    }
    sb.push_str(" -->");
    sb
}

/// The body with the GFM task prefix when the item is a task:
/// `- [x] ` when the task has an `end` property, otherwise `- [ ] `.
pub fn body_with_task_prefix(item: &Item, scalars: &BTreeMap<String, String>) -> String {
    if item.types.iter().any(|t| t == "task") {
        let prefix = if scalars.contains_key("end") {
            "- [x] "
        } else {
            "- [ ] "
        };
        return format!("{prefix}{}", item.body);
    }
    item.body.clone()
}

fn append_property(sb: &mut String, key: &str, value: &str) {
    sb.push(' ');
    sb.push_str(key);
    sb.push_str("=\"");
    sb.push_str(&escape(value));
    sb.push('"');
}

/// Escapes a property value: `\` becomes `\\` and `"` becomes `\"`.
pub fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmtid::Hmtid;

    fn item(level: i32, body: &str, types: &[&str]) -> Item {
        Item {
            id: Hmtid::parse("2026-09-29-00-00-01-AAAAAAA").unwrap(),
            page_id: Hmtid::parse("2026-09-29-00-00-00-AAAAAAA").unwrap(),
            parent_item_id: None,
            dir_path: String::new(),
            name: "p".into(),
            item_type: "doc".into(),
            created_date: 0,
            modified_date: 0,
            idx: "z0".into(),
            level,
            body: body.into(),
            ast: String::new(),
            collapsed: false,
            types: types.iter().map(|s| s.to_string()).collect(),
            transclude_item_id: None,
        }
    }

    // メタデータの行と本文が書き出される。
    #[test]
    fn writes_metadata_and_body() {
        let items = vec![item(2, "hello", &[])];
        let s = serialize(&items, &ScalarProps::new(), &ListProps::new());
        assert_eq!(
            s,
            "<!-- @@ 2026-09-29-00-00-01-AAAAAAA idx=\"z0\" -->\nhello\n"
        );
    }

    // タスクには接頭辞が付き、プロパティはメタデータの行に書かれる。
    #[test]
    fn a_task_takes_its_prefix_and_its_properties_go_on_the_metadata_line() {
        let it = item(1, "do it", &["task"]);
        let mut scalars = BTreeMap::new();
        scalars.insert("plan".to_string(), "2026-09-27".to_string());
        let mut lists = BTreeMap::new();
        lists.insert("tags".to_string(), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            metadata_line(&it, &scalars, &lists, None),
            "<!-- @ 2026-09-29-00-00-01-AAAAAAA idx=\"z0\" plan=\"2026-09-27\" tags=\"a,b\" -->"
        );
        assert_eq!(body_with_task_prefix(&it, &scalars), "- [ ] do it");
        scalars.insert("end".to_string(), "x".to_string());
        assert_eq!(body_with_task_prefix(&it, &scalars), "- [x] do it");
    }

    // 値の引用符とバックスラッシュはエスケープされる。
    #[test]
    fn quotes_and_backslashes_in_values_are_escaped() {
        assert_eq!(escape("a\"b\\c"), "a\\\"b\\\\c");
    }
}
