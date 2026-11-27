//! The examples of the lines that cross the border of the Emacs shell, from its Rust to its
//! Lisp (ADR 078), for D2 (ADR 089, ADR 090). The Lisp reads each of them in a test of ERT.
//!
//! Only the lines to Emacs have examples. The lines from Emacs are written in Lisp by hand,
//! from keys that nothing generates: there is nothing of the Rust types to hold them against.

use crate::emacs_text::EmacsText;
use crate::protocol::{
    write_line, BacklinkEntry, BacklinkLine, Bullet, FreeKey, Outbound, PropertyRow, RowView,
    SearchEntry, SearchKind, SidebarHeading, SidebarIndex, SidebarLine, SidebarView, TaskMark,
};
use client_core::changes::examples::is_current;
use client_core::changes::ListChange;
use client_core::BodyModel;
use facet_generate::reflection::format::{ContainerFormat, EnumTagging, Format, VariantFormat};
use facet_generate::reflection::RegistryBuilder;
use facet_generate::Registry;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// An example of a line to Emacs, with the name that the tests know it by.
struct Example {
    id: &'static str,
    line: Outbound,
}

/// Text that is hard to carry: what JSON escapes, letters beyond ASCII, a letter and its
/// accent apart, and letters beyond the 16 bits of UTF-16.
const HARD: &str = "\"quoted\" \\ back\nline\ttab 日本語 e\u{301} 😀👍🏽";

/// A body with every look of a shown text, every kind of link and an image.
const EVERY_LOOK: &str = "# 一\n\n## 二\n\n### 三\n\n#### 四\n\n##### 五\n\n###### 六\n\n\
    **bold** *italic* ~~struck~~ __underlined__ ==marked== `code` x^raised^ x~lowered~ \
    ||hidden|| [[page]] [web](https://example.com/) [file](assets/sample.pdf)\n\n\
    ![image](assets/sample.png)\n\n- entry\n\n> quoted\n\n---\n\n```\nlet a;\n```\n\n\
    | name | value |\n|---|---|\n| a | b |";

/// The row of the item `id` with the Markdown `body`, as a plain item.
fn row(id: &str, body: &str) -> RowView {
    RowView {
        id: id.into(),
        level: 1,
        body: body.into(),
        text: EmacsText::of_markdown(body, "tags/"),
        bullet: Bullet::Plain,
        task: None,
        collapsed: false,
        header: None,
        selected: false,
    }
}

/// A line of a list of the sidebar.
fn sidebar_line(label: &str, dir: Option<&str>, folder: bool, current: bool) -> SidebarLine {
    SidebarLine {
        label: label.into(),
        dir: dir.map(String::from),
        path: format!("{}{label}", dir.unwrap_or_default()),
        folder,
        current,
    }
}

/// The examples of the lines to Emacs: each kind of line, and a second one of a kind that
/// can go without a value. The changes of `Rows` can be applied in order to any rows, as they
/// begin with a reset.
fn examples() -> Vec<Example> {
    let example = |id, line| Example { id, line };
    vec![
        example(
            "State",
            Outbound::State {
                ack: 3,
                holding: true,
            },
        ),
        example(
            "Rows",
            Outbound::Rows {
                ack: 4,
                page_id: Some("page-2".into()),
                changes: vec![
                    ListChange::Reset {
                        items: vec![
                            row("a", EVERY_LOOK),
                            RowView {
                                level: 2,
                                bullet: Bullet::Transclusion,
                                task: Some(TaskMark::Task),
                                collapsed: true,
                                ..row("b", HARD)
                            },
                            RowView {
                                bullet: Bullet::Broken,
                                task: Some(TaskMark::Running),
                                header: Some("2026-11-27 Fri".into()),
                                ..row("c", "三つ目")
                            },
                        ],
                    },
                    ListChange::Insert {
                        at: 1,
                        item: RowView {
                            bullet: Bullet::Hidden,
                            ..row("d", "")
                        },
                    },
                    ListChange::Update {
                        at: 3,
                        item: RowView {
                            task: Some(TaskMark::Done),
                            selected: true,
                            ..row("c", "変えた")
                        },
                    },
                    ListChange::Move { from: 0, to: 3 },
                    ListChange::Remove { at: 1 },
                ],
            },
        ),
        example(
            "Rows-2",
            Outbound::Rows {
                ack: 0,
                page_id: None,
                changes: vec![],
            },
        ),
        example(
            "Focus",
            Outbound::Focus {
                ack: 5,
                id: "a".into(),
                cursor: Some(1),
            },
        ),
        example(
            "Focus-2",
            Outbound::Focus {
                ack: 5,
                id: "a".into(),
                cursor: None,
            },
        ),
        example("Scroll", Outbound::Scroll { id: "a".into() }),
        example("Title", Outbound::Title { title: HARD.into() }),
        example(
            "PagePath",
            Outbound::PagePath {
                path: Some("仕事/メモ".into()),
            },
        ),
        example("PagePath-2", Outbound::PagePath { path: None }),
        example(
            "Index",
            Outbound::Index {
                folder: Some("仕事/メモ".into()),
                demote_name: Some("メモ".into()),
            },
        ),
        example(
            "Index-2",
            Outbound::Index {
                folder: None,
                demote_name: None,
            },
        ),
        example(
            "Properties",
            Outbound::Properties {
                rows: Some(vec![
                    PropertyRow {
                        key: "rating".into(),
                        label: "rating (number)".into(),
                        value: "5".into(),
                        removable: true,
                        editable: true,
                    },
                    PropertyRow {
                        key: "created".into(),
                        label: "created (datetime)".into(),
                        value: "2026-11-27T01:00:00Z".into(),
                        removable: false,
                        editable: false,
                    },
                ]),
                free_keys: vec![FreeKey {
                    key: "due".into(),
                    label: "due (datetime)".into(),
                }],
                datetime_keys: vec!["created".into(), "due".into()],
            },
        ),
        example(
            "Properties-2",
            Outbound::Properties {
                rows: None,
                free_keys: vec![],
                datetime_keys: vec![],
            },
        ),
        example(
            "Status",
            Outbound::Status {
                connection: "connected".into(),
            },
        ),
        example(
            "Search",
            Outbound::Search {
                results: [
                    SearchKind::Today,
                    SearchKind::Page,
                    SearchKind::Create,
                    SearchKind::Item,
                ]
                .into_iter()
                .enumerate()
                .map(|(position, kind)| SearchEntry {
                    position,
                    kind,
                    title: "仕事/メモ".into(),
                    label: "メモ".into(),
                    snippet: HARD.into(),
                })
                .collect(),
                searching: true,
            },
        ),
        example(
            "Recent",
            Outbound::Recent {
                paths: vec!["仕事/メモ".into(), "sample".into()],
            },
        ),
        example(
            "Sidebar",
            Outbound::Sidebar(SidebarView {
                index: Some(SidebarIndex {
                    header: "仕事/".into(),
                    path: "仕事/index".into(),
                    parent: Some("index".into()),
                    pages: vec![
                        sidebar_line("メモ", None, false, true),
                        sidebar_line("資料", None, true, false),
                    ],
                }),
                headings: vec![SidebarHeading {
                    id: "a".into(),
                    text: "一".into(),
                    level: 1,
                }],
                recent: vec![sidebar_line("メモ", Some("仕事/"), false, false)],
            }),
        ),
        example("Sidebar-2", Outbound::Sidebar(SidebarView::default())),
        example(
            "Backlinks",
            Outbound::Backlinks {
                groups: vec![BacklinkEntry {
                    title: "仕事/メモ".into(),
                    url: "/page/仕事/メモ".into(),
                    items: vec![BacklinkLine {
                        text: HARD.into(),
                        url: "/page/仕事/メモ#a".into(),
                    }],
                }],
            },
        ),
        example("Message", Outbound::Message { text: HARD.into() }),
        example("Copy", Outbound::Copy { text: HARD.into() }),
        example(
            "Browse",
            Outbound::Browse {
                address: "/assets/sample%20a.pdf".into(),
            },
        ),
        example(
            "PageNames",
            Outbound::PageNames {
                query: "メ".into(),
                titles: vec!["仕事/メモ".into()],
            },
        ),
        example(
            "Ask",
            Outbound::Ask {
                req: 7,
                message: "Delete the page?".into(),
            },
        ),
    ]
}

/// The file of the examples of the lines to Emacs, in the repository (ADR 090 D2).
fn border_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tools/shell-check/common/border-emacs.json")
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

// 境界の例のファイルの`outbound`の節は、今の例の行と同じである。`PETA_WRITE_EXAMPLES=1`を付けて回すと、書き直す。
#[test]
fn the_examples_of_the_border_are_current() {
    let lines: Vec<Value> = examples()
        .iter()
        // The text itself, not a JSON value of it: Emacs reads the line as the shell writes it
        .map(|example| json!({ "id": example.id, "line": write_line(&example.line) }))
        .collect();

    is_current(&border_file(), "outbound", Value::Array(lines)).unwrap();
}
