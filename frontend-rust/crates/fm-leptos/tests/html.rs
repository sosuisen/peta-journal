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
    let root = flatmarkdown::parse_body_with(md, &options());
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

// インライン書式を含む段落は em・strong・code・del 要素を持つ p にレンダリングされる。
#[test]
fn paragraph_with_inline_formatting() {
    assert_eq!(
        render_plain("hello *em* **strong** `code` ~~del~~"),
        "<p>hello <em>em</em> <strong>strong</strong> <code>code</code> <del>del</del></p>"
    );
}

// 見出しはレベルに応じた h1〜h6 要素にレンダリングされる。
#[test]
fn headings() {
    assert_eq!(render_plain("# One"), "<h1>One</h1>");
    assert_eq!(render_plain("### Three"), "<h3>Three</h3>");
    assert_eq!(render_plain("###### Six"), "<h6>Six</h6>");
}

// コードブロックは情報文字列に応じた language- クラスつきの pre/code にレンダリングされ、情報文字列が空ならクラスがなく、中身はエスケープされる。
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
    let root =
        flatmarkdown_ast::Node::from_json(r#"{"type":"code_block","info":"","literal":"plain"}"#)
            .unwrap();
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

// 項目の間に空行のないリスト（タイト）は、項目を p で包まない ul/ol にレンダリングされ、開始番号は start 属性になる。
#[test]
fn tight_lists() {
    assert_eq!(render_plain("- a\n- b"), "<ul><li>a</li><li>b</li></ul>");
    assert_eq!(render_plain("1. a\n2. b"), "<ol><li>a</li><li>b</li></ol>");
    assert_eq!(
        render_plain("3. a\n4. b"),
        "<ol start=\"3\"><li>a</li><li>b</li></ol>"
    );
}

// 項目の間に空行のあるリスト（ルーズ）は、項目を p で包む。
#[test]
fn loose_lists() {
    assert_eq!(
        render_plain("- a\n\n- b"),
        "<ul><li><p>a</p></li><li><p>b</p></li></ul>"
    );
}

// タイトなリストで p が外れるのは、項目の直下の段落だけである。項目の中の引用の段落は p で包まれる。
#[test]
fn a_paragraph_in_a_block_quote_of_a_tight_item_keeps_its_p() {
    assert_eq!(
        render_plain("- a\n  > q"),
        "<ul><li>a<blockquote><p>q</p></blockquote></li></ul>"
    );
}

// ルーズなリストの中のタイトなリストは、p で包まれない。タイトかどうかはリストごとに決まる。
#[test]
fn a_tight_list_inside_a_loose_list_is_tight() {
    assert_eq!(
        render_plain("- a\n\n  - x\n  - y\n\n- b"),
        "<ul><li><p>a</p><ul><li>x</li><li>y</li></ul></li><li><p>b</p></li></ul>"
    );
}

// タスクリストは disabled なチェックボックスを持つ li にレンダリングされ、完了項目には checked が付く。
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

// 配置指定つきのテーブルは th/td に align 属性を持つ table にレンダリングされ、指定のない列には属性が付かない。
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

// 引用と水平線は blockquote と hr にレンダリングされる。
#[test]
fn blockquote_and_rule() {
    assert_eq!(
        render_plain("> quoted\n\n---"),
        "<blockquote><p>quoted</p></blockquote><hr>"
    );
}

// アラートは種別のクラスを持つ div にレンダリングされ、タイトルは指定がなければ種別名、指定があればその文字列になる。
#[test]
fn alert_with_default_and_custom_title() {
    assert_eq!(
        render_plain(
            "> [!NOTE]
> body"
        ),
        "<div class=\"alert alert-note\"><p class=\"alert-title\">Note</p><p>body</p></div>"
    );
    assert_eq!(
        render_plain("> [!WARNING] Mind the gap
> body"),
        "<div class=\"alert alert-warning\"><p class=\"alert-title\">Mind the gap</p><p>body</p></div>"
    );
    assert_eq!(
        render_plain(
            ">>> [!TIP]
body
>>>"
        ),
        "<div class=\"alert alert-tip\"><p class=\"alert-title\">Tip</p><p>body</p></div>"
    );
}

// リンクは href と title を持つ a に、画像は src と alt を持つ img にレンダリングされる。
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

// 画像 URL のフラグメントの w と h は img の width と height 属性になり、値のないフラグや数値でない値、0は無視される。
#[test]
fn embed_width_and_height() {
    // Both sizes.
    assert_eq!(
        render_plain("![alt](img.png#w=300&h=240)"),
        "<p><img src=\"img.png\" alt=\"alt\" width=\"300\" height=\"240\"></p>"
    );
    // Width only: the height follows the aspect ratio.
    assert_eq!(
        render_plain("![alt](img.png#w=300)"),
        "<p><img src=\"img.png\" alt=\"alt\" width=\"300\"></p>"
    );
    // Height only.
    assert_eq!(
        render_plain("![alt](img.png#h=240)"),
        "<p><img src=\"img.png\" alt=\"alt\" height=\"240\"></p>"
    );
    // A bare flag, a non-numeric value, or zero is not a size.
    assert_eq!(
        render_plain("![alt](img.png#w&h=big)"),
        "<p><img src=\"img.png\" alt=\"alt\"></p>"
    );
    assert_eq!(
        render_plain("![alt](img.png#w=0&h=0)"),
        "<p><img src=\"img.png\" alt=\"alt\"></p>"
    );
}

// ウィキリンクとハッシュタグはオプションのプレフィックスとクラスを持つ a にレンダリングされ、オプションがなければプレフィックスもクラスもない。
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

// 脚注参照は脚注へのリンクを持つ sup に、脚注定義は参照へ戻るラベルつきの div にレンダリングされる。
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

// 段落内の改行はそのまま改行文字として、インライン HTML とブロック HTML は span で包んでレンダリングされる。
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

// JSON から作ったその他のノード（shortcode・math・alert・spoilered_text・footnote_definition）は Angular レンダラーと同じ HTML になる。仕様にないノード（insert など）は描かない。
#[test]
fn other_nodes_from_json() {
    let render_json = |json: &str| {
        clean(
            render_node(
                &flatmarkdown_ast::Node::from_json(json).unwrap(),
                &RenderOptions::default(),
            )
            .to_html(),
        )
    };
    assert_eq!(
        render_json(r#"{"type":"shortcode","code":"smile","emoji":"😄"}"#),
        "😄"
    );
    assert_eq!(
        render_json(r#"{"type":"math","literal":"x^2"}"#),
        "<code class=\"math math-inline\">x^2</code>"
    );
    assert_eq!(
        render_json(
            r#"{"type":"alert","alert_type":"note","title":null,"children":[{"type":"paragraph","children":[{"type":"text","value":"n"}]}]}"#
        ),
        "<div class=\"alert alert-note\"><p class=\"alert-title\">Note</p><p>n</p></div>"
    );
    assert_eq!(
        render_json(r#"{"type":"spoilered_text","children":[{"type":"text","value":"s"}]}"#),
        "<span class=\"spoiler\">s</span>"
    );
    for unknown in [
        r#"{"type":"insert","children":[{"type":"text","value":"x"}]}"#,
        r#"{"type":"raw","value":"<i>r</i>"}"#,
        r#"{"type":"front_matter","value":"x"}"#,
    ] {
        assert_eq!(render_json(unknown), "", "{unknown}");
    }
    assert_eq!(
        render_json(r#"{"type":"footnote_definition","name":"a","children":[{"type":"code_block","info":"","literal":"c"}]}"#),
        "<div id=\"fn-a\" class=\"footnote\"><a href=\"#fnref-a\" class=\"footnote-label\">[a]:</a><pre><code>c</code></pre></div>"
    );
}

// FmRoot コンポーネントは AST の JSON 文字列を HTML にレンダリングし、JSON でない文字列には空を返す。
#[test]
fn fm_root_component() {
    let json = flatmarkdown::parse_body_with("# Hi", &options()).to_json();
    let html = clean(view! { <FmRoot ast=json /> }.to_html());
    assert_eq!(html, "<h1>Hi</h1>");
    let html =
        clean(view! { <FmRoot ast="not json".to_string() options=page_options() /> }.to_html());
    assert_eq!(html, "");
}
