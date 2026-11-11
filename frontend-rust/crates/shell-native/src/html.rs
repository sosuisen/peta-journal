//! The HTML of an item, which a native shell draws (ADR 025), and the HTML of a copy, which it
//! puts on the clipboard (ADR 027). A shell does not read the AST itself: `ast_to_html` takes
//! every kind of node, and the compiler sees to that.

use flatmarkdown_ast::Node;
use flatmarkdown_ast2html::ast_to_html;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

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
/// which is the address of the journal server. An AST that cannot be read gives no HTML: an
/// empty copy has an empty string for its AST.
///
/// The web frontend does the same in its own crate (`copied_html.rs`, `page_links.rs`), and
/// the two are to give the same links.
pub fn copied_html(ast: &str, app_url: &str, tags_dir: &str) -> String {
    let Ok(mut root) = Node::from_json(ast) else {
        return String::new();
    };
    link_to_the_app(&mut root, app_url, tags_dir);
    ast_to_html(&root)
}

/// Makes every wikilink and hashtag under `node` an ordinary link to its page.
fn link_to_the_app(node: &mut Node, app_url: &str, tags_dir: &str) {
    let link = match node {
        Node::Wikilink { url, children } => Some(Node::Link {
            url: url_of(app_url, &format!("/pages/{url}")),
            title: String::new(),
            children: std::mem::take(children),
        }),
        Node::Hashtag { value } => Some(Node::Link {
            url: url_of(app_url, &format!("/pages/{tags_dir}{value}")),
            title: String::new(),
            children: vec![Node::Text {
                value: format!("#{value}"),
            }],
        }),
        _ => None,
    };
    if let Some(link) = link {
        *node = link;
    }
    for child in node.children_mut().into_iter().flatten() {
        link_to_the_app(child, app_url, tags_dir);
    }
}

/// What is encoded in a path that leaves the app: the same characters as `encodeURI` of
/// JavaScript, with which the web frontend encodes its own URLs. `#` stays, so a link to a
/// place in a page still leads there.
const OUTSIDE_A_URL: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

fn url_of(app_url: &str, path: &str) -> String {
    format!("{app_url}{}", utf8_percent_encode(path, OUTSIDE_A_URL))
}
