use crate::emacs_text::{EmacsLook, EmacsText};
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
