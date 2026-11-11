//! The HTML of an item, which a native shell draws (ADR 025), and the HTML of a copy, which it
//! puts on the clipboard (ADR 027). A shell does not read the AST itself: `ast_to_html` takes
//! every kind of node, and the compiler sees to that.

use flatmarkdown_ast::Node;
use flatmarkdown_ast2html::ast_to_html;
use peta_page_links::AppLinks;

/// The HTML of an item, from its AST as the JSON that the view model carries. The error says
/// why the JSON could not be read. An item without a body has an empty string for its AST,
/// and nothing to show.
pub fn item_html(ast: &str) -> Result<String, String> {
    if ast.is_empty() {
        return Ok(String::new());
    }
    let node = Node::from_json(ast).map_err(|unreadable| unreadable.to_string())?;
    Ok(ast_to_html(&node))
}

/// The HTML of a copy, from the AST of its tree Markdown as the core hands it over. Another
/// app reads the HTML when the copy is pasted there, so a wikilink and a hashtag become whole
/// URLs that open the page in the browser: `app_url` is where the web frontend is served,
/// which is the address of the journal server. An AST that cannot be read gives no HTML.
pub fn copied_html(ast: &str, app_url: &str, tags_dir: &str) -> String {
    peta_page_links::copied_html(ast, AppLinks { app_url, tags_dir })
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
