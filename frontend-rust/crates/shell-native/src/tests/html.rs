//! The HTML of an item, which a native shell draws (ADR 025).

use crate::item_html;
use flatmarkdown::parse_body;

// 項目のASTから、HTMLができる。Flat Markdownだけにある種類も、HTMLになる。
#[test]
fn the_ast_of_an_item_becomes_html() {
    let ast = parse_body("Some **bold**, [[Idea]] and #todo").to_json();

    let html = item_html(&ast).unwrap();

    assert_eq!(
        html,
        "<p>Some <strong>bold</strong>, <a href=\"Idea\" data-wikilink=\"true\">Idea</a> and \
         <a href=\"todo\" class=\"hashtag\">#todo</a></p>\n"
    );
}

// 読めないASTは、エラーになる。
#[test]
fn an_ast_that_cannot_be_read_is_an_error() {
    assert!(item_html("").is_err());
    assert!(item_html("{\"type\":").is_err());
}
