use crate::emacs_text::EmacsText;
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
