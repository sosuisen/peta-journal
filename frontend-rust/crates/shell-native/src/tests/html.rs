//! The HTML of an item, which a native shell draws (ADR 025), and the HTML of a copy, which it
//! puts on the clipboard (ADR 027).

use crate::{copied_html, hashtag_url, item_html, wikilink_url};
use flatmarkdown::{parse_body_with, BodyOptions, LinkResolution};

/// The AST of a body, as the journal server makes it: with wikilinks and hashtags.
fn ast_of(body: &str) -> String {
    let as_the_server = BodyOptions {
        link_resolution: LinkResolution::On {
            prefixes: Vec::new(),
        },
    };
    parse_body_with(body, &as_the_server).to_json()
}

// 項目のASTから、HTMLができる。Flat Markdownだけにある種類も、HTMLになる。
#[test]
fn the_ast_of_an_item_becomes_html() {
    let ast = ast_of("Some **bold**, [[Idea]] and #todo");

    let html = item_html(&ast).unwrap();

    assert_eq!(
        html,
        "<p>Some <strong>bold</strong>, <a href=\"Idea\" data-wikilink=\"true\">Idea</a> and \
         <a href=\"todo\" class=\"hashtag\">#todo</a></p>\n"
    );
}

// 本文が空の項目は、ASTが空の文字列である。そのHTMLも、空である。
#[test]
fn an_item_without_a_body_has_no_html() {
    assert_eq!(item_html(""), Ok(String::new()));
}

// 読めないASTは、エラーになる。
#[test]
fn an_ast_that_cannot_be_read_is_an_error() {
    assert!(item_html("{\"type\":").is_err());
}

const APP: &str = "http://localhost:8087";

// コピーのASTから、HTMLができる。リストの入れ子も残る。
#[test]
fn the_ast_of_a_copy_becomes_html() {
    let ast = ast_of("- a **bold**\n  - b");

    assert_eq!(
        copied_html(&ast, APP, "tags/"),
        "<ul>\n<li>a <strong>bold</strong><ul>\n<li>b</li>\n</ul>\n</li>\n</ul>\n"
    );
}

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

// 空白や日本語を含むページ名は、1つのURLとして読めるように符号化される。ページの中の場所を指す # は残る。
#[test]
fn a_page_name_of_a_copy_is_encoded_into_one_url() {
    let ast = ast_of("[[Other Page]] [[メモ#place]]");

    assert_eq!(
        copied_html(&ast, APP, ""),
        "<p><a href=\"http://localhost:8087/pages/Other%20Page\">Other Page</a> \
         <a href=\"http://localhost:8087/pages/%E3%83%A1%E3%83%A2#place\">メモ#place</a></p>\n"
    );
}

// 外のサイトへのリンクは、そのままである。
#[test]
fn a_link_of_a_copy_to_another_site_stays_as_it_is() {
    let ast = ast_of("[site](https://example.com)");

    assert_eq!(
        copied_html(&ast, APP, "tags/"),
        "<p><a href=\"https://example.com\">site</a></p>\n"
    );
}

// 空のコピーは、ASTが空の文字列である。読めないASTと同じく、HTMLは空になる。
#[test]
fn a_copy_without_an_ast_that_can_be_read_has_no_html() {
    assert_eq!(copied_html("", APP, "tags/"), "");
    assert_eq!(copied_html("{\"type\":", APP, "tags/"), "");
}

// ウィキリンクのページをブラウザ版で開くURLは、アプリのURLの /pages/ の下にある。空白や日本語は符号化され、# は残る。
#[test]
fn the_url_of_the_page_of_a_wikilink_is_under_pages_of_the_app() {
    assert_eq!(wikilink_url(APP, "Dir/Page"), "http://localhost:8087/pages/Dir/Page");
    assert_eq!(
        wikilink_url(APP, "メモ 帳#place"),
        "http://localhost:8087/pages/%E3%83%A1%E3%83%A2%20%E5%B8%B3#place"
    );
}

// タグのページをブラウザ版で開くURLは、タグ用ディレクトリの下にある。ディレクトリが空なら、/pages/ の直下にある。
#[test]
fn the_url_of_the_page_of_a_tag_is_under_the_tag_directory() {
    assert_eq!(hashtag_url(APP, "tags/", "topic"), "http://localhost:8087/pages/tags/topic");
    assert_eq!(hashtag_url(APP, "", "topic"), "http://localhost:8087/pages/topic");
}
