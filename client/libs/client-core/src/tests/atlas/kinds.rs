//! How the items of the atlas page are drawn (ADR 053): page cards, plain cards and the
//! blobs of the parents.

use super::*;

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
