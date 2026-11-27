//! The examples that the native shells run in their tests as the core does (ADR 089, ADR 090),
//! as the bytes that cross the border of a native shell.

use crate::{Changes, RowChange};
use client_core::changes::examples::row_examples;
use client_core::RichText;
use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use serde_json::{json, Value};

/// The part of the native shells in `list-changes.json`: the examples of the changes to the
/// rows. A native shell reads only `Changes` from the border, so each of the rows before, the
/// changes and the rows after is the bytes of a `Changes`, in hexadecimal. The rows before and
/// after are a `Reset` of the rows.
fn list_changes() -> Value {
    let rows: Vec<Value> = row_examples::<RichText>()
        .into_iter()
        .map(|example| {
            json!({
                "id": example.id,
                "before": crossing(vec![RowChange::Reset { items: example.before }]),
                "changes": crossing(example.changes.into_iter().map(RowChange::from).collect()),
                "after": crossing(vec![RowChange::Reset { items: example.after }]),
            })
        })
        .collect();
    json!({ "rows": rows })
}

/// The changes `rows`, with no panel, as the hexadecimal of the bytes that cross the border.
fn crossing(rows: Vec<RowChange<RichText>>) -> String {
    let changes = Changes {
        rows,
        headings: Vec::new(),
        index_pages: Vec::new(),
        panels: Vec::new(),
    };
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &changes).expect("the changes are always written");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use client_core::changes::examples::is_current;
    use std::path::PathBuf;

    /// The file of the examples of the changes to lists, in the repository.
    fn list_changes_file() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/shell-check/common/list-changes.json")
    }

    // 例のファイルの`native`の節は、今の行の例と同じである。`PETA_WRITE_EXAMPLES=1`を付けて回すと、書き直す。
    #[test]
    fn the_native_part_of_the_list_change_examples_is_current() {
        is_current(&list_changes_file(), "native", list_changes()).unwrap();
    }
}
