use crate::emacs_text::{EmacsLook, EmacsText};
use client_core::rich_text::RichLink;
use client_core::BodyModel;

fn text_of(markdown: &str) -> EmacsText {
    EmacsText::of_markdown(markdown, "tags/")
}

// 段落のtextは、表示の文字と、その後ろの改行である。飾りのない文字には範囲がない。
#[test]
fn a_paragraph_is_its_shown_letters_and_a_line_break_with_no_span_for_plain_letters() {
    let text = text_of("first\n\nsecond");

    assert_eq!(text.text, "first\nsecond\n");
    assert!(text.spans.is_empty(), "{:?}", text.spans);
}

/// The places and the looks of each span of `text`.
fn looks_of(text: &EmacsText) -> Vec<(u32, u32, Vec<EmacsLook>)> {
    text.spans
        .iter()
        .map(|span| (span.from, span.to, span.looks.clone()))
        .collect()
}

// 文字の見た目は、範囲の見た目になる。位置は文字（コードポイント）で数える。
#[test]
fn the_looks_of_letters_are_the_looks_of_a_span_placed_in_code_points() {
    let text = text_of("日本 **太字** `code`");

    assert_eq!(text.text, "日本 太字 code\n");
    assert_eq!(
        looks_of(&text),
        [
            (3, 5, vec![EmacsLook::Bold]),
            (6, 10, vec![EmacsLook::Code])
        ]
    );
}

// リンクのある文字は、範囲にリンクを持つ。
#[test]
fn letters_with_a_link_have_the_link_on_their_span() {
    let text = text_of("see [web](https://example.com/) now");

    assert_eq!(text.text, "see web now\n");
    let links: Vec<(u32, u32, Option<RichLink>)> = text
        .spans
        .iter()
        .map(|span| (span.from, span.to, span.link.clone()))
        .collect();
    assert_eq!(
        links,
        [(
            4,
            7,
            Some(RichLink::Web {
                url: "https://example.com/".into()
            })
        )]
    );
}

// 見出しの段落は、行全体に見出しの深さの見た目を持つ。
#[test]
fn a_heading_has_the_look_of_its_level_over_its_whole_line() {
    let text = text_of("## 見出し **強く**");

    assert_eq!(text.text, "見出し 強く\n");
    assert_eq!(
        looks_of(&text),
        [
            (0, 6, vec![EmacsLook::Heading2]),
            (4, 6, vec![EmacsLook::Bold])
        ]
    );
}
