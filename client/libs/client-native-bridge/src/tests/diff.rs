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
