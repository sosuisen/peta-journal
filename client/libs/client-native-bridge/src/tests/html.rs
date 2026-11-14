//! The HTML of a copy, which a native shell puts on the clipboard (ADR 027).

use crate::copied_html;
use flatmarkdown::{parse_body_with, BodyOptions};

/// The AST of a body, as the journal server makes it: with wikilinks and hashtags.
fn ast_of(body: &str) -> String {
    parse_body_with(body, &BodyOptions::relative_page_links()).to_json()
}

const APP: &str = "http://localhost:8087";

// コピーのHTMLでは、ウィキリンクが、ブラウザ版でそのページを開くふつうのリンクになる。表示する文字は変わらない。
#[test]
fn a_wikilink_of_a_copy_opens_its_page_in_the_browser() {
    let ast = ast_of("see [[Dir/Page|alias]]");

    assert_eq!(
        copied_html(&ast, APP, "tags/"),
        "<p>see <a href=\"http://localhost:8087/pages/Dir/Page\">alias</a></p>\n"
    );
}

// コピーのHTMLでは、ハッシュタグが、タグ用ディレクトリの下のページを開くリンクになる。リストの中にあっても同じである。
#[test]
fn a_hashtag_of_a_copy_opens_the_page_of_the_tag_in_the_browser() {
    let ast = ast_of("- a\n  - about #topic");

    assert_eq!(
        copied_html(&ast, APP, "tags/"),
        "<ul>\n<li>a<ul>\n<li>about <a href=\"http://localhost:8087/pages/tags/topic\">#topic</a></li>\n</ul>\n</li>\n</ul>\n"
    );
}

// 空のコピーは、ASTが空の文字列である。読めないASTと同じく、HTMLは空になる。
#[test]
fn a_copy_without_an_ast_that_can_be_read_has_no_html() {
    assert_eq!(copied_html("", APP, "tags/"), "");
    assert_eq!(copied_html("{\"type\":", APP, "tags/"), "");
}
