//! The check that every shell has the common tests (ADR 089), on small repositories in a
//! temporary folder.

use std::fs;
use std::path::{Path, PathBuf};

use shell_check::common::Problem;

const APP_RS: &str = "pub enum Event {\n    Start,\n}\n";

const CORE_TOML: &str =
    "[s2.excluded]\n\n[common.list-changes]\nexamples = \"list-changes.json\"\n";

/// The rules of a TypeScript shell whose tests are under `app/src`, with `entry` as what it
/// says of the common test `list-changes`.
fn shell_toml(entry: &str) -> String {
    format!(
        "[s2]\nlanguage = \"typescript\"\nsources = [\"app/src\"]\n\n\
         [common]\ntests = [\"app/src\"]\n\n[common.list-changes]\n{entry}\n"
    )
}

/// A repository with the core, the rules of `shell` and the files of `files`, each a path from
/// the root and its text.
fn repository(name: &str, shell: &str, files: &[(&str, &str)]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("shell-check-common-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    write(&root.join("client/libs/client-core/src/app.rs"), APP_RS);
    write(&root.join("rules/core.toml"), CORE_TOML);
    write(&root.join("rules/shell.toml"), shell);
    for (path, text) in files {
        write(&root.join(path), text);
    }
    root
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn problems(root: &Path) -> Vec<Problem> {
    shell_check::run_common(root, &root.join("rules"), "shell").unwrap()
}

const READS_THE_EXAMPLES: &str = "import cases from '../common/list-changes.json';\n";

// 共通のテストのファイルがあり、例のファイルを読み、テストの道具が集める場所にあれば、何も出ない。
#[test]
fn a_shell_with_the_common_test_has_no_problem() {
    let root = repository(
        "good",
        &shell_toml("test = \"app/src/listChanges.test.ts\""),
        &[("app/src/listChanges.test.ts", READS_THE_EXAMPLES)],
    );

    assert_eq!(problems(&root), vec![]);
}

// 共通のテストに、テストのファイルもpendingもexcludedもないシェルは、出る。
#[test]
fn a_shell_that_says_nothing_of_a_common_test_is_reported() {
    let shell = "[s2]\nlanguage = \"typescript\"\nsources = [\"app/src\"]\n\n\
                 [common]\ntests = [\"app/src\"]\n";
    let root = repository("not-named", shell, &[]);

    assert_eq!(
        problems(&root),
        vec![Problem::NotNamed {
            test: "list-changes".into()
        }]
    );
}

// テストのファイルがないと、出る。
#[test]
fn a_test_file_that_is_not_there_is_reported() {
    let root = repository(
        "no-file",
        &shell_toml("test = \"app/src/listChanges.test.ts\""),
        &[],
    );

    assert_eq!(
        problems(&root),
        vec![Problem::NoFile {
            test: "list-changes".into(),
            file: "app/src/listChanges.test.ts".into(),
        }]
    );
}

// テストのファイルが例のファイルの名前を読んでいないと、出る。
#[test]
fn a_test_file_that_does_not_read_the_examples_is_reported() {
    let root = repository(
        "does-not-read",
        &shell_toml("test = \"app/src/listChanges.test.ts\""),
        &[("app/src/listChanges.test.ts", "test('rows', () => {});\n")],
    );

    assert_eq!(
        problems(&root),
        vec![Problem::DoesNotRead {
            test: "list-changes".into(),
            file: "app/src/listChanges.test.ts".into(),
            examples: "list-changes.json".into(),
        }]
    );
}

// テストのファイルが、テストの道具の集める場所の外にあると、出る。集めるのは、testsのフォルダーの下の、言語のテストのファイルの名前のものだけである。
#[test]
fn a_test_file_that_the_test_tool_does_not_collect_is_reported() {
    for (name, file) in [
        ("outside", "app/lib/listChanges.test.ts"),
        ("not-a-test", "app/src/listChanges.ts"),
    ] {
        let root = repository(
            name,
            &shell_toml(&format!("test = \"{file}\"")),
            &[(file, READS_THE_EXAMPLES)],
        );

        assert_eq!(
            problems(&root),
            vec![Problem::NotCollected {
                test: "list-changes".into(),
                file: file.into(),
            }],
            "{file}"
        );
    }
}
