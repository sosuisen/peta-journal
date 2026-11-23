//! The body of a row, as the web shell draws it.

use client_core::BodyModel;
use flatmarkdown_ast::Node;
use serde::Serialize;

/// The body of a row as HTML, which the shell puts into its element as it is (ADR 079). Wasm
/// makes it, so that the shell needs no code that turns the AST into HTML.
#[derive(Debug, Clone, PartialEq, Serialize, facet::Facet)]
#[facet(transparent)]
pub struct HtmlBody(pub String);

impl BodyModel for HtmlBody {
    fn of(ast: Node, tags_dir: &str) -> HtmlBody {
        let options = page_urls::html_options(tags_dir);
        HtmlBody(flatmarkdown_ast2html::ast_to_html_with(&ast, &options))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use client_core::testing::plain_item;
    use client_core::{Model, PetaApp};
    use crux_core::App;

    // 行の本文は、本文のHTMLである。
    #[test]
    fn the_body_of_a_row_is_the_html_of_its_body() {
        let mut model = Model::default();
        model
            .state
            .set_items(vec![plain_item("i1", 1, "**bold** and [[notes]]")]);

        let view = PetaApp::<HtmlBody>::default().view(&model);

        let HtmlBody(html) = &view.outline.rows[0].body_model;
        assert!(html.contains("<strong>bold</strong>"), "{html}");
        assert!(html.contains(r#"href="/pages/notes""#), "{html}");
    }
}
