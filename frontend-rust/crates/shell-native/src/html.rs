//! The HTML of an item, which a native shell draws (ADR 025). A shell does not read the AST
//! itself: `ast_to_html` takes every kind of node, and the compiler sees to that.

use flatmarkdown_ast::Node;
use flatmarkdown_ast2html::ast_to_html;

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
