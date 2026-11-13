//! The HTML of a copy, which a native shell puts on the clipboard (ADR 027), and the URLs of
//! the pages that links name. A shell draws an item from the AST of its row (ADR 041).

use client_page_links::AppLinks;

/// The HTML of a copy, from the AST of its tree Markdown as the core hands it over. Another
/// app reads the HTML when the copy is pasted there, so a wikilink and a hashtag become whole
/// URLs that open the page in the browser: `app_url` is where the web frontend is served,
/// which is the address of the journal server. An AST that cannot be read gives no HTML.
pub fn copied_html(ast: &str, app_url: &str, tags_dir: &str) -> String {
    client_page_links::copied_html(ast, AppLinks { app_url, tags_dir })
}

/// The URL that opens the page of a wikilink in the web frontend, served at `app_url`.
pub fn wikilink_url(app_url: &str, page: &str) -> String {
    let tags_dir = "";
    AppLinks { app_url, tags_dir }.wikilink(page)
}

/// The URL that opens the page of a tag in the web frontend. A tag page lies under the
/// configured tag directory.
pub fn hashtag_url(app_url: &str, tags_dir: &str, tag: &str) -> String {
    AppLinks { app_url, tags_dir }.hashtag(tag)
}
