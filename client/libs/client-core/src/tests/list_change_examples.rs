use crate::changes::examples::row_examples;
use crate::changes::ListChange;
use std::collections::BTreeSet;

// 行の例は、差分の5つの種類をすべて含む。
#[test]
fn the_row_examples_have_every_kind_of_change() {
    let kinds: BTreeSet<&str> = row_examples::<()>()
        .iter()
        .flat_map(|example| &example.changes)
        .map(|change| match change {
            ListChange::Insert { .. } => "Insert",
            ListChange::Remove { .. } => "Remove",
            ListChange::Move { .. } => "Move",
            ListChange::Update { .. } => "Update",
            ListChange::Reset { .. } => "Reset",
        })
        .collect();

    assert_eq!(
        kinds,
        BTreeSet::from(["Insert", "Remove", "Move", "Update", "Reset"])
    );
}
