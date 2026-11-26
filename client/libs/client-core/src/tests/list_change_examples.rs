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

// 行の例のidは重ならない。シェルは、回した例をidで数える。
#[test]
fn the_row_examples_have_ids_of_their_own() {
    let examples = row_examples::<()>();
    let ids: BTreeSet<&str> = examples.iter().map(|example| example.id).collect();

    assert_eq!(ids.len(), examples.len());
}

// 行の例は、空の並びへの差分、空の並びからの差分、先頭と末尾に入れる差分を含む。
#[test]
fn the_row_examples_change_empty_lists_and_both_ends() {
    let examples = row_examples::<()>();
    let has = |test: &dyn Fn(&ListChange<_>, usize) -> bool| {
        examples.iter().any(|example| {
            example
                .changes
                .iter()
                .any(|change| test(change, example.before.len()))
        })
    };

    assert!(examples
        .iter()
        .any(|example| example.before.is_empty() && !example.after.is_empty()));
    assert!(examples
        .iter()
        .any(|example| !example.before.is_empty() && example.after.is_empty()));
    assert!(has(&|change, _| matches!(
        change,
        ListChange::Insert { at: 0, .. }
    )));
    assert!(has(
        &|change, len| matches!(change, ListChange::Insert { at, .. } if *at as usize == len)
    ));
}
