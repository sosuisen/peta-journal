//! Renders bodies through `flatmarkdown` and `fm_leptos` and compares the HTML with what
//! the Angular renderer (`flatmarkdown-ast2angular`) produces for the same nodes.
//!
//! `to_html()` needs Leptos' `ssr` feature, which the dev-dependencies enable.

use flatmarkdown::{BodyOptions, LinkResolution};
use fm_leptos::{render_node, FmRoot, RenderOptions};
use leptos::prelude::*;

fn options() -> BodyOptions {
    BodyOptions {
        link_resolution: LinkResolution::On {
            prefixes: vec!["./".to_string(), "../".to_string()],
        },
    }
}

/// Leptos' server renderer emits `<!>` hydration markers between dynamic children and a
/// `<!--<() />-->` placeholder for an empty view; neither is an element, so strip them
/// before comparing. Leptos also writes `class` after the other attributes, so the expected
/// strings below follow that order (attribute order has no meaning in HTML).
fn clean(html: String) -> String {
    html.replace("<!>", "")
        .replace("<!---->", "")
        .replace("<!--<() />-->", "")
}

fn render_md(md: &str, opts: &RenderOptions) -> String {
    let json = flatmarkdown::body_to_ast_with(md, &options());
    let root = fm_ast::parse(&json).unwrap();
    clean(render_node(&root, opts).to_html())
}

fn render_plain(md: &str) -> String {
    render_md(md, &RenderOptions::default())
}

fn page_options() -> RenderOptions {
    RenderOptions {
        wikilink_prefix: "/pages/".into(),
        wikilink_class: "wiki-link".into(),
        hashtag_prefix: "/pages/".into(),
        hashtag_class: "hashtag-link".into(),
    }
}

#[test]
fn paragraph_with_inline_formatting() {
    assert_eq!(
        render_plain("hello *em* **strong** `code` ~~del~~"),
        "<p>hello <em>em</em> <strong>strong</strong> <code>code</code> <del>del</del></p>"
    );
}

#[test]
fn headings() {
    assert_eq!(render_plain("# One"), "<h1>One</h1>");
    assert_eq!(render_plain("### Three"), "<h3>Three</h3>");
    assert_eq!(render_plain("###### Six"), "<h6>Six</h6>");
}

#[test]
fn code_block_with_and_without_info() {
    assert_eq!(
        render_plain("```rust\nfn main() {}\n```"),
        "<pre><code class=\"language-rust\">fn main() {}\n</code></pre>"
    );
    // A bare fence gets the info string "text" from flatmarkdown.
    assert_eq!(
        render_plain("```\nplain\n```"),
        "<pre><code class=\"language-text\">plain\n</code></pre>"
    );
    // With an empty info string there is no class attribute at all.
    let root = fm_ast::parse(r#"{"type":"code_block","info":"","literal":"plain"}"#).unwrap();
    assert_eq!(
        clean(render_node(&root, &RenderOptions::default()).to_html()),
        "<pre><code>plain</code></pre>"
    );
    // Text is escaped, never injected as markup.
    assert_eq!(
        render_plain("```\n<b>\n```"),
        "<pre><code class=\"language-text\">&lt;b&gt;\n</code></pre>"
    );
}

#[test]
fn lists() {
    // comrak marks items `tight: false` even in a tight list, and the Angular renderer reads
    // the item's flag, so list items keep their `<p>`.
    assert_eq!(
        render_plain("- a\n- b"),
        "<ul><li><p>a</p></li><li><p>b</p></li></ul>"
    );
    assert_eq!(
        render_plain("1. a\n2. b"),
        "<ol><li><p>a</p></li><li><p>b</p></li></ol>"
    );
    assert_eq!(
        render_plain("3. a\n4. b"),
        "<ol start=\"3\"><li><p>a</p></li><li><p>b</p></li></ol>"
    );
}

#[test]
fn task_list() {
    assert_eq!(
        render_plain("- [ ] todo\n- [x] done"),
        "<ul>\
         <li><input type=\"checkbox\" disabled>todo</li>\
         <li><input type=\"checkbox\" checked disabled>done</li>\
         </ul>"
    );
}

#[test]
fn table_with_alignment() {
    assert_eq!(
        render_plain("| a | b | c |\n|:--|:-:|---|\n| 1 | 2 | 3 |"),
        "<table>\
         <tr><th align=\"left\">a</th><th align=\"center\">b</th><th>c</th></tr>\
         <tr><td align=\"left\">1</td><td align=\"center\">2</td><td>3</td></tr>\
         </table>"
    );
}

#[test]
fn blockquote_and_rule() {
    assert_eq!(
        render_plain("> quoted\n\n---"),
        "<blockquote><p>quoted</p></blockquote><hr>"
    );
}

#[test]
fn link_and_image() {
    assert_eq!(
        render_plain("[site](https://example.com \"Title\")"),
        "<p><a href=\"https://example.com\" title=\"Title\">site</a></p>"
    );
    assert_eq!(
        render_plain("[site](https://example.com)"),
        "<p><a href=\"https://example.com\">site</a></p>"
    );
    assert_eq!(
        render_plain("![alt text](pic.png)"),
        "<p><img src=\"pic.png\" alt=\"alt text\"></p>"
    );
}

#[test]
fn wikilink_and_hashtag() {
    assert_eq!(
        render_md("see [[Other Page]] and #topic", &page_options()),
        "<p>see \
         <a href=\"/pages/Other Page\" data-wikilink=\"true\" class=\"wiki-link\">Other Page</a> \
         and \
         <a href=\"/pages/topic\" data-hashtag=\"true\" class=\"hashtag-link\">#topic</a>\
         </p>"
    );
    // Without options: no prefix, no class attribute.
    assert_eq!(
        render_plain("[[Page]]"),
        "<p><a href=\"Page\" data-wikilink=\"true\">Page</a></p>"
    );
    assert_eq!(
        render_plain("#tag"),
        "<p><a href=\"tag\" data-hashtag=\"true\">#tag</a></p>"
    );
}

#[test]
fn footnotes() {
    assert_eq!(
        render_plain("text[^1]\n\n[^1]: the note"),
        "<p>text<sup id=\"fnref-1\" class=\"footnote-ref\"><a href=\"#fn-1\">[1]</a></sup></p>\
         <div id=\"fn-1\" class=\"footnote\">\
         <p><a href=\"#fnref-1\" class=\"footnote-label\">[1]:</a> the note</p>\
         </div>"
    );
}

#[test]
fn breaks_and_html() {
    assert_eq!(render_plain("first\nsecond"), "<p>first\nsecond</p>");
    assert_eq!(
        render_plain("a <b>bold</b> c"),
        "<p>a <span><b></span>bold<span></b></span> c</p>"
    );
    assert_eq!(
        render_plain("<div>\nblock\n</div>"),
        "<span><div>\nblock\n</div></span>"
    );
}

#[test]
fn other_nodes_from_json() {
    let render_json = |json: &str| {
        clean(render_node(&fm_ast::parse(json).unwrap(), &RenderOptions::default()).to_html())
    };
    assert_eq!(render_json(r#"{"type":"front_matter","value":"x"}"#), "");
    assert_eq!(render_json(r#"{"type":"escaped"}"#), "");
    assert_eq!(
        render_json(r#"{"type":"escaped_tag","value":"<t>"}"#),
        "&lt;t&gt;"
    );
    assert_eq!(
        render_json(r#"{"type":"shortcode","code":"smile","emoji":"😄"}"#),
        "😄"
    );
    assert_eq!(
        render_json(r#"{"type":"math","display_math":true,"literal":"x^2"}"#),
        "<pre><code class=\"math math-display\">x^2</code></pre>"
    );
    assert_eq!(
        render_json(r#"{"type":"math","literal":"x^2"}"#),
        "<code class=\"math math-inline\">x^2</code>"
    );
    assert_eq!(
        render_json(
            r#"{"type":"alert","alert_type":"note","title":null,"children":[{"type":"paragraph","children":[{"type":"text","value":"n"}]}]}"#
        ),
        "<div class=\"alert alert-note\"><p>n</p></div>"
    );
    assert_eq!(
        render_json(r#"{"type":"spoilered_text","children":[{"type":"text","value":"s"}]}"#),
        "<span class=\"spoiler\">s</span>"
    );
    assert_eq!(
        render_json(r#"{"type":"raw","value":"<i>r</i>"}"#),
        "<span><i>r</i></span>"
    );
    assert_eq!(
        render_json(r#"{"type":"insert","children":[{"type":"text","value":"x"}]}"#),
        ""
    );
    assert_eq!(
        render_json(r#"{"type":"footnote_definition","name":"a","children":[{"type":"code_block","info":"","literal":"c"}]}"#),
        "<div id=\"fn-a\" class=\"footnote\"><a href=\"#fnref-a\" class=\"footnote-label\">[a]:</a><pre><code>c</code></pre></div>"
    );
}

#[test]
fn fm_root_component() {
    let json = flatmarkdown::body_to_ast_with("# Hi", &options());
    let html = clean(view! { <FmRoot ast=json /> }.to_html());
    assert_eq!(html, "<h1>Hi</h1>");
    let html =
        clean(view! { <FmRoot ast="not json".to_string() options=page_options() /> }.to_html());
    assert_eq!(html, "");
}
