//! Runs the program on a small repository in a temporary folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const APP_RS: &str = "pub enum Event {\n    Start,\n    PasteText {\n        text: String,\n    },\n    PageLoaded,\n}\n";

const CORE_TOML: &str = "[s2.excluded]\nPageLoaded = \"answer: the doc page\"\n";

const SHELL_TOML: &str =
    "[s2]\nlanguage = \"swift\"\nsources = [\"app/Sources\"]\n\n[s2.excluded]\n";

/// A repository with the core and one Swift shell, and the rules of the shell as `shell`.
fn repository(name: &str, swift: &str, shell_toml: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("shell-check-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    write(&root.join("client/libs/client-core/src/app.rs"), APP_RS);
    write(&root.join("app/Sources/Shell/Shell.swift"), swift);
    write(&root.join("app/Sources/notes.txt"), ".pasteText");
    write(&root.join("rules/core.toml"), CORE_TOML);
    write(&root.join("rules/shell.toml"), shell_toml);
    root
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn shell_check(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_shell-check"))
        .args(args)
        .output()
        .unwrap()
}

fn run_s2(root: &Path) -> Output {
    shell_check(&[
        "s2",
        "shell",
        "--root",
        root.to_str().unwrap(),
        "--rules",
        root.join("rules").to_str().unwrap(),
    ])
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

// シェルのソースに出てこない種類が出力され、終了コードは1である。サブフォルダのソースも読まれ、拡張子の違うファイルは読まれない。
#[test]
fn prints_the_missing_kinds_and_exits_with_1() {
    let root = repository("missing", "send(.start)\n", SHELL_TOML);
    let output = run_s2(&root);
    assert_eq!(stdout(&output), "missing: PasteText\n");
    assert_eq!(output.status.code(), Some(1));
}

// どの種類も出てくるか除外されていれば、何も出力されず、終了コードは0である。
#[test]
fn prints_nothing_and_exits_with_0_when_every_kind_is_named() {
    let root = repository(
        "clean",
        "send(.start)\nsend(.pasteText(text: t))\n",
        SHELL_TOML,
    );
    let output = run_s2(&root);
    assert_eq!(stdout(&output), "");
    assert_eq!(output.status.code(), Some(0));
}

// `Event`に無い名前が除外リストにあれば、出力される。
#[test]
fn prints_an_exclusion_that_is_no_kind_of_event() {
    let rules = format!("{SHELL_TOML}Removed = \"gone\"\nPasteText = \"no paste\"\n");
    let root = repository("unknown", "send(.start)\n", &rules);
    let output = run_s2(&root);
    assert_eq!(stdout(&output), "unknown exclusion: Removed\n");
    assert_eq!(output.status.code(), Some(1));
}

// ルールが指すソースのフォルダが無ければ、動かずに終了コード2で終わる。
#[test]
fn exits_with_2_when_a_source_folder_is_not_there() {
    let rules = SHELL_TOML.replace("app/Sources", "app/Missing");
    let root = repository("no-folder", "", &rules);
    let output = run_s2(&root);
    assert_eq!(output.status.code(), Some(2));
}

// ルールのファイルがTOMLとして読めなければ、動かずに終了コード2で終わる。
#[test]
fn exits_with_2_when_the_rules_are_no_toml() {
    let root = repository("bad-toml", "", "[s2\n");
    let output = run_s2(&root);
    assert_eq!(output.status.code(), Some(2));
}

// コアの`app.rs`に`pub enum Event`が無ければ、動かずに終了コード2で終わる。
#[test]
fn exits_with_2_when_the_core_has_no_event() {
    let root = repository("no-event", "", SHELL_TOML);
    write(
        &root.join("client/libs/client-core/src/app.rs"),
        "pub enum Route {\n}\n",
    );
    let output = run_s2(&root);
    assert_eq!(output.status.code(), Some(2));
}

// 引数が足りなければ、使い方を出して終了コード2で終わる。
#[test]
fn exits_with_2_and_prints_the_usage_without_a_shell() {
    let output = shell_check(&["s2"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .starts_with("usage:"));
}

// S2でないチェックの名前には、終了コード2で終わる。
#[test]
fn exits_with_2_for_an_unknown_check() {
    let output = shell_check(&["s9", "shell"]);
    assert_eq!(output.status.code(), Some(2));
}

// 知らないオプションや、値の無いオプションには、終了コード2で終わる。
#[test]
fn exits_with_2_for_a_wrong_option() {
    assert_eq!(
        shell_check(&["s2", "shell", "--depth", "1"]).status.code(),
        Some(2)
    );
    assert_eq!(
        shell_check(&["s2", "shell", "--root"]).status.code(),
        Some(2)
    );
}
