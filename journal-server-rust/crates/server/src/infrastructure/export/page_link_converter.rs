//! Converts page links between the in-app form and the on-disk form of a page body.
//! In the app a body holds wikilinks (`[[path]]`, `[[path#id|label]]`) and hashtags (`#tag`).
//! On disk they become Markdown links whose destination is the path of the target file relative
//! to the file being written. Text inside fenced code blocks and inline code spans is never converted.

use std::sync::LazyLock;

use regex::{Captures, Regex};

use crate::domain::journal_name;

use super::path_mapper;

static WIKILINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\[\]|]+?)(?:\|([^\[\]]*))?\]\]").unwrap());
/// `(?<=^| )#tag`: the regex crate has no look-behind, so the preceding character is captured.
static HASHTAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)(^| )#([\p{L}\p{N}_/-]+)").unwrap());
/// `(?<!!)[label](dest)`: the preceding character is captured and checked for `!`.
static MARKDOWN_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|[^!])\[([^\[\]]*)\]\((<[^<>]*>|[^()\s]+)\)").unwrap());
static INLINE_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`+").unwrap());
static FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s{0,3}(```|~~~)").unwrap());

/// Rewrites wikilinks and hashtags as relative Markdown links for the file at `source_rel_path`.
pub fn to_markdown_links(body: &str, source_rel_path: &str) -> String {
    let source_dir = dir_of(source_rel_path);
    map_outside_code(body, &|text| hashtags_to_links(&wikilinks_to_links(text, source_dir), source_dir))
}

/// Rewrites relative Markdown links that point at a page file back into wikilinks and hashtags.
pub fn to_wikilinks(body: &str, source_rel_path: &str) -> String {
    let source_dir = dir_of(source_rel_path);
    map_outside_code(body, &|text| links_to_wikilinks(text, source_dir))
}

// ---- export direction ----

fn wikilinks_to_links(text: &str, source_dir: &str) -> String {
    WIKILINK
        .replace_all(text, |m: &Captures| {
            let target = m.get(1).map(|g| g.as_str()).unwrap_or("");
            let label = m.get(2).map(|g| g.as_str());
            let (page_path, fragment) = match target.find('#') {
                Some(pos) => (&target[..pos], &target[pos..]),
                None => (target, ""),
            };
            let dest = format!("{}{}", relativize(source_dir, &page_file_of(page_path)), fragment);
            markdown_link(label.unwrap_or(target), &dest)
        })
        .into_owned()
}

fn hashtags_to_links(text: &str, source_dir: &str) -> String {
    HASHTAG
        .replace_all(text, |m: &Captures| {
            let lead = m.get(1).map(|g| g.as_str()).unwrap_or("");
            let raw = m.get(2).map(|g| g.as_str()).unwrap_or("");
            let tag = trim_trailing_slashes(raw);
            if tag.is_empty() || tag.starts_with('/') {
                return m.get(0).map(|g| g.as_str()).unwrap_or("").to_string();
            }
            let rest = &raw[tag.len()..];
            format!(
                "{lead}{}{rest}",
                markdown_link(&format!("#{tag}"), &relativize(source_dir, &page_file_of(tag)))
            )
        })
        .into_owned()
}

fn markdown_link(label: &str, dest: &str) -> String {
    let needs_brackets = dest.chars().any(|c| matches!(c, ' ' | '(' | ')' | '<' | '>'));
    if needs_brackets {
        format!("[{label}](<{dest}>)")
    } else {
        format!("[{label}]({dest})")
    }
}

/// Maps a page path (dirPath + name) to its file path under the markdown root.
fn page_file_of(page_path: &str) -> String {
    let (dir_path, name) = match page_path.rfind('/') {
        Some(pos) => (&page_path[..=pos], &page_path[pos + 1..]),
        None => ("", page_path),
    };
    let page_type = if dir_path.is_empty() && journal_name::is_journal_file_name(name) {
        "journal"
    } else {
        "doc"
    };
    path_mapper::to_rel_path(page_type, dir_path, name)
}

// ---- import direction ----

fn links_to_wikilinks(text: &str, source_dir: &str) -> String {
    MARKDOWN_LINK
        .replace_all(text, |m: &Captures| {
            let lead = m.get(1).map(|g| g.as_str()).unwrap_or("");
            let whole = m.get(0).map(|g| g.as_str()).unwrap_or("");
            let label = m.get(2).map(|g| g.as_str()).unwrap_or("");
            let dest = unwrap_brackets(m.get(3).map(|g| g.as_str()).unwrap_or(""));
            let (file, fragment) = match dest.find('#') {
                Some(pos) => (&dest[..pos], &dest[pos..]),
                None => (dest, ""),
            };
            match resolve(source_dir, file).and_then(|p| page_path_of(&p)) {
                None => whole.to_string(),
                Some(page_path) => format!("{lead}{}", wikilink_or_hashtag(label, &page_path, fragment)),
            }
        })
        .into_owned()
}

fn wikilink_or_hashtag(label: &str, page_path: &str, fragment: &str) -> String {
    if fragment.is_empty() && label == format!("#{page_path}") {
        return format!("#{page_path}");
    }
    let target = format!("{page_path}{fragment}");
    if label == target || label == page_path {
        return format!("[[{target}]]");
    }
    format!("[[{target}|{label}]]")
}

fn page_path_of(rel_path: &str) -> Option<String> {
    path_mapper::from_rel_path(rel_path).map(|loc| format!("{}{}", loc.dir_path, loc.name))
}

fn unwrap_brackets(dest: &str) -> &str {
    if dest.starts_with('<') && dest.ends_with('>') && dest.len() >= 2 {
        &dest[1..dest.len() - 1]
    } else {
        dest
    }
}

// ---- path helpers (POSIX style, independent of the host file system) ----

fn dir_of(rel_path: &str) -> &str {
    match rel_path.rfind('/') {
        Some(pos) => &rel_path[..pos],
        None => "",
    }
}

/// The path of `target_rel_path` relative to `source_dir`; both are root-relative.
pub fn relativize(source_dir: &str, target_rel_path: &str) -> String {
    let from = segments(source_dir);
    let to = segments(target_rel_path);
    let mut common = 0;
    while common < from.len() && common + 1 < to.len() && from[common] == to[common] {
        common += 1;
    }
    let mut sb = String::new();
    for _ in common..from.len() {
        sb.push_str("../");
    }
    sb.push_str(&to[common..].join("/"));
    sb
}

/// Resolves `relative` against `source_dir`; None when it escapes the root.
pub fn resolve(source_dir: &str, relative: &str) -> Option<String> {
    if relative.is_empty() || relative.starts_with('/') || relative.contains("://") {
        return None;
    }
    let mut stack: Vec<String> = segments(source_dir).into_iter().map(|s| s.to_string()).collect();
    for seg in relative.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            stack.pop()?;
        } else {
            stack.push(seg.to_string());
        }
    }
    Some(stack.join("/"))
}

fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

fn trim_trailing_slashes(s: &str) -> &str {
    s.trim_end_matches('/')
}

// ---- code protection ----

/// Applies `f` to every stretch of text that is outside fenced blocks and inline code.
fn map_outside_code(body: &str, f: &dyn Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut text_run = String::new();
    let mut in_fence = false;
    let lines: Vec<&str> = body.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        let nl = if i < lines.len() - 1 { "\n" } else { "" };
        if FENCE.is_match(line) {
            flush(&mut out, &mut text_run, f);
            out.push_str(line);
            out.push_str(nl);
            in_fence = !in_fence;
        } else if in_fence {
            out.push_str(line);
            out.push_str(nl);
        } else {
            text_run.push_str(line);
            text_run.push_str(nl);
        }
    }
    flush(&mut out, &mut text_run, f);
    out
}

fn flush(out: &mut String, text_run: &mut String, f: &dyn Fn(&str) -> String) {
    if text_run.is_empty() {
        return;
    }
    // Inline code spans stay untouched; only the text between them is converted
    let text = std::mem::take(text_run);
    let mut last = 0;
    let mut pos = 0;
    while pos < text.len() {
        let Some(open) = INLINE_CODE.find_at(&text, pos) else { break };
        let ticks = open.as_str();
        // The span closes at the next run of exactly the same number of backticks
        let mut close: Option<(usize, usize)> = None;
        let mut search = open.end();
        while let Some(m) = INLINE_CODE.find_at(&text, search) {
            if m.as_str().len() == ticks.len() {
                close = Some((m.start(), m.end()));
                break;
            }
            search = m.end();
        }
        match close {
            Some((cs, ce)) => {
                out.push_str(&f(&text[last..open.start()]));
                out.push_str(&text[open.start()..ce]);
                last = ce;
                pos = ce;
                let _ = cs;
            }
            None => {
                pos = open.end();
            }
        }
    }
    out.push_str(&f(&text[last..]));
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOURNAL: &str = "journals/2026-09-27.md";
    const NESTED: &str = "docs/a/b.md";

    #[test]
    fn wikilink_to_link_and_back() {
        assert_eq!(to_markdown_links("see [[My Page]]", JOURNAL), "see [My Page](../docs/My%20Page.md)".replace("%20", " ").replace("(../docs/My Page.md)", "(<../docs/My Page.md>)"));
        assert_eq!(to_markdown_links("[[My Page]]", NESTED), "[My Page](<../My Page.md>)");
        assert_eq!(to_wikilinks("[My Page](<../My Page.md>)", NESTED), "[[My Page]]");
        assert_eq!(to_wikilinks("[label](../docs/x.md#frag)", JOURNAL), "[[x#frag|label]]");
        assert_eq!(to_markdown_links("[[2026-09-27]]", NESTED), "[2026-09-27](../../journals/2026-09-27.md)");
    }

    #[test]
    fn hashtags() {
        assert_eq!(to_markdown_links("note #test here", JOURNAL), "note [#test](../docs/test.md) here");
        assert_eq!(to_markdown_links("#a/b\n#x/", JOURNAL), "[#a/b](../docs/a/b.md)\n[#x](../docs/x.md)/");
        let body = "foo#bar \\#esc https://x/#frag";
        assert_eq!(to_markdown_links(body, JOURNAL), body);
        assert_eq!(to_wikilinks("[#test](../docs/test.md)", JOURNAL), "#test");
    }

    #[test]
    fn code_is_untouched() {
        let body = "```\n[[x]]\n```\n`[[y]]` and [[z]]";
        assert_eq!(to_markdown_links(body, JOURNAL), "```\n[[x]]\n```\n`[[y]]` and [z](../docs/z.md)");
    }

    #[test]
    fn images_and_external_links_stay() {
        assert_eq!(to_wikilinks("![img](../docs/x.md)", JOURNAL), "![img](../docs/x.md)");
        assert_eq!(to_wikilinks("[a](https://x.y/z.md)", JOURNAL), "[a](https://x.y/z.md)");
        assert_eq!(to_wikilinks("[a](../../etc.md)", JOURNAL), "[a](../../etc.md)");
    }

    #[test]
    fn relativize_and_resolve() {
        assert_eq!(relativize("docs/a", "docs/b.md"), "../b.md");
        assert_eq!(relativize("journals", "docs/x.md"), "../docs/x.md");
        assert_eq!(relativize("docs", "docs/x.md"), "x.md");
        assert_eq!(resolve("docs/a", "../b.md"), Some("docs/b.md".to_string()));
        assert_eq!(resolve("docs", "../../x.md"), None);
    }
}
