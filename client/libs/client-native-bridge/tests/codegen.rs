//! Runs the program that writes the types of the shells. It only exists with the feature
//! `facet_typegen`: `cargo test -p client-native-bridge --features facet_typegen`.
#![cfg(feature = "facet_typegen")]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs the program for a new folder of this test, and gives the folder.
fn generated(test: &str) -> PathBuf {
    generated_for("swift", test)
}

/// Runs the program for the language and a new folder of this test, and gives the folder.
fn generated_for(language: &str, test: &str) -> PathBuf {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    let _ = std::fs::remove_dir_all(&out);
    let status = Command::new(env!("CARGO_BIN_EXE_codegen"))
        .arg(language)
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    out
}

/// The Swift source with the types of the border.
fn swift_types(out: &Path) -> String {
    let file = out.join("swift/PetaTypes/Sources/PetaTypes/PetaTypes.swift");
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()))
}

// Swiftの型が生成される。シェルが送るイベントとキー、描くビューモデル、シェルへの依頼とその答えが入る。
#[test]
fn the_swift_types_of_the_border_are_written() {
    let swift = swift_types(&generated("types"));

    for declared in [
        "enum Event",
        "struct ViewModel",
        "enum ShellEffect",
        "struct Requests",
        "enum UiOutput",
        "enum KeyPress",
        "enum DragMove",
        "enum Typing",
    ] {
        assert!(swift.contains(&format!("public {declared}:")), "{declared}");
    }
}

// ビューモデルが`Box`の中に持つ型も、生成される。生成の道具は`Box`の中をたどらないので、漏れるとSwiftがコンパイルできない。
#[test]
fn a_type_the_view_model_keeps_in_a_box_is_written() {
    let swift = swift_types(&generated("boxed"));

    assert!(swift.contains("case page(PageView?)"));
    assert!(swift.contains("public struct PageView:"));
}

// 生成されたSwiftに、タプルはない。Swiftのタプルは比べられないので、それを持つ型がコンパイルできなくなる。
#[test]
fn no_tuple_is_written() {
    let swift = swift_types(&generated("tuples"));

    let tuples: Vec<&str> = swift
        .lines()
        .filter(|line| line.contains("public var") && line.contains(": [("))
        .collect();
    assert!(tuples.is_empty(), "{tuples:?}");
}

/// The C# source with the types of the border.
fn csharp_types(out: &Path) -> String {
    let file = out.join("csharp/PetaTypes/PetaTypes.cs");
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()))
}

// C#の型が生成される。シェルが送るイベントとキー、描くビューモデル、シェルへの依頼とその答えが入る。
#[test]
fn the_csharp_types_of_the_border_are_written() {
    let csharp = csharp_types(&generated_for("csharp", "csharp-types"));

    for declared in [
        "abstract record Event",
        "partial class ViewModel",
        "closed record ShellEffect",
        "partial class Requests",
        "abstract record UiOutput",
        "partial class PageView",
        "abstract record KeyPress",
        "abstract record DragMove",
        "abstract record Typing",
    ] {
        assert!(
            csharp.contains(&format!("public {declared} ")),
            "{declared}"
        );
    }
}

// 生成されたC#で、変種の名前は、その変種が持つ型の名前を隠さない。隠すとC#がコンパイルできない（facet_generate 0.19の不具合）。
#[test]
fn a_csharp_variant_does_not_hide_the_type_it_holds() {
    let csharp = csharp_types(&generated_for("csharp", "csharp-variants"));

    assert!(csharp.contains("record SelectionKey(global::PetaTypes.SelectionKey Value) : Event;"));
}

// 生成されたC#は、フィールドのないenumの`Option`を、値型として読む。参照型として読むとC#がコンパイルできない（facet_generate 0.19の不具合）。
#[test]
fn csharp_reads_an_optional_plain_enum_as_a_value() {
    let csharp = csharp_types(&generated_for("csharp", "csharp-options"));

    assert!(csharp.contains(
        "FacetHelpers.DeserializeOption(deserializer, d => TaskStateBincode.Deserialize(d))"
    ));
}

/// The Kotlin source with the types of the border.
fn kotlin_types(out: &Path) -> String {
    let file = out.join("kotlin/com/sosuisha/petajournal/types/Types.kt");
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()))
}

// Kotlinの型が生成される。シェルが送るイベントとキー、描くビューモデル、シェルへの依頼とその答えが入る。
#[test]
fn the_kotlin_types_of_the_border_are_written() {
    let kotlin = kotlin_types(&generated_for("kotlin", "kotlin-types"));

    for declared in [
        "sealed interface Event",
        "data class ViewModel",
        "sealed interface ShellEffect",
        "data class Requests",
        "sealed interface UiOutput",
        "data class PageView",
        "sealed interface KeyPress",
        "sealed interface DragMove",
        "sealed interface Typing",
    ] {
        assert!(
            kotlin.contains(&format!(
                "
{declared}"
            )),
            "{declared}"
        );
    }
}

// 生成されたKotlinで、変種の名前は、Kotlinのリストを隠さない。隠すとKotlinがコンパイルできない（facet_generate 0.19の不具合）。
#[test]
fn a_variant_does_not_hide_the_list_of_kotlin() {
    let kotlin = kotlin_types(&generated_for("kotlin", "kotlin-list"));

    assert!(kotlin.contains("val value: kotlin.collections.List<String>"));
}
