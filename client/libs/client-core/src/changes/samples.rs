//! Sample values of the types that cross the border, for the examples of D2 (ADR 090): each
//! shell reads them with its generated types and writes them back as they were.
//!
//! The samples are made from the registry of facet_generate, the description of the types
//! that the generators of the shells read. So they hold what the generated types know, and a
//! new type or variant comes into the samples by itself.

use facet_generate::reflection::format::{
    ContainerFormat, Format, Named, QualifiedTypeName, VariantFormat,
};
use facet_generate::Registry;
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashMap};

/// A sample of a type, as the JSON of facet, with the name that the shells know it by.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub id: String,
    pub value: Value,
}

/// The samples of the type named `root` in `registry`.
///
/// An enum has a sample of each of its variants, named by it. Any other type has a sample
/// named by the type, and a sample `empty` with no optional value and empty lists. More
/// samples follow while they hold a variant of an enum inside that no sample held yet.
///
/// A list holds a sample of each variant of its enum, and any other list one value. An
/// option holds a value, and the values of a kind take turns: the samples hold the edges of
/// the numbers, and text that is not ASCII.
pub fn samples(registry: &Registry, root: &str) -> Vec<Sample> {
    Sampler::new(registry).samples(root)
}

/// The variants of the enums reachable from `root` that no sample of `samples` holds, as the
/// name of the enum and the name of the variant.
pub fn uncovered(registry: &Registry, root: &str) -> Vec<(String, String)> {
    let mut sampler = Sampler::new(registry);
    sampler.samples(root);
    let mut reachable = BTreeSet::new();
    reach(
        registry,
        &named(registry, root),
        &mut BTreeSet::new(),
        &mut reachable,
    );
    reachable.difference(&sampler.covered).cloned().collect()
}

/// This many types down, the samples hold no optional value, empty lists and maps, and the
/// variants of an enum that hold nothing, so that a type that holds itself ends. Only the
/// types count: every type that holds itself does so through a type.
const DEEPEST: usize = 8;

/// The most turns of samples that are made to hold every variant inside. A variant behind
/// other enums that take turns comes up only every few turns.
const TURNS: usize = 64;

/// The numbers at the edges that the shells may not hold as they are, as a JavaScript number
/// beyond 2^53. Each kind takes the next in turn.
const U64S: &[u64] = &[u64::MAX, (1 << 53) + 1, 0, 1];
const I64S: &[i64] = &[i64::MIN, -1, i64::MAX];
const U32S: &[u32] = &[u32::MAX, 0];
const I32S: &[i32] = &[i32::MIN, 7];
const U8S: &[u8] = &[u8::MAX, 0];
const F64S: &[f64] = &[0.1, -2.5, f64::MAX, 5e-324];
const F32S: &[f32] = &[0.5, -1.25, f32::MAX, 1e-7];
/// Text that a shell may change when it reads it: letters beyond ASCII, a letter and its
/// accent apart (not NFC), letters beyond the 16 bits of UTF-16, and what JSON escapes.
const STRS: &[&str] = &[
    "日本語の文",
    "e\u{301}",
    "😀👍🏽",
    "\"quoted\" \\ back\nline\ttab",
    "",
];

struct Sampler<'r> {
    registry: &'r Registry,
    /// The next value of each kind, and the next variant of each enum.
    turns: HashMap<String, usize>,
    /// The variants that a sample held, as the name of the enum and of the variant.
    covered: BTreeSet<(String, String)>,
}

impl<'r> Sampler<'r> {
    fn new(registry: &'r Registry) -> Self {
        Sampler {
            registry,
            turns: HashMap::new(),
            covered: BTreeSet::new(),
        }
    }

    fn samples(&mut self, root: &str) -> Vec<Sample> {
        let name = named(self.registry, root);
        let mut reachable = BTreeSet::new();
        reach(self.registry, &name, &mut BTreeSet::new(), &mut reachable);
        match &self.registry[&name] {
            ContainerFormat::Enum(variants, _, _) => {
                let mut samples: Vec<Sample> = variants
                    .values()
                    .map(|variant| Sample {
                        id: variant.name.clone(),
                        value: self.variant(&name, variant, 0),
                    })
                    .collect();
                for turn in 2..=TURNS {
                    if self.covered.is_superset(&reachable) {
                        break;
                    }
                    for variant in variants.values() {
                        let held = self.covered.len();
                        let value = self.variant(&name, variant, 0);
                        if self.covered.len() > held {
                            let id = format!("{}-{turn}", variant.name);
                            samples.push(Sample { id, value });
                        }
                    }
                }
                samples
            }
            _ => {
                let mut samples = vec![Sample {
                    id: root.to_string(),
                    value: self.named(&name, 0),
                }];
                for turn in 2..=TURNS {
                    if self.covered.is_superset(&reachable) {
                        break;
                    }
                    let held = self.covered.len();
                    let value = self.named(&name, 0);
                    if self.covered.len() > held {
                        let id = format!("{root}-{turn}");
                        samples.push(Sample { id, value });
                    }
                }
                let value = self.named(&name, DEEPEST);
                samples.push(Sample {
                    id: "empty".to_string(),
                    value,
                });
                samples
            }
        }
    }

    fn value(&mut self, format: &Format, depth: usize) -> Value {
        let deep = depth >= DEEPEST;
        match format {
            Format::TypeName(name) => self.named(name, depth + 1),
            Format::Unit => Value::Null,
            Format::Bool => json!(self.turn("bool", &[true, false])),
            Format::U8 => json!(self.turn("u8", U8S)),
            Format::U32 => json!(self.turn("u32", U32S)),
            Format::U64 => json!(self.turn("u64", U64S)),
            Format::I32 => json!(self.turn("i32", I32S)),
            Format::I64 => json!(self.turn("i64", I64S)),
            Format::F32 => json!(self.turn("f32", F32S)),
            Format::F64 => json!(self.turn("f64", F64S)),
            Format::Str => json!(self.turn("str", STRS)),
            Format::Option(_) if deep => Value::Null,
            Format::Option(inner) => self.value(inner, depth),
            Format::Seq(_) | Format::Set(_) if deep => json!([]),
            Format::Seq(inner) | Format::Set(inner) => Value::Array(self.elements(inner, depth)),
            Format::Map { .. } if deep => json!({}),
            Format::Map { key, value } => {
                let key = match self.value(key, depth) {
                    Value::String(key) => key,
                    key => key.to_string(),
                };
                let value = self.value(value, depth);
                Value::Object(Map::from_iter([(key, value)]))
            }
            Format::Tuple(formats) => self.all(formats, depth),
            Format::TupleArray { content, size } => {
                Value::Array((0..*size).map(|_| self.value(content, depth)).collect())
            }
            // None of these crosses the border now. A sample of one is to be made when it does
            other => panic!("no sample is made for the format {other:?}"),
        }
    }

    /// The elements of a list of `inner`: a sample of each variant of an enum, or one value.
    fn elements(&mut self, inner: &Format, depth: usize) -> Vec<Value> {
        if let Format::TypeName(name) = inner {
            if let ContainerFormat::Enum(variants, _, _) = &self.registry[name] {
                return variants
                    .values()
                    .map(|variant| self.variant(name, variant, depth + 1))
                    .collect();
            }
        }
        vec![self.value(inner, depth)]
    }

    fn named(&mut self, name: &QualifiedTypeName, depth: usize) -> Value {
        assert!(
            depth < DEEPEST * 4,
            "the samples of {} do not end: it holds itself with no option or list",
            name.name
        );
        match &self.registry[name] {
            ContainerFormat::UnitStruct(_) => Value::Null,
            ContainerFormat::NewTypeStruct(format, _) => self.value(format, depth),
            ContainerFormat::TupleStruct(formats, _) => self.all(formats, depth),
            ContainerFormat::Struct(fields, _) => self.fields(fields, depth),
            ContainerFormat::Enum(variants, _, _) => {
                let variants: Vec<&Named<VariantFormat>> = variants.values().collect();
                // Deep down, only the variants that hold nothing, so that the sample ends, or all
                // of them when each holds something: there, what they hold is empty. They take
                // turns too: an enum may be reached only that deep
                let choices: Vec<usize> = (0..variants.len())
                    .filter(|&index| {
                        depth < DEEPEST || matches!(variants[index].value, VariantFormat::Unit)
                    })
                    .collect();
                let choices = if choices.is_empty() {
                    (0..variants.len()).collect()
                } else {
                    choices
                };
                let turn = self.turns.entry(name.to_string()).or_default();
                *turn += 1;
                let chosen = choices[(*turn - 1) % choices.len()];
                self.variant(name, variants[chosen], depth)
            }
        }
    }

    /// The variant as the JSON of facet: its name alone, or an object of its name and what
    /// it holds.
    fn variant(
        &mut self,
        name: &QualifiedTypeName,
        variant: &Named<VariantFormat>,
        depth: usize,
    ) -> Value {
        self.covered
            .insert((name.name.clone(), variant.name.clone()));
        let held = match &variant.value {
            VariantFormat::Unit => return json!(variant.name),
            VariantFormat::NewType(format) => self.value(format, depth),
            VariantFormat::Tuple(formats) => self.all(formats, depth),
            VariantFormat::Struct(fields) => self.fields(fields, depth),
            VariantFormat::Variable(_) => unreachable!("the registry is built"),
        };
        Value::Object(Map::from_iter([(variant.name.clone(), held)]))
    }

    fn fields(&mut self, fields: &[Named<Format>], depth: usize) -> Value {
        Value::Object(
            fields
                .iter()
                .map(|field| (field.name.clone(), self.value(&field.value, depth)))
                .collect(),
        )
    }

    fn all(&mut self, formats: &[Format], depth: usize) -> Value {
        Value::Array(
            formats
                .iter()
                .map(|format| self.value(format, depth))
                .collect(),
        )
    }

    /// The next of `values` for the kind `kind`.
    fn turn<T: Copy>(&mut self, kind: &str, values: &[T]) -> T {
        let turn = self.turns.entry(kind.to_string()).or_default();
        let value = values[*turn % values.len()];
        *turn += 1;
        value
    }
}

/// The name of the type `root` in `registry`.
fn named(registry: &Registry, root: &str) -> QualifiedTypeName {
    registry
        .keys()
        .find(|name| name.name == root)
        .unwrap_or_else(|| panic!("{root} is not in the registry"))
        .clone()
}

/// Adds the variants of the enums reachable from `name` to `reachable`.
fn reach(
    registry: &Registry,
    name: &QualifiedTypeName,
    seen: &mut BTreeSet<QualifiedTypeName>,
    reachable: &mut BTreeSet<(String, String)>,
) {
    if !seen.insert(name.clone()) {
        return;
    }
    let mut formats: Vec<&Format> = Vec::new();
    match &registry[name] {
        ContainerFormat::UnitStruct(_) => {}
        ContainerFormat::NewTypeStruct(format, _) => formats.push(format),
        ContainerFormat::TupleStruct(all, _) => formats.extend(all),
        ContainerFormat::Struct(fields, _) => formats.extend(fields.iter().map(|f| &f.value)),
        ContainerFormat::Enum(variants, _, _) => {
            for variant in variants.values() {
                reachable.insert((name.name.clone(), variant.name.clone()));
                match &variant.value {
                    VariantFormat::NewType(format) => formats.push(format),
                    VariantFormat::Tuple(all) => formats.extend(all),
                    VariantFormat::Struct(fields) => {
                        formats.extend(fields.iter().map(|f| &f.value))
                    }
                    VariantFormat::Unit | VariantFormat::Variable(_) => {}
                }
            }
        }
    }
    for format in formats {
        reach_format(registry, format, seen, reachable);
    }
}

fn reach_format(
    registry: &Registry,
    format: &Format,
    seen: &mut BTreeSet<QualifiedTypeName>,
    reachable: &mut BTreeSet<(String, String)>,
) {
    match format {
        Format::TypeName(name) => reach(registry, name, seen, reachable),
        Format::Option(inner) | Format::Seq(inner) | Format::Set(inner) => {
            reach_format(registry, inner, seen, reachable)
        }
        Format::TupleArray { content, .. } => reach_format(registry, content, seen, reachable),
        Format::Map { key, value } => {
            reach_format(registry, key, seen, reachable);
            reach_format(registry, value, seen, reachable);
        }
        Format::Tuple(all) => {
            for format in all {
                reach_format(registry, format, seen, reachable);
            }
        }
        _ => {}
    }
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

    #[derive(Facet)]
    #[allow(dead_code)]
    struct Drawing {
        title: String,
        main: Shape,
        note: Option<String>,
        shapes: Vec<Shape>,
        sizes: Vec<u64>,
    }

    // 構造体の型には、型の名前の見本と、値のない`empty`の見本がある。`empty`では、省ける値はなく、並びは空である。
    #[test]
    fn a_struct_has_a_sample_named_by_it_and_an_empty_one() {
        let samples = samples(&registry::<Drawing>(), "Drawing");

        assert_eq!(samples[0].id, "Drawing");
        assert!(samples[0].value["note"].is_string());
        assert_eq!(samples[0].value["sizes"].as_array().unwrap().len(), 1);
        let empty = samples.last().unwrap();
        assert_eq!(empty.id, "empty");
        assert_eq!(empty.value["note"], Value::Null);
        assert_eq!(empty.value["shapes"], json!([]));
        assert_eq!(
            empty.value["main"],
            json!("Dot"),
            "an enum that holds nothing"
        );
    }

    // 並びの列挙型は、変種ごとに1つの値を持つ。
    #[test]
    fn a_list_of_an_enum_holds_each_variant() {
        let samples = samples(&registry::<Drawing>(), "Drawing");

        let shapes = samples[0].value["shapes"].as_array().unwrap();
        assert_eq!(shapes.len(), 3);
        assert_eq!(shapes[0], json!("Dot"));
        assert!(shapes[1].get("Circle").is_some());
        assert!(shapes[2].get("Rect").is_some());
    }

    #[derive(Facet)]
    #[allow(dead_code)]
    struct Card {
        shape: Shape,
    }

    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Deck {
        One(Card),
        None,
    }

    // 1つの欄にしか出ない列挙型も、見本を足して、すべての変種を持つ。足した見本の名前は、変種の名前と番号である。
    #[test]
    fn samples_follow_until_every_variant_inside_is_held() {
        let registry = registry::<Deck>();

        let samples = samples(&registry, "Deck");

        let ids: Vec<&str> = samples.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["One", "None", "One-2", "One-3"]);
        assert_eq!(uncovered(&registry, "Deck"), Vec::<(String, String)>::new());
    }

    // 見本のない変種は、列挙型の名前と変種の名前で示される。
    #[test]
    fn the_variants_that_no_sample_holds_are_told() {
        let registry = registry::<Deck>();
        let mut sampler = Sampler::new(&registry);
        sampler.variant(
            &named(&registry, "Deck"),
            &Named::new(&VariantFormat::Unit, "None".to_string()),
            0,
        );

        let mut reachable = BTreeSet::new();
        reach(
            &registry,
            &named(&registry, "Deck"),
            &mut BTreeSet::new(),
            &mut reachable,
        );
        let left: Vec<_> = reachable.difference(&sampler.covered).cloned().collect();

        assert!(left.contains(&("Shape".to_string(), "Rect".to_string())));
        assert!(left.contains(&("Deck".to_string(), "One".to_string())));
        assert!(!left.contains(&("Deck".to_string(), "None".to_string())));
    }

    #[derive(Facet)]
    #[allow(dead_code)]
    struct Tree {
        name: String,
        children: Vec<Tree>,
        parent: Option<Box<Tree>>,
    }

    // 自分を持つ型の見本も、深くなると省ける値と並びが空になって終わる。
    #[test]
    fn the_sample_of_a_type_that_holds_itself_ends() {
        let samples = samples(&registry::<Tree>(), "Tree");

        let mut depth = 0;
        let mut tree = &samples[0].value;
        while let Some(child) = tree["children"].as_array().and_then(|c| c.first()) {
            tree = child;
            depth += 1;
        }
        assert!(depth > 1 && depth <= DEEPEST, "{depth}");
    }

    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Mark {
        Plain,
        Bold,
        Held(u32),
        Italic,
    }

    /// A type that holds `Mark` only below `DEEPEST`, eight types down.
    macro_rules! chain {
        ($name:ident, $field:ty) => {
            #[derive(Facet)]
            #[allow(dead_code)]
            struct $name {
                next: $field,
            }
        };
    }
    chain!(Deep0, Deep1);
    chain!(Deep1, Deep2);
    chain!(Deep2, Deep3);
    chain!(Deep3, Deep4);
    chain!(Deep4, Deep5);
    chain!(Deep5, Deep6);
    chain!(Deep6, Deep7);
    chain!(Deep7, Mark);

    // 深いところでしか届かない列挙型も、何も持たない変種を順に使い、そのすべてが見本に出る。
    #[test]
    fn an_enum_reached_only_deep_takes_turns_among_its_variants_that_hold_nothing() {
        let registry = registry::<Deep0>();

        assert_eq!(
            uncovered(&registry, "Deep0"),
            [("Mark".to_string(), "Held".to_string())]
        );
    }

    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Link {
        Page(String),
        Web(String),
        File(String),
    }

    chain!(Far0, Far1);
    chain!(Far1, Far2);
    chain!(Far2, Far3);
    chain!(Far3, Far4);
    chain!(Far4, Far5);
    chain!(Far5, Far6);
    chain!(Far6, Far7);
    chain!(Far7, Link);

    // 深いところでしか届かない列挙型のすべての変種が何かを持つなら、そのすべてを順に使う。
    #[test]
    fn an_enum_reached_only_deep_whose_variants_all_hold_something_takes_turns_among_them_all() {
        let registry = registry::<Far0>();

        assert_eq!(uncovered(&registry, "Far0"), Vec::<(String, String)>::new());
    }

    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum OnlyA {
        V,
    }
    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum OnlyB {
        V,
    }
    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum OnlyC {
        V,
    }
    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum OnlyD {
        V,
    }
    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum OnlyE {
        V,
    }

    /// Holds an enum in each kind of format that holds another, eight types down.
    #[derive(Facet)]
    #[allow(dead_code)]
    struct Holder {
        maybe: Option<OnlyA>,
        list: Vec<OnlyB>,
        map: std::collections::BTreeMap<String, OnlyC>,
        pair: (Option<OnlyD>, u8),
        row: [Option<OnlyE>; 1],
    }

    chain!(Hold0, Hold1);
    chain!(Hold1, Hold2);
    chain!(Hold2, Hold3);
    chain!(Hold3, Hold4);
    chain!(Hold4, Hold5);
    chain!(Hold5, Hold6);
    chain!(Hold6, Hold7);
    chain!(Hold7, Holder);

    /// The variants of `Holder` that no sample can hold: deep down, the options are empty.
    fn held_by_nothing() -> Vec<(String, String)> {
        ["OnlyA", "OnlyB", "OnlyC", "OnlyD", "OnlyE"]
            .map(|name| (name.to_string(), "V".to_string()))
            .to_vec()
    }

    // 選べる値、並び、対応表、組、配列の中の列挙型も、届くものとして数える。深いところで空になれば、見本のない変種として示される。
    #[test]
    fn an_enum_inside_any_format_is_reachable_and_told_when_no_sample_holds_it() {
        let registry = registry::<Hold0>();

        assert_eq!(uncovered(&registry, "Hold0"), held_by_nothing());
    }

    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Pick {
        A,
        B,
    }

    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Lone {
        V,
    }

    #[derive(Facet)]
    #[allow(dead_code)]
    struct Partly {
        pick: Pick,
        maybe: Option<Lone>,
        far: Hold0,
    }

    // 構造体の型の見本は、新しい変種を持つときだけ足す。持てない変種が残っても、それで止まる。浅いところの選べる値は、値を持つ。
    #[test]
    fn a_struct_gets_more_samples_only_while_they_hold_new_variants() {
        let registry = registry::<Partly>();

        let samples = samples(&registry, "Partly");

        let ids: Vec<&str> = samples.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["Partly", "Partly-2", "empty"]);
        assert_eq!(samples[0].value["maybe"], json!("V"));
        assert_eq!(uncovered(&registry, "Partly"), held_by_nothing());
    }

    #[derive(Facet)]
    #[allow(dead_code)]
    struct Pair(u8, bool);

    // 組の構造体の見本は、持つ値の並びである。
    #[test]
    fn a_tuple_struct_is_the_list_of_what_it_holds() {
        let samples = samples(&registry::<Pair>(), "Pair");

        assert_eq!(samples[0].value, json!([u8::MAX, true]));
    }

    #[derive(Facet)]
    #[allow(dead_code)]
    struct Turns {
        i: [i64; 3],
        f: [f64; 4],
        g: [f32; 4],
    }

    // 数の種類ごとに、端の値を順にすべて使う。
    #[test]
    fn each_kind_of_number_takes_all_its_values_in_turn() {
        let samples = samples(&registry::<Turns>(), "Turns");

        assert_eq!(samples[0].value["i"], json!(I64S));
        assert_eq!(samples[0].value["i"][1], json!(-1));
        assert_eq!(samples[0].value["f"], json!(F64S));
        assert_eq!(samples[0].value["f"][1], json!(-2.5));
        assert_eq!(samples[0].value["g"], json!(F32S));
        assert_eq!(samples[0].value["g"][1], json!(-1.25));
    }

    #[derive(Facet)]
    #[allow(dead_code)]
    struct Numbers {
        a: u64,
        b: u64,
        c: i64,
        d: f64,
        e: String,
        f: String,
    }

    // 数は、シェルがそのまま持てないかもしれない端の値を、順に使う。文字は、ASCIIの外の文字を使う。
    #[test]
    fn the_samples_use_the_edges_of_the_numbers_and_text_beyond_ascii() {
        let samples = samples(&registry::<Numbers>(), "Numbers");

        let sample = &samples[0].value;
        assert_eq!(sample["a"], json!(u64::MAX));
        assert_eq!(sample["b"], json!((1u64 << 53) + 1));
        assert_eq!(sample["c"], json!(i64::MIN));
        assert_eq!(sample["d"], json!(0.1));
        assert_eq!(sample["e"], json!("日本語の文"));
        assert_eq!(sample["f"], json!("e\u{301}"));
    }
}
