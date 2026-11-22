use crate::diff::{list_changes, ListChange};

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
