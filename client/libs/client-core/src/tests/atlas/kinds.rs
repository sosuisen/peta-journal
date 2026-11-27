//! How the items of the atlas page are drawn (ADR 053): page cards, plain cards and the
//! blobs of the parents.

use super::*;
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
