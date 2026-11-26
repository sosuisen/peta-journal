//! The examples of the values of the border for the native shells, for D2 (ADR 090): the
//! samples of the core, each read as its type and written as the bytes that cross the border,
//! in hexadecimal. Only with the feature `facet_typegen`, as the samples come from the
//! description of the types: `cargo test -p client-native-bridge --features facet_typegen`.
#![cfg(feature = "facet_typegen")]

use client_core::changes::examples::is_current;
use client_core::changes::samples::{samples, uncovered};
use client_core::Event;
use client_native_bridge::Changes;
use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use facet::Facet;
use facet_generate::reflection::RegistryBuilder;
use facet_generate::Registry;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;

/// The description of the types of `T`, as the generators of the native types read it.
fn registry<T: Facet<'static>>() -> Registry {
    RegistryBuilder::new()
        .add_type::<T>()
        .and_then(RegistryBuilder::build)
        .expect("the types of the border are described")
}

/// The examples of the type `root`: each sample read as `T`, and its bytes in hexadecimal.
fn border_examples<T: Facet<'static> + Serialize>(root: &str) -> Value {
    let examples: Vec<Value> = samples(&registry::<T>(), root)
        .into_iter()
        .map(|sample| {
            let value: T = facet_json::from_str(&sample.value.to_string())
                .unwrap_or_else(|error| panic!("the sample {} is no {root}: {error}", sample.id));
            let mut bytes = Vec::new();
            BincodeFfiFormat::serialize(&mut bytes, &value).expect("the samples are written");
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            json!({ "id": sample.id, "bytes": hex })
        })
        .collect();
    Value::Array(examples)
}

/// The file of the examples of the values of the border of the native shells.
fn border_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/shell-check/common/border-native.json")
}

// 境界の値の例は、`Changes`と`Event`から届くすべての列挙型の、すべての変種を持つ。
#[test]
fn the_examples_of_the_border_values_hold_every_variant() {
    for (registry, root) in [
        (registry::<Changes>(), "Changes"),
        (registry::<Event>(), "Event"),
    ] {
        assert_eq!(
            uncovered(&registry, root),
            Vec::<(String, String)>::new(),
            "{root}"
        );
    }
}

// 境界の値の例のファイルの節は、今の見本と同じである。`PETA_WRITE_EXAMPLES=1`を付けて回すと、書き直す。
#[test]
fn the_examples_of_the_border_values_are_current() {
    is_current(
        &border_file(),
        "changes",
        border_examples::<Changes>("Changes"),
    )
    .unwrap();
    is_current(&border_file(), "events", border_examples::<Event>("Event")).unwrap();
}
