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

/// The rules of a Rust shell whose tests are modules under `app/src/tests`.
const RUST_SHELL: &str = "[s2]\nlanguage = \"rust\"\nsources = [\"app/src\"]\n\n\
                          [common]\ntests = [\"app/src/tests\"]\n\n[common.list-changes]\n\
                          test = \"app/src/tests/list_changes.rs\"\n";

const RUST_TEST: &str = "const CASES: &str = include_str!(\"../../common/list-changes.json\");\n";

// Rustのテストのファイルは、同じフォルダーのmodで読まれていないと、出る。cargoは、modで読まれたファイルしかビルドしない。
#[test]
fn a_rust_test_file_that_no_mod_declares_is_reported() {
    let undeclared = repository(
        "rust-undeclared",
        RUST_SHELL,
        &[
            ("app/src/tests/list_changes.rs", RUST_TEST),
            ("app/src/tests/mod.rs", "mod protocol;\n"),
        ],
    );
    let declared = repository(
        "rust-declared",
        RUST_SHELL,
        &[
            ("app/src/tests/list_changes.rs", RUST_TEST),
            ("app/src/tests/mod.rs", "mod list_changes;\nmod protocol;\n"),
        ],
    );

    assert_eq!(
        problems(&undeclared),
        vec![Problem::NotDeclared {
            test: "list-changes".into(),
            file: "app/src/tests/list_changes.rs".into(),
        }]
    );
    assert_eq!(problems(&declared), vec![]);
}

/// The rules of the Emacs shell: its Rust tests are modules under `app/src/tests`, and ERT runs
/// the tests of its Lisp under `app/lisp/test`.
const EMACS_SHELL: &str = "[s2]\nlanguage = \"rust\"\nsources = [\"app/src\"]\n\n\
                           [common]\ntests = [\"app/src/tests\", \"app/lisp/test\"]\n\n\
                           [common.list-changes]\ntest = \"app/lisp/test/app-test.el\"\n";

// Rustのシェルは、Lispのテストのファイルも共通のテストにできる。EmacsのシェルはRustとLispでできていて、Lispのテストは、modで読まれなくてもERTが回す。
#[test]
fn a_rust_shell_may_have_a_common_test_in_lisp_that_no_mod_declares() {
    let root = repository(
        "rust-lisp",
        EMACS_SHELL,
        &[
            (
                "app/lisp/test/app-test.el",
                "(insert-file-contents \"common/list-changes.json\")\n",
            ),
            ("app/src/tests/mod.rs", "mod protocol;\n"),
        ],
    );

    assert_eq!(problems(&root), vec![]);
}

// pendingにissueの番号があれば、テストのファイルがなくても出ない。番号が0だと、出る。
#[test]
fn a_pending_test_needs_the_number_of_an_issue() {
    let pending = repository("pending", &shell_toml("pending = 1208"), &[]);
    let no_issue = repository("no-issue", &shell_toml("pending = 0"), &[]);

    assert_eq!(problems(&pending), vec![]);
    assert_eq!(
        problems(&no_issue),
        vec![Problem::NoIssue {
            test: "list-changes".into()
        }]
    );
}

// test、pending、excludedは、ちょうど1つだけ書く。2つ以上を書くと出て、何も書かない節は、書かないのと同じに出る。
#[test]
fn a_common_test_says_one_of_test_pending_and_excluded() {
    let two = repository(
        "two",
        &shell_toml("pending = 1208\nexcluded = \"no rows\""),
        &[],
    );
    let none = repository("none", &shell_toml(""), &[]);
    let excluded = repository("excluded", &shell_toml("excluded = \"no rows\""), &[]);

    assert_eq!(
        problems(&two),
        vec![Problem::MoreThanOne {
            test: "list-changes".into()
        }]
    );
    assert_eq!(
        problems(&none),
        vec![Problem::NotNamed {
            test: "list-changes".into()
        }]
    );
    assert_eq!(problems(&excluded), vec![]);
}

// 共通のテストの一覧にない名前を書くと、出る。名前を書き誤ったまま、そのテストを備えたつもりにならないためである。
#[test]
fn a_name_that_is_no_common_test_is_reported() {
    let shell = format!(
        "{}\n[common.list-change]\npending = 1208\n",
        shell_toml("pending = 1208")
    );
    let root = repository("unknown", &shell, &[]);

    assert_eq!(
        problems(&root),
        vec![Problem::UnknownTest {
            name: "list-change".into()
        }]
    );
}
