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

    // 本文が求める画像の大きさは、画像のURLの断片に書かれる。深いところの画像も同じである。
    #[test]
    fn the_size_an_image_asks_for_is_written_in_the_fragment_of_its_url() {
        let html = |body: &str| HtmlBody::of(client_core::asts::parse(body), "tags/").0;

        let sized = html("- ![alt](img.png#w=300&h=240)");
        let plain = html("![alt](img.png)");

        assert!(sized.contains(r#"src="img.png#w=300&amp;h=240""#), "{sized}");
        assert!(plain.contains(r#"src="img.png""#), "{plain}");
    }
}
