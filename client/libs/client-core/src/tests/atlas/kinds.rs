//! How the items of the atlas page are drawn (ADR 053): page cards, plain cards and the
//! blobs of the parents.

use super::*;
use crate::rich_text::RichLink;
use crate::screens::atlas::CardTone;

/// An item of level 1 placed at the origin
fn placed(id: &str, body: &str) -> Item {
    atlas_item(id, 1, body, &[("atlas_bounds", "0,0,0,40")])
}

// ほかのフォルダのページへのカードは、リンクのラベルを出す。ラベルがなければ、リンク先の名前を出す。
#[test]
fn a_card_to_a_page_of_another_folder_shows_the_label_of_its_link() {
    let h = atlas_of(
        vec![],
        vec![
            placed("a", "[[x/Target|Shown]]"),
            placed("b", "[[x/y/Other]]"),
        ],
    );

    let labels: Vec<String> = h.view().atlas_links.into_iter().map(|l| l.label).collect();
    assert_eq!(labels, ["Shown", "Other"]);
}

// 1行目が1つのwikilinkだけでない項目は、ノーマルカードである。1行目の文字を出し、リンク先を持たない。
#[test]
fn an_item_whose_first_line_is_not_one_link_is_a_plain_card_showing_its_first_line() {
    let h = atlas_of(
        vec![],
        vec![placed("p", "**Note** with [[x/A|a link]]\nmore lines")],
    );

    let card = &h.view().atlas_links[0];
    assert_eq!(
        (card.tone, card.label.as_str(), card.target.as_deref()),
        (CardTone::Plain, "Note with a link", None)
    );
}

// カードの1行目のリンクは、ページのリンクもウェブのリンクも、押せる文字の並びとして渡される。
#[test]
fn the_links_of_the_first_line_of_a_card_go_with_its_letters() {
    let h = atlas_of(
        vec![],
        vec![placed(
            "p",
            "see [[x/A|a page]] or [a site](https://example.com)\n[[x/B]]",
        )],
    );

    let links: Vec<RichLink> = h.view().atlas_links[0]
        .runs
        .iter()
        .filter_map(|run| run.link.clone())
        .collect();
    assert_eq!(
        links,
        [
            RichLink::Page {
                url: "/pages/x/A".into()
            },
            RichLink::Web {
                url: "https://example.com".into()
            },
        ]
    );
}

// カードは、2行目より後も含む本文の全体を持つ。右クリックのメニューから、全文を見せるためである。
#[test]
fn a_card_carries_the_whole_text_of_its_item() {
    let h = atlas_of(
        vec![child("one")],
        vec![
            atlas_item(
                "i-one",
                1,
                "[[a/b/one]]\nmore of one",
                &[("atlas_bounds", "0,0,0,40")],
            ),
            placed("p", "note\nmore of the note"),
        ],
    );

    let view = h.view();
    assert_eq!(
        (view.atlas_cards[0].text.as_str(), view.atlas_links[0].text.as_str()),
        ("[[a/b/one]]\nmore of one", "note\nmore of the note")
    );
}
