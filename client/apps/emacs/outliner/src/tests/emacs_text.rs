use crate::emacs_text::{EmacsLook, EmacsText, SharedText};
use client_core::rich_text::{RichBlock, RichLink, RichParagraph, RichText};
use client_core::BodyModel;
use proptest::prelude::*;

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

// 見出しの見た目は、1から6のどの深さにもある。
#[test]
fn every_level_of_a_heading_from_one_to_six_has_its_own_look() {
    let looks: Vec<Vec<EmacsLook>> = (1..=6)
        .map(|level| {
            let text = text_of(&format!("{} 題", "#".repeat(level)));
            text.spans
                .iter()
                .flat_map(|span| span.looks.clone())
                .collect()
        })
        .collect();

    assert_eq!(
        looks,
        [
            vec![EmacsLook::Heading1],
            vec![EmacsLook::Heading2],
            vec![EmacsLook::Heading3],
            vec![EmacsLook::Heading4],
            vec![EmacsLook::Heading5],
            vec![EmacsLook::Heading6],
        ]
    );
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

// 区切り線は、表示の文字ではない線である。
#[test]
fn a_rule_is_a_line_that_is_not_letters() {
    let text = text_of("上\n\n---\n\n下");

    let line = "─".repeat(20);
    assert_eq!(text.text, format!("上\n{line}\n下\n"));
    assert_eq!(parts_of(&text), [(2, 22, vec![EmacsLook::Rule], false)]);
}

// コードブロックの行は、コードの見た目を持つ。
#[test]
fn the_lines_of_a_code_block_have_the_look_of_code() {
    let text = text_of("前\n```\nlet a;\nlet b;\n```");

    assert_eq!(text.text, "前\nlet a;\nlet b;\n");
    assert_eq!(parts_of(&text), [(2, 15, vec![EmacsLook::CodeBlock], true)]);
}

// 画像は、代わりの文字とその改行の範囲に画像を持つ。どちらも表示の文字ではない。
#[test]
fn an_image_is_on_the_span_of_its_alt_text_which_is_not_letters() {
    let text = text_of("前\n\n![猫](assets/sample.png)\n\n後");

    assert_eq!(text.text, "前\n猫\n後\n");
    let images: Vec<(u32, u32, Option<String>, bool)> = text
        .spans
        .iter()
        .map(|span| {
            let path = span.image.as_ref().map(|image| image.path.clone());
            (span.from, span.to, path, span.letters)
        })
        .collect();
    assert_eq!(images, [(2, 4, Some("assets/sample.png".into()), false)]);
}

// 代わりの文字のない画像は、`image`の文字で見える。改行だけでは、画像を見せる場所がない。
#[test]
fn an_image_without_alt_text_shows_as_the_word_image() {
    let text = text_of("![](assets/sample.png)");

    assert_eq!(text.text, "image\n");
    assert_eq!(parts_of(&text), [(0, 6, vec![], false)]);
}

// リストの中のコードブロックも、項目の記号と字下げの後ろに平らになる。
#[test]
fn a_code_block_in_a_list_is_flat_behind_the_mark_and_the_indent_of_its_entry() {
    let text = text_of("- 前\n\n  ```\n  a\n  b\n  ```");

    assert_eq!(text.text, "• 前\n  a\n  b\n");
    assert_eq!(
        parts_of(&text),
        [
            (0, 2, vec![EmacsLook::Mark], false),
            (4, 6, vec![], false),
            (6, 8, vec![EmacsLook::CodeBlock], true),
            (8, 10, vec![], false),
            (10, 11, vec![EmacsLook::CodeBlock], true),
        ]
    );
}

// 引用の中のコードブロックも、行ごとに引用の印の後ろに平らになる。
#[test]
fn a_code_block_in_a_quote_is_flat_behind_a_quote_mark_on_each_line() {
    let text = text_of("> 前\n>\n> ```\n> a\n> ```");

    assert_eq!(text.text, "▎ 前\n▎ a\n");
    assert_eq!(
        parts_of(&text),
        [
            (0, 2, vec![EmacsLook::Quote], false),
            (4, 6, vec![EmacsLook::Quote], false),
            (6, 7, vec![EmacsLook::CodeBlock], true),
        ]
    );
}

// 表の升は、全角を2と数えた幅で、列ごとにそろう。升の間の縦線は、升の後ろの改行の代わりの表示の文字である。
#[test]
fn the_cells_of_a_table_line_up_in_columns_and_the_bar_stands_for_the_line_break_of_a_cell() {
    let text = text_of("| 名前 | 値 |\n|---|---|\n| a | 長い値 |");

    assert_eq!(text.text, "名前 │ 値\na    │ 長い値\n");
    assert_eq!(
        parts_of(&text),
        [
            (0, 2, vec![EmacsLook::TableHeader], true),
            (2, 3, vec![], false),
            (4, 5, vec![], false),
            (5, 6, vec![EmacsLook::TableHeader], true),
            (8, 12, vec![], false),
            (13, 14, vec![], false),
        ]
    );
}

/// Parts of bodies of every kind that `RichText` reads.
const PARTS: &[&str] = &[
    "plain 文字",
    "**bold** and *it* ~~struck~~ ==mark==",
    "# 見出し **強く**",
    "- a\n- b\n  - c",
    "1. one\n2. two",
    "- [ ] task\n- [x] done",
    "> quote\n>\n> > deep",
    "```\ncode\nmore\n```",
    "| a | b |\n|---|---|\n| 長い | x |",
    "![猫](assets/sample.png)",
    "![](assets/sample.png)",
    "---",
    "see [link](https://example.com/) and [[ページ]] #tag",
    "- item\n\n  ```\n  in list\n  ```",
    "> q\n>\n> ```\n> c\n> ```",
    "a | b",
];

/// The shown text of `blocks` as the native shells count it: the letters of each paragraph
/// and of the lines of each code block, each with a line break after it.
fn native_shown(blocks: &[RichBlock], shown: &mut String) {
    let paragraph = |paragraph: &RichParagraph, shown: &mut String| {
        paragraph
            .runs
            .iter()
            .for_each(|run| shown.push_str(&run.text));
        shown.push('\n');
    };
    for block in blocks {
        match block {
            RichBlock::Text { paragraphs } => {
                paragraphs.iter().for_each(|each| paragraph(each, shown))
            }
            RichBlock::Code { lines, .. } => {
                shown.push_str(lines);
                shown.push('\n');
            }
            RichBlock::Table { rows } => rows
                .iter()
                .flat_map(|row| &row.cells)
                .for_each(|cell| paragraph(cell, shown)),
            RichBlock::Image { .. } => {}
            RichBlock::Entries { entries, .. } => entries
                .iter()
                .for_each(|entry| native_shown(&entry.blocks, shown)),
            RichBlock::Quote { blocks, .. } => native_shown(blocks, shown),
        }
    }
}

/// The letters of `text` that are shown letters: those of no span that Rust added.
fn emacs_letters(text: &EmacsText) -> Vec<char> {
    text.text
        .chars()
        .enumerate()
        .filter(|&(at, _)| {
            let at = u32::try_from(at).unwrap();
            !text
                .spans
                .iter()
                .any(|span| !span.letters && span.from <= at && at < span.to)
        })
        .map(|(_, letter)| letter)
        .collect()
}

proptest! {
    // 表示の文字だけを拾うと、ネイティブと同じ表示の文字になる。表の縦線は、升の後ろの改行の代わりである。
    #[test]
    fn the_shown_letters_of_an_emacs_text_are_those_of_the_native_shells(
        parts in prop::collection::vec(prop::sample::select(PARTS), 1..6)
    ) {
        let markdown = parts.join("\n\n");
        let mut native = String::new();
        native_shown(&RichText::of_markdown(&markdown, "tags/").blocks, &mut native);
        let native: Vec<char> = native.chars().collect();

        let emacs = emacs_letters(&text_of(&markdown));

        prop_assert_eq!(emacs.len(), native.len(), "{:?}", markdown);
        for (shown, written) in native.iter().zip(&emacs) {
            prop_assert!(
                shown == written || (*shown == '\n' && *written == '│'),
                "{:?}: {:?} for {:?}", markdown, written, shown
            );
        }
    }
}

// 別々に作った同じ本文のSharedTextは等しい。同じものを共有していなくても、行は変わっていない。
#[test]
fn shared_texts_made_apart_from_the_same_body_are_equal() {
    let one = SharedText::of_markdown("**本文**", "tags/");
    let other = SharedText::of_markdown("**本文**", "tags/");

    assert_eq!(one, other);
    assert_ne!(one, SharedText::of_markdown("本文", "tags/"));
}
