//! The body of a row, as the web shell draws it.

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
