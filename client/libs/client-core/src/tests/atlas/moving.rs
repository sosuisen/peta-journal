//! Changing the parent and the order of an item by moving its card (ADR 053).

use super::*;

/// The parent `g` with the cards of one and two under it, one above the other, and the card
/// of three alone at the given bounds.
fn atlas_with_parent(three: &str) -> Harness {
    atlas_of(
        vec![child("one"), child("three"), child("two")],
        vec![
            atlas_item("g", 1, "", &[]),
            card_item("one", 2, Some("0,0,0,40")),
            card_item("two", 2, Some("0,100,0,40")),
            card_item("three", 1, Some(three)),
        ],
    )
}

fn move_under(h: &mut Harness, card_ids: &[&str], parent_id: &str) {
    h.send(Event::MoveAtlasCards {
        card_ids: card_ids.iter().map(|id| id.to_string()).collect(),
        parent_id: parent_id.into(),
    });
}

// 親のblobに落とされたカードは、1回の送信で、その親の子になる。下に兄弟がなければ、親の木の末尾に入る。
#[test]
fn a_card_dropped_on_the_blob_of_a_parent_goes_under_it_behind_the_siblings_above_it() {
    let mut h = atlas_with_parent("0,200,0,40");
    move_under(&mut h, &["a/b/three"], "g");

    assert_eq!(
        batches(&h),
        [[leveled("i-three", 2), moved(&["i-three"], "i-two")]]
    );
}
