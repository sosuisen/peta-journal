//! Sample values of the types that cross the border, for the examples of D2 (ADR 090): each
//! shell reads them with its generated types and writes them back as they were.
//!
//! The samples are made from the registry of facet_generate, the description of the types
//! that the generators of the shells read. So they hold what the generated types know, and a
//! new type or variant comes into the samples by itself.

use facet_generate::reflection::format::{ContainerFormat, Format, QualifiedTypeName};
use facet_generate::Registry;
use serde_json::Value;

/// A sample of a type, as the JSON of facet, with the name that the shells know it by.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub id: String,
    pub value: Value,
}

/// The samples of the type named `root` in `registry`.
pub fn samples(registry: &Registry, root: &str) -> Vec<Sample> {
    let _ = (registry, root);
    let _: Option<(&ContainerFormat, &Format, &QualifiedTypeName)> = None;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use facet::Facet;
    use facet_generate::reflection::RegistryBuilder;
    use serde_json::json;

    fn registry<'a, T: Facet<'a>>() -> Registry {
        RegistryBuilder::new()
            .add_type::<T>()
            .unwrap()
            .build()
            .unwrap()
    }

    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Shape {
        Dot,
        Circle(u32),
        Rect { wide: u32, high: u32 },
    }

    // 列挙型の型には、変種ごとに1つの見本がある。見本の名前は変種の名前で、値はfacetのJSONの形である。
    #[test]
    fn an_enum_has_a_sample_of_each_variant_named_by_it() {
        let samples = samples(&registry::<Shape>(), "Shape");

        let ids: Vec<&str> = samples.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["Dot", "Circle", "Rect"]);
        assert_eq!(samples[0].value, json!("Dot"));
        assert!(samples[1].value["Circle"].is_u64());
        assert!(samples[2].value["Rect"]["wide"].is_u64());
        assert!(samples[2].value["Rect"]["high"].is_u64());
    }
}
