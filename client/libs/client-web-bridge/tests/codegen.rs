//! Runs the program that writes the TypeScript types of the border for the web shell.

use std::path::Path;
use std::process::Command;

// フォルダを渡すと、境界の型がTypeScriptとして、そのフォルダに書き出される。
#[test]
fn the_program_writes_the_types_into_the_folder_it_is_given() {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("web-types");
    let _ = std::fs::remove_dir_all(&out);

    let status = Command::new(env!("CARGO_BIN_EXE_codegen"))
        .arg(&out)
        .status()
        .unwrap();

    assert!(status.success());
    let types = std::fs::read_to_string(out.join("peta-types.ts")).unwrap();
    assert!(types.contains("export class Changes"));
}

// フォルダを渡さないと、使い方を示して失敗する。
#[test]
fn the_program_without_a_folder_shows_its_usage_and_fails() {
    let output = Command::new(env!("CARGO_BIN_EXE_codegen"))
        .output()
        .unwrap();

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("usage: codegen <folder>"), "{error}");
}
