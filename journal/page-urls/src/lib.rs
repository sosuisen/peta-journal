//! The URLs of the pages of PetaJournal (ADR 061).
//!
//! A page opens at `/pages/<name>`, and an item of it at `/pages/<name>#<item id>`. The rule
//! of where pages live and how their URLs are read is here alone. How a name goes into a URL,
//! and how it is encoded, is the rule of flatmarkdown, so this crate passes the templates to
//! flatmarkdown and back without looking into them (ADR 061).

use flatmarkdown_ast2html::{fill_href, href_template_from_prefix};

/// The base that every page lives under. `pagesBase` of the config of the server.
pub const PAGES_BASE: &str = "/pages/";

/// The template of the href of a wikilink, for the renderers of flatmarkdown.
pub fn wikilink_href() -> String {
    href_template_from_prefix(PAGES_BASE)
}

/// The URL of the page `page_name`, encoded.
pub fn page_url(page_name: &str) -> String {
    fill_href(&wikilink_href(), page_name)
}

#[cfg(test)]
mod tests;
