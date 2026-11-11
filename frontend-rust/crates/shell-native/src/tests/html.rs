//! The HTML of an item, which a native shell draws (ADR 025).

use crate::item_html;
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
