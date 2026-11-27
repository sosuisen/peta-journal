//! Changing the parent and the order of an item by moving its card (ADR 053).

use super::*;

/// The parent `g` with the cards of one and two under it, one above the other, and the card
/// of three alone at the given bounds.
fn atlas_with_parent(three: &str) -> Harness {
    atlas_of(
        vec![child("one"), child("three"), child("two")],
        vec![
            parent_item("g", 1, "", &[]),
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
            parent_item("g", 1, "", &[]),
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

// 同時に落とされた複数のカードは、1回の送信で、それぞれの場所から入り先が決まる。同じ兄弟の前に入るカードは、上のものから並ぶ。
#[test]
fn cards_dropped_at_once_go_in_each_by_where_it_lies_in_one_batch() {
    let mut h = atlas_of(
        vec![
            child("five"),
            child("four"),
            child("one"),
            child("three"),
            child("two"),
        ],
        vec![
            parent_item("g", 1, "", &[]),
            card_item("one", 2, Some("0,0,0,40")),
            card_item("two", 2, Some("0,100,0,40")),
            card_item("five", 1, Some("0,60,0,40")),
            card_item("three", 1, Some("0,50,0,40")),
            card_item("four", 1, Some("0,150,0,40")),
        ],
    );
    move_under(&mut h, &["a/b/four", "a/b/five", "a/b/three"], "g");

    assert_eq!(
        batches(&h),
        [[
            leveled("i-three", 2),
            leveled("i-five", 2),
            moved(&["i-three", "i-five"], "i-one"),
            leveled("i-four", 2),
            moved(&["i-four"], "i-two"),
        ]]
    );
}

/// The parent `g` with the card of one, the parent `p` with the card of two, and the card of
/// three under it, and the card of four alone at the given bounds.
fn atlas_with_nested_parents(four: &str) -> Harness {
    atlas_of(
        vec![child("four"), child("one"), child("three"), child("two")],
        vec![
            parent_item("g", 1, "", &[]),
            card_item("one", 2, Some("0,0,0,40")),
            parent_item("p", 2, "", &[("atlas_side", "left")]),
            card_item("two", 3, Some("0,100,0,40")),
            card_item("three", 2, Some("0,300,0,40")),
            card_item("four", 1, Some(four)),
        ],
    )
}

// 親の下の親にも入る。カードは、その親のすぐ下のレベルになる。
#[test]
fn a_card_goes_under_a_parent_that_is_under_a_parent_at_the_level_below_it() {
    let mut h = atlas_with_nested_parents("0,50,0,40");
    move_under(&mut h, &["a/b/four"], "p");

    assert_eq!(
        batches(&h),
        [[leveled("i-four", 3), moved(&["i-four"], "p")]]
    );
}

// 上の兄弟が親のときは、その木の後ろに入る。親には自分の`atlas_bounds`がないので、比べる相手にならない。
#[test]
fn a_card_goes_in_behind_the_tree_of_a_sibling_above_it_that_is_a_parent() {
    let mut h = atlas_with_nested_parents("0,200,0,40");
    move_under(&mut h, &["a/b/four"], "g");

    assert_eq!(
        batches(&h),
        [[leveled("i-four", 2), moved(&["i-four"], "i-two")]]
    );
}

// 内側の親から外側の親へ移ったカードは、外側の親の子になる。最後のカードが出て親でなくなった項目は、同じ送信で辺のプロパティを失う。
#[test]
fn an_item_whose_last_card_goes_under_another_parent_loses_its_side_in_the_same_batch() {
    let mut h = atlas_with_nested_parents("0,200,0,40");
    move_under(&mut h, &["a/b/two"], "g");

    assert_eq!(
        batches(&h),
        [[leveled("i-two", 2), moved(&["i-two"], "p"), unsided("p"),]]
    );
}

// すでにその親のすぐ下にあるカードは、動かない。子のない項目、存在しない項目、存在しないカードでは、何も送られない。
#[test]
fn a_card_right_under_the_parent_stays_and_no_card_goes_under_an_item_that_is_no_parent() {
    let mut h = atlas_with_parent("0,200,0,40");
    move_under(&mut h, &["a/b/one"], "g");
    move_under(&mut h, &["a/b/three"], "i-one");
    move_under(&mut h, &["a/b/three"], "none");
    move_under(&mut h, &["a/b/none"], "g");

    assert!(batches(&h).is_empty());
}

// リンクカードは、項目のidで名指しされ、ページのカードと同じに親の子になる。
#[test]
fn a_link_card_goes_under_a_parent_as_the_card_of_a_page_does() {
    let mut h = atlas_of(
        vec![child("one"), child("two")],
        vec![
            parent_item("g", 1, "", &[]),
            card_item("one", 2, Some("0,0,0,40")),
            card_item("two", 2, Some("0,100,0,40")),
            atlas_item("l", 1, "[[x/Sample]]", &[("atlas_bounds", "0,50,0,40")]),
        ],
    );
    move_under(&mut h, &["l"], "g");

    assert_eq!(batches(&h), [[leveled("l", 2), moved(&["l"], "i-one")]]);
}

// 親の下の親から出されたカードは、元の親の木の末尾の次に、元の親と同じレベルで入る。外側の親の子のままである。
#[test]
fn a_card_taken_out_of_a_parent_under_a_parent_goes_behind_its_tree_at_its_level() {
    let mut h = atlas_with_nested_parents("0,200,0,40");
    h.send(Event::UngroupAtlasCards {
        card_ids: vec!["a/b/two".into()],
    });

    assert_eq!(
        batches(&h),
        [[leveled("i-two", 2), moved(&["i-two"], "p"), unsided("p"),]]
    );
}
