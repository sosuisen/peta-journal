//! The Edit of a card and the Add Card of the canvas: the body of an item as Markdown.

use super::*;

fn edit(h: &mut Harness, card_id: &str, text: &str) {
    h.send(Event::SetAtlasCardText {
        card_id: card_id.into(),
        text: text.into(),
    });
}

/// An atlas with the card of one, the link card `l` and the plain card `p`
fn atlas_of_three() -> Harness {
    atlas_of(
        vec![child("one")],
        vec![
            card_item("one", 1, Some("0,0,0,40")),
            atlas_item("l", 1, "[[x/Target]]", &[("atlas_bounds", "10,20,0,40")]),
            atlas_item("p", 1, "plain", &[("atlas_bounds", "30,20,0,40")]),
        ],
    )
}

// Editで書いたMarkdownは、そのままカードの項目の本文になる。ページのカードはページで、リンクカードとノーマルカードは項目で名指す。前後の空白は取り除かれる。
#[test]
fn the_markdown_written_in_the_edit_of_a_card_is_the_body_of_its_item() {
    // One atlas for each: the outline read again after a change has only what the test answers
    let edited = |card_id: &str, text: &str| {
        let mut h = atlas_of_three();
        edit(&mut h, card_id, text);
        batches(&h)
    };

    assert_eq!(
        [
            edited("a/b/one", "[[a/b/one]]\nabout one "),
            edited("l", " [[x/Target|Shown]]\nmore"),
            edited("p", "**plain** text"),
        ],
        [
            [[written("i-one", "[[a/b/one]]\nabout one")]],
            [[written("l", "[[x/Target|Shown]]\nmore")]],
            [[written("p", "**plain** text")]],
        ]
    );
}

// 本文が変わらないEdit、空のEdit、カードでないもののEditでは、何も送られない。
#[test]
fn an_edit_that_changes_nothing_or_empties_a_card_or_names_no_card_sends_nothing() {
    let mut h = atlas_of_three();
    edit(&mut h, "l", "[[x/Target]] ");
    edit(&mut h, "p", "  ");
    edit(&mut h, "nothing", "text");

    assert!(batches(&h).is_empty());
}

fn add(h: &mut Harness, x: f64, y: f64, text: &str) {
    h.send(Event::AddAtlasCard {
        x,
        y,
        text: text.into(),
    });
}

// Add Cardで書いたMarkdownは、ページの末尾のレベル1の項目になり、右クリックした点を丸めた位置に置かれる。1回の送信である。
#[test]
fn an_added_card_is_an_item_at_the_end_with_its_place_in_one_batch() {
    let mut h = atlas_of(vec![], vec![]);
    add(&mut h, 10.4, -20.6, " a note\nmore ");

    assert_eq!(
        batches(&h),
        [[made(
            ItemPosition::Last,
            "a note\nmore",
            &[("atlas_bounds", "10,-21,0,40")]
        )]]
    );
}

// 1行目がwikilinkだけのカードを足すと、読み直したアウトラインで、リンクカードとして描かれる。
#[test]
fn an_added_card_whose_first_line_is_a_link_is_a_link_card() {
    let mut h = atlas_of(vec![], vec![]);
    answer_outline(
        &mut h,
        vec![atlas_item(
            "new",
            1,
            "[[x/Target]]",
            &[("atlas_bounds", "0,0,0,40")],
        )],
    );
    add(&mut h, 0.0, 0.0, "[[x/Target]]");

    assert_eq!(
        batches(&h),
        [[made(
            ItemPosition::Last,
            "[[x/Target]]",
            &[("atlas_bounds", "0,0,0,40")]
        )]]
    );
    let links = h.view().atlas_links;
    assert_eq!(links[0].target.as_deref(), Some("x/Target"));
}

// 空のテキストや、数でない点では、何も足さない。
#[test]
fn an_empty_card_or_one_at_no_point_is_not_added() {
    let mut h = atlas_of(vec![], vec![]);
    add(&mut h, 0.0, 0.0, " \n ");
    add(&mut h, f64::NAN, 0.0, "text");
    add(&mut h, 0.0, f64::INFINITY, "text");

    assert!(batches(&h).is_empty());
}
