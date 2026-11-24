//! Runs the checks on this repository, for every shell that has rules in `rules/`. This makes
//! `cargo test` fail on the machine of the developer and in CI when a shell does not go through
//! the core (ADR 064).

use std::fs;
use std::path::{Path, PathBuf};

/// The rules that are no shell of their own: they hold for every shell.
const CORE: &str = "core";

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The shells that have rules: the names of the files in `rules/`, without `core.toml`.
fn shells(rules: &Path) -> Vec<String> {
    let mut shells: Vec<String> = fs::read_dir(rules)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .filter_map(|path| Some(path.file_stem()?.to_str()?.to_string()))
        .filter(|name| name != CORE)
        .collect();
    shells.sort();
    shells
}

// ルールのあるどのシェルでも、S2は何も見つけない。見つけたら、シェルでその種類を扱うか、そのシェルのルールの除外の一覧に理由を書く。
#[test]
fn s2_finds_nothing_in_any_shell_with_rules() {
    let root = manifest().join("../../..");
    let rules = manifest().join("rules");
    let shells = shells(&rules);
    assert!(
        !shells.is_empty(),
        "no rules of a shell in {}",
        rules.display()
    );

    let mut found = Vec::new();
    for shell in &shells {
        let report = shell_check::run_s2(&root, &rules, shell).unwrap();
        found.extend(
            report
                .missing
                .iter()
                .map(|k| format!("{shell}: missing: {k}")),
        );
        found.extend(
            report
                .unknown_exclusions
                .iter()
                .map(|k| format!("{shell}: unknown exclusion: {k}")),
        );
    }

    assert!(
        found.is_empty(),
        "S2 found kinds of `Event` (ADR 064):\n{}\n\
         Name a missing kind in the shell, or exclude it in rules/<shell>.toml with its reason. \
         Remove an unknown exclusion. See client/tools/shell-check/README.md.",
        found.join("\n")
    );
}
