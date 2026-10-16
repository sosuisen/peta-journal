//! Converts between items, Tree Markdown (nested list format) and Flat Markdown.

use crate::domain::model::Item;
use crate::infrastructure::persistence::property_repository::{ListProps, ScalarProps};

use super::flat_markdown_serializer;

/// Converts a list of items to Tree Markdown, with levels relative to the minimum level.
pub fn items_to_tree_markdown(items: &[Item]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let min_level = items.iter().map(|i| i.level).min().unwrap_or(1);
    let mut sb = String::new();
    for (i, item) in items.iter().enumerate() {
        let depth = (item.level - min_level).max(0) as usize;
        let indent = "  ".repeat(depth);
        let mut lines = item.body.split('\n');
        sb.push_str(&indent);
        sb.push_str("- ");
        sb.push_str(lines.next().unwrap_or(""));
        for line in lines {
            sb.push('\n');
            sb.push_str(&indent);
            sb.push_str("  ");
            sb.push_str(line);
        }
        if i < items.len() - 1 {
            sb.push('\n');
        }
    }
    sb
}

/// Converts a list of items to Flat Markdown, including properties in the metadata line.
pub fn items_to_flat_markdown(items: &[Item], scalar_props: &ScalarProps, list_props: &ListProps) -> String {
    flat_markdown_serializer::serialize(items, scalar_props, list_props)
}

struct Marker {
    pos: usize,
    level: i32,
}

/// Parses Tree Markdown into Flat Markdown. Each list item becomes a separate item; items deeper
/// than the top level get a metadata line whose number of `@` characters is the level.
pub fn tree_markdown_to_flat_markdown(tree_md: &str) -> String {
    if tree_md.is_empty() {
        return String::new();
    }
    let text = tree_md.replace('\r', "");
    let mut items: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    let mut item_prefix_len = 0usize;
    let mut current_level = 0i32;

    for line in text.split('\n') {
        if let Some(marker) = find_list_marker_pos(line) {
            if let Some(body) = current.take() {
                items.push(flat_item(&body, current_level));
            }
            current_level = marker.level;
            item_prefix_len = marker.pos + 2; // indent + "- "
            let mut first_line = line[item_prefix_len..].to_string();
            // Re-add "- " for GFM task prefix so flat markdown parser can detect task type
            if first_line.starts_with("[ ] ") || first_line.starts_with("[x] ") {
                first_line = format!("- {first_line}");
            }
            current = Some(first_line);
        } else if let Some(body) = current.as_mut() {
            if line.chars().count() >= item_prefix_len {
                // Continuation line: strip the item's full prefix length
                let content: String = line.chars().skip(item_prefix_len).collect();
                body.push('\n');
                body.push_str(&content);
            }
        }
    }
    if let Some(body) = current.take() {
        items.push(flat_item(&body, current_level));
    }
    items.join("\n\n")
}

/// One Flat Markdown item: the body, preceded by a metadata line when it is nested.
fn flat_item(body: &str, level: i32) -> String {
    if level > 1 {
        format!("<!-- {} -->\n{body}", "@".repeat(level as usize))
    } else {
        body.to_string()
    }
}

/// Finds the "- " list marker after optional indent (two spaces or a tab per level).
fn find_list_marker_pos(line: &str) -> Option<Marker> {
    let chars: Vec<char> = line.chars().collect();
    let mut pos = 0usize;
    let mut level = 1i32;
    while pos < chars.len() {
        if pos + 1 < chars.len() && chars[pos] == '-' && chars[pos + 1] == ' ' {
            return Some(Marker { pos, level });
        } else if pos + 1 < chars.len() && chars[pos] == ' ' && chars[pos + 1] == ' ' {
            pos += 2;
            level += 1;
        } else if chars[pos] == '\t' {
            pos += 1;
            level += 1;
        } else {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_to_flat() {
        let flat = tree_markdown_to_flat_markdown("- a\n  - b\n    second line\n- [ ] c");
        assert_eq!(flat, "a\n\n<!-- @@ -->\nb\nsecond line\n\n- [ ] c");
    }

    #[test]
    fn empty() {
        assert_eq!(tree_markdown_to_flat_markdown(""), "");
    }
}
