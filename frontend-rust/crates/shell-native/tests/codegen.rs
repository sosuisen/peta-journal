//! Runs the program that writes the types of the shells. It only exists with the feature
//! `facet_typegen`: `cargo test -p peta-shell-native --features facet_typegen`.
#![cfg(feature = "facet_typegen")]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs the program for a new folder of this test, and gives the folder.
fn generated(test: &str) -> PathBuf {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    let _ = std::fs::remove_dir_all(&out);
    let status = Command::new(env!("CARGO_BIN_EXE_codegen"))
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

// Swiftの型が生成される。シェルが送るイベント、描くビューモデル、シェルへの依頼とその答えが入る。
#[test]
fn the_swift_types_of_the_border_are_written() {
    let swift = swift_types(&generated("types"));

    for declared in [
        "enum Event",
        "struct ViewModel",
        "enum ShellEffect",
        "struct Requests",
        "enum UiOutput",
    ] {
        assert!(swift.contains(&format!("public {declared}:")), "{declared}");
    }
}
