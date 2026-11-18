//! The URLs of the pages of PetaJournal (ADR 061).
//!
//! A page opens at `/pages/<name>`, and an item of it at `/pages/<name>#<item id>`. The rule
//! of where pages live and how their URLs are read is here alone. How a name goes into a URL,
//! and how it is encoded, is the rule of flatmarkdown, so this crate passes the templates to
//! flatmarkdown and back without looking into them (ADR 061).

use flatmarkdown_ast2html::{fill_href, href_template_from_prefix, HtmlOptions};

/// The base that every page lives under. `pagesBase` of the config of the server, which is
/// also the base that the relative paths of images are read from.
pub const PAGES_BASE: &str = "/pages/";

/// The form of the id of an item, as a regular expression: an HMTID, with or without the time
/// split up (`20260426035700_QZ998PX`, `2026-04-26-03-57-00-QZ998PX`). It looks at the digits
/// and the letters only, so it matches more than `Hmtid::parse` accepts, such as month 13.
///
/// It is written for both the `regex-lite` crate and `RegExp` of JavaScript, which read it in
/// the same way (ADR 051). `wikilinkFragmentPattern` of the config of the server.
pub const ITEM_ID_PATTERN: &str = "[0-9]{4}[-_]?[0-9]{2}[-_]?[0-9]{2}[-_]?[0-9]{2}[-_]?[0-9]{2}[-_]?[0-9]{2}[-_][0-9A-HJKMNP-TV-Z]{7}";

/// The template of the href of a wikilink, for the renderers of flatmarkdown.
pub fn wikilink_href() -> String {
    href_template_from_prefix(PAGES_BASE)
}

/// The template of the href of a hashtag, for the renderers of flatmarkdown. The page of a tag
/// lives under the folder of the tags, `tags_dir`.
pub fn hashtag_href(tags_dir: &str) -> String {
    href_template_from_prefix(&format!("{PAGES_BASE}{tags_dir}"))
}

/// The options with which a renderer of flatmarkdown links wikilinks and hashtags to their
/// pages, and an item id after a `#` to its item.
pub fn html_options(tags_dir: &str) -> HtmlOptions {
    HtmlOptions {
        wikilink_href: wikilink_href(),
        hashtag_href: hashtag_href(tags_dir),
        wikilink_fragment_pattern: ITEM_ID_PATTERN.to_string(),
    }
}

/// The name of the page of the tag `tag`, without its `#`.
pub fn tag_page_name(tags_dir: &str, tag: &str) -> String {
    format!("{tags_dir}{tag}")
}

/// The URL of the page `page_name`, encoded.
pub fn page_url(page_name: &str) -> String {
    fill_href(&wikilink_href(), page_name)
}

/// The URL of the item `item_id` of the page `page_name`, encoded.
pub fn item_url(page_name: &str, item_id: &str) -> String {
    format!("{}#{item_id}", page_url(page_name))
}

/// The name of the page and the id of the item that the text of a wikilink names. The `#`
/// before an item id splits them. Any other `#` is a part of the name, as in `[[C#]]`.
pub fn split_wikilink(content: &str) -> (&str, Option<&str>) {
    flatmarkdown_ast2html::split_wikilink(content, ITEM_ID_PATTERN)
}

/// What a URL of a page names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageAddress {
    /// The name of the page. It is empty for `/pages/`.
    pub name: String,
    /// The id of the item to go to, from after the `#`.
    pub item: Option<String>,
}

/// What the URL of a page names, such as `/pages/C%23#20260426035700_QZ998PX`. `None` for a
/// URL that is not under [`PAGES_BASE`].
///
/// The URL is split at its first `#`, as a name has its own `#` encoded. The path and the item
/// id are each decoded once, so a URL that a browser has encoded again gives the same name.
/// A URL that is not encoded is read as it is, unless a `%` in a name is followed by two hex
/// digits.
pub fn read_url(url: &str) -> Option<PageAddress> {
    let (path, item) = match url.split_once('#') {
        Some((path, item)) => (path, Some(item)),
        None => (url, None),
    };
    let name = path.strip_prefix(PAGES_BASE)?.trim_end_matches('/');
    Some(PageAddress {
        name: decoded(name),
        item: item.filter(|item| !item.is_empty()).map(decoded),
    })
}

/// `text` with every `%XX` turned back into its byte.
fn decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes.get(i + 1..i + 3).and_then(|hex| {
            let hex = std::str::from_utf8(hex).ok()?;
            u8::from_str_radix(hex, 16).ok()
        });
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests;
