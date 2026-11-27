//! The examples of the changes to the rows that the Lisp of the Emacs shell applies as the core
//! does (ADR 089, ADR 090 D1), as the lines that cross the border of the shell (ADR 078). The
//! Lisp applies each of them to a buffer in a test of ERT.
//!
//! The lists before and after, and the changes between them, are those of the core
//! (`row_examples`). Only their form is of this crate: each row is a `RowView`, as the shell
//! makes it of a row of the core.

use crate::emacs_text::{EmacsText, SharedText};
use crate::protocol::{write_line, Outbound, RowView};
use client_core::changes::examples::{is_current, row_examples};
use client_core::changes::ListChange;
use client_core::view::Row;
use serde_json::{json, Value};
use std::path::PathBuf;

/// The page of every example. The lines of one example name the same page, as the lines of
/// the changes within a page do.
const PAGE: &str = "page";

/// The row of the core `row` as Emacs is told of it.
fn view(row: Row<SharedText>) -> RowView {
    RowView::of(&row, EmacsText::clone(&row.body_model.0))
}

/// The line that tells Emacs of the changes `changes` to its rows.
fn line(changes: Vec<ListChange<Row<SharedText>>>) -> String {
    write_line(&Outbound::Rows {
        ack: 0,
        page_id: Some(PAGE.to_string()),
        changes: changes.into_iter().map(|change| change.map(view)).collect(),
    })
}

/// The part of the Emacs shell in `list-changes.json`: the examples of the changes to the
/// rows. Emacs reads its rows only from the lines of the shell, so each of the rows before,
/// the changes and the rows after is the text of such a line. The rows before and after are a
/// `Reset` of the rows.
fn list_changes() -> Value {
    let rows: Vec<Value> = row_examples::<SharedText>()
        .into_iter()
        .map(|example| {
            json!({
                "id": example.id,
                "before": line(vec![ListChange::Reset { items: example.before }]),
                "changes": line(example.changes),
                "after": line(vec![ListChange::Reset { items: example.after }]),
            })
        })
        .collect();
    json!({ "rows": rows })
}

/// The file of the examples of the changes to lists, in the repository.
fn list_changes_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tools/shell-check/common/list-changes.json")
}

// 例のファイルの`emacs`の節は、今の行の例と同じである。前の並び、差分、あとの並びは、コアの例のままで、形だけがEmacsへ送る行である。`PETA_WRITE_EXAMPLES=1`を付けて回すと、書き直す。
#[test]
fn the_emacs_part_of_the_list_change_examples_is_current() {
    is_current(&list_changes_file(), "emacs", list_changes()).unwrap();
}
