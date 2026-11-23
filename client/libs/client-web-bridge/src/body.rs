//! The body of a row, as the web shell draws it.

use client_core::BodyModel;
use flatmarkdown_ast::{Node, PropValue};
use serde::Serialize;
use std::collections::BTreeMap;

/// The body of a row as HTML, which the shell puts into its element as it is (ADR 079). Wasm
/// makes it, so that the shell needs no code that turns the AST into HTML.
#[derive(Debug, Clone, PartialEq, Serialize, facet::Facet)]
#[facet(transparent)]
pub struct HtmlBody(pub String);

impl BodyModel for HtmlBody {
    fn of(mut ast: Node, tags_dir: &str) -> HtmlBody {
        ready_image_urls(&mut ast);
        let options = page_urls::html_options(tags_dir);
        HtmlBody(flatmarkdown_ast2html::ast_to_html_with(&ast, &options))
    }
}

/// Readies the URL of every image under `node` for the `<img>` of the shell.
fn ready_image_urls(node: &mut Node) {
    if let Node::Embed { url, props, .. } = node {
        start_asset_at_top(url);
        keep_size(url, props);
    }
    for child in node.children_mut().into_iter().flatten() {
        ready_image_urls(child);
    }
}

/// Makes the URL of a file of the assets folder start at the top of the app: `/assets/a.png`
/// for `../assets/a.png`. A page writes it relative to its own file. The browser would read it
/// from the URL of the page, which is deeper for a page in a folder. No `<base>` is used: it
/// would also move the links to a footnote (`#fn-1`) off the page.
fn start_asset_at_top(url: &mut String) {
    if client_core::rich_text::asset_path(url).is_some() {
        *url = format!("/{}", url.trim_start_matches(['.', '/']));
    }
}

/// Puts the size of the image into the fragment of its URL, as `w` and `h`.
/// `ast_to_html_with` leaves the size out, because each app uses it in its own way (ADR 026).
/// The shell reads it back from the `<img>`.
fn keep_size(url: &mut String, props: &BTreeMap<String, PropValue>) {
    let sides: Vec<String> = ["w", "h"]
        .into_iter()
        .filter_map(|side| match props.get(side) {
            Some(PropValue::Text(text)) => Some(format!("{side}={}", text.trim())),
            _ => None,
        })
        .collect();
    if !sides.is_empty() {
        url.push('#');
        url.push_str(&sides.join("&"));
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

        assert!(
            sized.contains(r#"src="img.png#w=300&amp;h=240""#),
            "{sized}"
        );
        assert!(plain.contains(r#"src="img.png""#), "{plain}");
    }

    // assetsの画像のURLは、アプリの一番上からのパスになる。ページのURLがフォルダーの下にあっても、画像に届く。
    #[test]
    fn the_url_of_an_asset_image_starts_at_the_top_of_the_app() {
        let html = HtmlBody::of(client_core::asts::parse("![alt](../assets/a.png)"), "tags/").0;

        assert!(html.contains(r#"src="/assets/a.png""#), "{html}");
    }
}
