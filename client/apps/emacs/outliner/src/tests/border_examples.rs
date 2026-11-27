//! The examples of the lines that cross the border of the Emacs shell, from its Rust to its
//! Lisp (ADR 078), for D2 (ADR 089, ADR 090). The Lisp reads each of them in a test of ERT.
//!
//! Only the lines to Emacs have examples. The lines from Emacs are written in Lisp by hand,
//! from keys that nothing generates: there is nothing of the Rust types to hold them against.

use crate::protocol::{write_line, Outbound};
use facet_generate::reflection::format::{ContainerFormat, EnumTagging, Format, VariantFormat};
use facet_generate::reflection::RegistryBuilder;
use facet_generate::Registry;
use serde_json::Value;
use std::collections::BTreeSet;

/// An example of a line to Emacs, with the name that the tests know it by.
struct Example {
    id: &'static str,
    line: Outbound,
}

/// The examples of the lines to Emacs.
fn examples() -> Vec<Example> {
    Vec::new()
}

/// The description of the types of the lines to Emacs, as the writer of the Lisp reads it.
fn registry() -> Registry {
    RegistryBuilder::new()
        .add_type::<Outbound>()
        .and_then(RegistryBuilder::build)
        .expect("the protocol has a registry")
}

/// A variant of an enum, as the name of the enum and the name of the variant as it is sent.
type Variant = (String, String);

/// Every variant of every enum of the lines to Emacs. The registry holds the types that
/// `Outbound` reaches, and no other.
fn every_variant(registry: &Registry) -> BTreeSet<Variant> {
    let mut all = BTreeSet::new();
    for (name, format) in registry {
        if let ContainerFormat::Enum(variants, _, _) = format {
            for variant in variants.values() {
                all.insert((name.name.clone(), variant.name.clone()));
            }
        }
    }
    all
}

/// Adds the variants of the enums that `value`, the JSON of a `format`, holds to `held`.
fn hold(registry: &Registry, format: &Format, value: &Value, held: &mut BTreeSet<Variant>) {
    match format {
        Format::TypeName(name) => match &registry[name] {
            ContainerFormat::Struct(fields, _) => {
                for field in fields {
                    hold(registry, &field.value, &value[&field.name], held);
                }
            }
            ContainerFormat::Enum(variants, tagging, _) => {
                // The kind is under the tag, or it is the one key of the value, or the value
                // itself when the kind holds nothing
                let (kind, inside) = match (tagging, value) {
                    (EnumTagging::Internal { tag }, _) => (value[tag].as_str(), value),
                    (_, Value::Object(map)) => {
                        let (kind, inside) = map.iter().next().expect("a kind has a name");
                        (Some(kind.as_str()), inside)
                    }
                    _ => (value.as_str(), value),
                };
                let variant = variants
                    .values()
                    .find(|variant| Some(variant.name.as_str()) == kind)
                    .unwrap_or_else(|| panic!("{value} is no {}", name.name));
                held.insert((name.name.clone(), variant.name.clone()));
                match &variant.value {
                    VariantFormat::Struct(fields) => {
                        for field in fields {
                            hold(registry, &field.value, &inside[&field.name], held);
                        }
                    }
                    VariantFormat::NewType(format) => hold(registry, format, inside, held),
                    _ => {}
                }
            }
            _ => {}
        },
        Format::Option(inner) if !value.is_null() => hold(registry, inner, value, held),
        Format::Seq(inner) => {
            for value in value.as_array().into_iter().flatten() {
                hold(registry, inner, value, held);
            }
        }
        _ => {}
    }
}

// 境界の例は、Emacsへ送る行のすべての種類と、その中のすべての列挙型のすべての変種を持つ。行の種類を足したら、その例も足す。
#[test]
fn the_examples_of_the_border_hold_every_kind_of_line_and_every_variant_inside() {
    let registry = registry();
    let outbound = registry
        .keys()
        .find(|name| name.name == "Outbound")
        .expect("the lines to Emacs are described")
        .clone();
    let mut held = BTreeSet::new();
    for example in examples() {
        let line: Value = serde_json::from_str(&write_line(&example.line)).unwrap();
        hold(
            &registry,
            &Format::TypeName(outbound.clone()),
            &line,
            &mut held,
        );
    }

    let missing: Vec<Variant> = every_variant(&registry)
        .difference(&held)
        .cloned()
        .collect();
    assert_eq!(missing, Vec::<Variant>::new());
}
