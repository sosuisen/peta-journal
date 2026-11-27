//! The examples of the lines that cross the border of the Emacs shell, from its Rust to its
//! Lisp (ADR 078), for D2 (ADR 089, ADR 090). The Lisp reads each of them in a test of ERT.
//!
//! Only the lines to Emacs have examples. The lines from Emacs are written in Lisp by hand,
//! from keys that nothing generates: there is nothing of the Rust types to hold them against.

use crate::protocol::{write_line, Outbound};
use client_core::changes::examples::is_current;
use client_core::changes::samples::{small_samples, uncovered, Sample, TEXTS};
use facet_generate::reflection::format::{ContainerFormat, EnumTagging};
use facet_generate::reflection::RegistryBuilder;
use facet_generate::Registry;
use serde_json::{json, Map, Value};
use std::path::PathBuf;

/// The description of the types of the lines to Emacs, as the writer of the Lisp reads it.
fn registry() -> Registry {
    RegistryBuilder::new()
        .add_type::<Outbound>()
        .and_then(RegistryBuilder::build)
        .expect("the protocol has a registry")
}

/// The key under which a line says its kind: the tag of `Outbound`.
fn tag(registry: &Registry) -> String {
    let outbound = registry
        .iter()
        .find(|(name, _)| name.name == "Outbound")
        .map(|(_, format)| format);
    match outbound {
        Some(ContainerFormat::Enum(_, EnumTagging::Internal { tag }, _)) => tag.clone(),
        other => panic!("a line to Emacs has its kind inside: {other:?}"),
    }
}

/// The line that the shell writes for `sample`. A sample has its kind outside, as the key of
/// what the kind holds, and a line has it inside, under `tag`.
fn line_of(sample: &Sample, tag: &str) -> String {
    let (kind, held) = sample
        .value
        .as_object()
        .and_then(|kinds| kinds.iter().next())
        .unwrap_or_else(|| panic!("the sample {} holds nothing", sample.id));
    let mut line = Map::from_iter([(tag.to_string(), json!(kind))]);
    line.extend(held.as_object().cloned().unwrap_or_default());
    let line: Outbound = facet_json::from_str(&Value::Object(line).to_string())
        .unwrap_or_else(|error| panic!("the sample {} is no line: {error}", sample.id));
    write_line(&line)
}

/// The examples of the lines to Emacs: the samples of the core with small numbers, each as the
/// text of the line that the shell writes. The numbers are small as Emacs does at once what a
/// line says: a place at the edge of the numbers is in no buffer.
fn examples() -> Value {
    let registry = registry();
    let tag = tag(&registry);
    let lines: Vec<Value> = small_samples(&registry, "Outbound")
        .iter()
        // The text itself, not a JSON value of it: Emacs reads the line as the shell writes it
        .map(|sample| json!({ "id": sample.id, "line": line_of(sample, &tag) }))
        .collect();
    Value::Array(lines)
}

/// The file of the examples of the lines to Emacs, in the repository (ADR 090 D2).
fn border_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tools/shell-check/common/border-emacs.json")
}

// 境界の例は、Emacsへ送る行のすべての種類と、その中のすべての列挙型のすべての変種を持つ。
#[test]
fn the_examples_of_the_border_hold_every_kind_of_line_and_every_variant_inside() {
    assert_eq!(
        uncovered(&registry(), "Outbound"),
        Vec::<(String, String)>::new()
    );
}

// 境界の例のファイルは、今の見本と同じである。`outbound`の節は行の例で、`texts`の節は例に出る文字である。Lispのテストは、この文字をidに持つ行から始める。`PETA_WRITE_EXAMPLES=1`を付けて回すと、書き直す。
#[test]
fn the_examples_of_the_border_are_current() {
    is_current(&border_file(), "outbound", examples()).unwrap();
    is_current(&border_file(), "texts", json!(TEXTS)).unwrap();
}
