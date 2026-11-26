//! The examples that the web shell runs in its tests as the core does (ADR 089, ADR 090), as
//! the JSON that crosses the border of the web shell.

use crate::body::HtmlBody;
use client_core::changes::examples::row_examples;
use facet::Facet;
use serde_json::{json, Value};

/// The part of the web shell in `list-changes.json`: the examples of the changes to the rows.
/// Each holds the rows before, the changes and the rows after, as the shell reads them.
pub fn list_changes() -> Value {
    let rows: Vec<Value> = row_examples::<HtmlBody>()
        .into_iter()
        .map(|example| {
            json!({
                "id": example.id,
                "before": crossing(&example.before),
                "changes": crossing(&example.changes),
                "after": crossing(&example.after),
            })
        })
        .collect();
    json!({ "rows": rows })
}

/// `value` as the JSON of facet that crosses the border.
fn crossing<'a, T: Facet<'a>>(value: &T) -> Value {
    let text = facet_json::to_string(value).expect("the examples are always valid JSON");
    serde_json::from_str(&text).expect("facet writes JSON that serde reads")
}

#[cfg(test)]
mod tests {
    use super::*;
    use client_core::changes::examples::is_current;
    use client_core::changes::samples::samples;
    use client_core::changes::Changes;
    use client_core::Event;
    use facet_generate::reflection::RegistryBuilder;
    use std::path::PathBuf;

    /// The examples of the type `root` that crosses the border, for D2 (ADR 090): the samples
    /// of the core, each read as `T` and written as the JSON that crosses.
    fn border_examples<T: Facet<'static>>(root: &str) -> Value {
        let registry = RegistryBuilder::new()
            .add_type::<T>()
            .and_then(RegistryBuilder::build)
            .expect("the types of the border are described");
        let examples: Vec<Value> = samples(&registry, root)
            .into_iter()
            .map(|sample| {
                let value: T =
                    crate::border::read(&sample.value.to_string()).unwrap_or_else(|error| {
                        panic!("the sample {} is no {root}: {error}", sample.id)
                    });
                json!({ "id": sample.id, "value": crossing(&value) })
            })
            .collect();
        Value::Array(examples)
    }

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

    /// The file of the examples of the values of the border of the web shell (ADR 090 D2).
    fn border_file() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/shell-check/common/border-web.json")
    }

    // 境界の値の例のファイルの節は、今の見本と同じである。シェルが読む`Changes`と、シェルが書く`Event`である。
    #[test]
    fn the_examples_of_the_border_values_are_current() {
        is_current(
            &border_file(),
            "changes",
            border_examples::<Changes<HtmlBody>>("Changes"),
        )
        .unwrap();
        is_current(&border_file(), "events", border_examples::<Event>("Event")).unwrap();
    }
}
