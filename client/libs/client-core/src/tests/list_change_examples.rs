use crate::changes::examples::{is_part, row_examples};
use crate::changes::ListChange;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

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

/// A file in a folder of its own under the temporary folder, holding `text` when given.
fn examples_file(name: &str, text: Option<&str>) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("peta-examples-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    let file = folder.join("examples.json");
    if let Some(text) = text {
        fs::write(&file, text).unwrap();
    }
    file
}

// 例のファイルの節が今の例と違うと、書き直さずに、書き直し方を返す。
#[test]
fn a_part_that_differs_is_an_error_and_stays() {
    let file = examples_file("differs", Some(r#"{"web": 1}"#));

    let result = is_part(&file, "web", json!(2), false);

    assert!(result.unwrap_err().contains("PETA_WRITE_EXAMPLES=1"));
    assert_eq!(fs::read_to_string(&file).unwrap(), r#"{"web": 1}"#);
}

// 書き直すときは、その節だけを今の例にし、ほかの節は残す。
#[test]
fn writing_a_part_keeps_the_other_parts() {
    let file = examples_file("keeps", Some(r#"{"web": 1, "native": 3}"#));

    is_part(&file, "web", json!(2), true).unwrap();

    let written: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(written, json!({"web": 2, "native": 3}));
    assert_eq!(is_part(&file, "web", json!(2), false), Ok(()));
}

// 例のファイルがまだなければ、書き直すときに作る。
#[test]
fn writing_makes_the_file_that_is_not_there() {
    let file = examples_file("new", None);

    is_part(&file, "native", json!("00"), true).unwrap();

    assert_eq!(is_part(&file, "native", json!("00"), false), Ok(()));
}
