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

// 親の子になるカードは、自分よりYが大きい最初の兄弟の前に入る。上に置いたものが、`.md`でも上に来る。
#[test]
fn a_card_goes_in_before_the_first_sibling_that_lies_below_it() {
    let sent = |three: &str| {
        let mut h = atlas_with_parent(three);
        move_under(&mut h, &["a/b/three"], "g");
        batches(&h).remove(0)
    };

    // Between the two: behind the sibling above it
    assert_eq!(
        sent("0,50,0,40"),
        [leveled("i-three", 2), moved(&["i-three"], "i-one")]
    );
    // Above both: right behind the parent
    assert_eq!(
        sent("0,-50,0,40"),
        [leveled("i-three", 2), moved(&["i-three"], "g")]
    );
}

// Yが同じなら、Xが小さい方が上である。XもYも同じなら、後から足した方が下である。
#[test]
fn of_two_cards_at_the_same_height_the_left_one_is_the_upper_and_the_later_one_the_lower() {
    let sent = |three: &str| {
        let mut h = atlas_with_parent(three);
        move_under(&mut h, &["a/b/three"], "g");
        batches(&h).remove(0)
    };

    // Left of the second sibling, at its height
    assert_eq!(
        sent("-10,100,0,40"),
        [leveled("i-three", 2), moved(&["i-three"], "i-one")]
    );
    // Right of the first sibling, at its height
    assert_eq!(
        sent("10,0,0,40"),
        [leveled("i-three", 2), moved(&["i-three"], "i-one")]
    );
    // Where the first sibling lies
    assert_eq!(
        sent("0,0,0,40"),
        [leveled("i-three", 2), moved(&["i-three"], "i-one")]
    );
}

// `atlas_bounds`のない兄弟は、比べる相手にならない。その兄弟の順は変わらない。
#[test]
fn a_sibling_without_a_place_is_not_compared_and_keeps_its_place_in_the_order() {
    let mut h = atlas_of(
        vec![child("one"), child("three"), child("two")],
        vec![
            atlas_item("g", 1, "", &[]),
            card_item("one", 2, None),
            atlas_item("note", 2, "sample", &[]),
            card_item("two", 2, Some("0,100,0,40")),
            card_item("three", 1, Some("0,50,0,40")),
        ],
    );
    move_under(&mut h, &["a/b/three"], "g");

    assert_eq!(
        batches(&h),
        [[leveled("i-three", 2), moved(&["i-three"], "note")]]
    );
}
