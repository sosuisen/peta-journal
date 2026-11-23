use crate::changes::{list_changes, ListChange};
use proptest::prelude::*;

/// An item of a list in a test: its id, and what it holds.
#[derive(Debug, Clone, PartialEq)]
struct Line {
    id: String,
    text: String,
}

fn line(id: &str, text: &str) -> Line {
    Line {
        id: id.into(),
        text: text.into(),
    }
}

/// The changes from `before` to `after`.
fn changes(before: &[Line], after: &[Line]) -> Vec<ListChange<Line>> {
    list_changes(before, after, |line| &line.id)
}

// 同じリストには、操作がない。
#[test]
fn the_same_list_has_no_changes() {
    let list = [line("a", "one"), line("b", "two")];

    assert_eq!(changes(&list, &list), vec![]);
}

// 中身の変わった行は、その位置の更新になる。
#[test]
fn an_item_whose_content_changed_is_updated_in_its_place() {
    let before = [line("a", "one"), line("b", "two")];
    let after = [line("a", "one"), line("b", "TWO")];

    assert_eq!(
        changes(&before, &after),
        vec![ListChange::Update {
            at: 1,
            item: line("b", "TWO")
        }]
    );
}

// 新しい行は、その位置への挿入になる。
#[test]
fn a_new_item_is_inserted_in_its_place() {
    let before = [line("a", "one"), line("c", "three")];
    let after = [line("a", "one"), line("b", "two"), line("c", "three")];

    assert_eq!(
        changes(&before, &after),
        vec![ListChange::Insert {
            at: 1,
            item: line("b", "two")
        }]
    );
}

// 消えた行は、後ろの位置から順に削除になる。
#[test]
fn items_that_are_gone_are_removed_from_the_last_place_on() {
    let before = [line("a", "one"), line("b", "two"), line("c", "three")];
    let after = [line("b", "two")];

    assert_eq!(
        changes(&before, &after),
        vec![ListChange::Remove { at: 2 }, ListChange::Remove { at: 0 }]
    );
}

// 順の変わった行は、移動になる。中身も変わっていれば、続けて更新になる。
#[test]
fn an_item_in_another_place_is_moved_and_then_updated_when_it_changed() {
    let before = [line("x", "one"), line("y", "two"), line("z", "three")];
    let after = [line("z", "THREE"), line("x", "one"), line("y", "two")];

    assert_eq!(
        changes(&before, &after),
        vec![
            ListChange::Move { from: 2, to: 0 },
            ListChange::Update {
                at: 0,
                item: line("z", "THREE")
            },
        ]
    );
}

// 前の行が1つも残らないときは、入れ替えが1つだけになる。別のページを開いたときである。
#[test]
fn a_list_that_keeps_no_item_is_reset_once() {
    let before = [line("a", "one"), line("b", "two")];
    let after = [line("c", "three"), line("d", "four")];

    assert_eq!(
        changes(&before, &after),
        vec![ListChange::Reset {
            items: after.to_vec()
        }]
    );
}

// 操作を順に当てると、前のリストが今のリストになる。削除、挿入、移動、更新が混ざっていても同じである。
#[test]
fn the_changes_applied_in_order_make_the_list_before_into_the_list_after() {
    let before = [
        line("a", "one"),
        line("b", "two"),
        line("c", "three"),
        line("d", "four"),
        line("e", "five"),
    ];
    let after = [
        line("e", "FIVE"),
        line("c", "three"),
        line("x", "new"),
        line("a", "one"),
        line("d", "FOUR"),
    ];

    assert_eq!(apply(&before, changes(&before, &after)), after);
}

/// A list of up to 8 items, each id at most once and in any order. An item holds one of a
/// few texts, so that an item of two lists often has another text in the other.
fn any_list() -> impl Strategy<Value = Vec<Line>> {
    let ids = ["a", "b", "c", "d", "e", "f", "g", "h"];
    proptest::sample::subsequence(ids.to_vec(), 0..=ids.len())
        .prop_shuffle()
        .prop_flat_map(|ids| {
            let texts = proptest::collection::vec(0..3u8, ids.len());
            (Just(ids), texts)
        })
        .prop_map(|(ids, texts)| {
            ids.iter()
                .zip(texts)
                .map(|(id, text)| line(id, &format!("text {text}")))
                .collect()
        })
}

proptest! {
    // IDが重ならない2つのリストなら、どの組でも、操作を順に当てると、前のリストが今のリストになる。
    #[test]
    fn the_changes_applied_in_order_make_any_list_into_any_other(
        before in any_list(),
        after in any_list(),
    ) {
        prop_assert_eq!(apply(&before, changes(&before, &after)), after);
    }
}

/// The list as a shell has it after it applied the changes to `list`, one after another.
fn apply(list: &[Line], changes: Vec<ListChange<Line>>) -> Vec<Line> {
    let mut list = list.to_vec();
    for change in changes {
        match change {
            ListChange::Insert { at, item } => list.insert(at as usize, item),
            ListChange::Remove { at } => {
                list.remove(at as usize);
            }
            ListChange::Move { from, to } => {
                let item = list.remove(from as usize);
                list.insert(to as usize, item);
            }
            ListChange::Update { at, item } => list[at as usize] = item,
            ListChange::Reset { items } => list = items,
        }
    }
    list
}

// 空のリストに行が来たときは、入れ替えになる。空のページに最初の行が届いたときである。
#[test]
fn an_empty_list_that_gets_items_is_reset() {
    let after = [line("a", "one")];

    assert_eq!(
        changes(&[], &after),
        vec![ListChange::Reset {
            items: after.to_vec()
        }]
    );
}

// 前も今も空のときは、操作がない。
#[test]
fn two_empty_lists_have_no_changes() {
    assert_eq!(changes(&[], &[]), vec![]);
}

// 操作の行を別の形に変えても、位置はそのまま残る。シェルが、比べた行を送る形に変えるときである。
#[test]
fn a_change_keeps_its_places_when_its_items_take_another_form() {
    let changes = vec![
        ListChange::Insert {
            at: 0,
            item: line("a", "one"),
        },
        ListChange::Remove { at: 1 },
        ListChange::Move { from: 2, to: 0 },
        ListChange::Update {
            at: 1,
            item: line("b", "two"),
        },
        ListChange::Reset {
            items: vec![line("c", "three"), line("d", "four")],
        },
    ];

    let texts: Vec<ListChange<String>> = changes
        .into_iter()
        .map(|change| change.map(|line| line.text))
        .collect();

    assert_eq!(
        texts,
        vec![
            ListChange::Insert {
                at: 0,
                item: "one".to_string()
            },
            ListChange::Remove { at: 1 },
            ListChange::Move { from: 2, to: 0 },
            ListChange::Update {
                at: 1,
                item: "two".to_string()
            },
            ListChange::Reset {
                items: vec!["three".to_string(), "four".to_string()]
            },
        ]
    );
}
