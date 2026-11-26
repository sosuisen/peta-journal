//! The examples that the web shell runs in its tests as the core does (ADR 089, ADR 090), as
//! the JSON that crosses the border of the web shell.

use serde_json::Value;

/// The part of the web shell in `list-changes.json`: the examples of the changes to the rows.
pub fn list_changes() -> Value {
    Value::Null
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

    // 例のファイルの`web`の節は、今の行の例と同じである。`PETA_WRITE_EXAMPLES=1`を付けて回すと、書き直す。
    #[test]
    fn the_web_part_of_the_list_change_examples_is_current() {
        is_current(&list_changes_file(), "web", list_changes()).unwrap();
    }
}
