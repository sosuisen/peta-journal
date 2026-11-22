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

/// The places, the looks and whether they are shown letters, of each span of `text`.
fn parts_of(text: &EmacsText) -> Vec<(u32, u32, Vec<EmacsLook>, bool)> {
    text.spans
        .iter()
        .map(|span| (span.from, span.to, span.looks.clone(), span.letters))
        .collect()
}

// リストの段落の前には、深さの字下げと記号がある。どちらも表示の文字ではない。
#[test]
fn a_paragraph_of_a_list_has_the_indent_of_its_depth_and_its_mark_which_are_not_letters() {
    let text = text_of("- 一つ\n  - 二つ");

    assert_eq!(text.text, "• 一つ\n  • 二つ\n");
    assert_eq!(
        parts_of(&text),
        [
            (0, 2, vec![EmacsLook::Mark], false),
            (5, 7, vec![], false),
            (7, 9, vec![EmacsLook::Mark], false),
        ]
    );
}

// 引用の段落の前には、引用の深さだけ引用の印がある。表示の文字ではない。
#[test]
fn a_quoted_paragraph_has_a_quote_mark_for_each_quote_which_is_not_letters() {
    let text = text_of("> 外\n>\n> > 中");

    assert_eq!(text.text, "▎ 外\n▎ ▎ 中\n");
    assert_eq!(
        parts_of(&text),
        [
            (0, 2, vec![EmacsLook::Quote], false),
            (4, 8, vec![EmacsLook::Quote], false),
        ]
    );
}
