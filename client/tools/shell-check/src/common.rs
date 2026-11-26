//! The check that every shell has the common tests (ADR 089).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::s2::Language;

/// A common test, as `rules/core.toml` names it.
#[derive(Debug, Deserialize)]
pub struct CommonTest {
    /// The file of the examples, in `client/tools/shell-check/common/`.
    pub examples: String,
}

/// What a shell says of one common test in `rules/<shell>.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The file of the test, from the root of the repository.
    pub test: Option<PathBuf>,
    /// The issue that puts the test into the shell.
    pub pending: Option<u32>,
    /// Why the shell needs no such test.
    pub excluded: Option<String>,
}

impl Entry {
    /// How many of `test`, `pending` and `excluded` the entry says. It must say one.
    fn said(&self) -> usize {
        [
            self.test.is_some(),
            self.pending.is_some(),
            self.excluded.is_some(),
        ]
        .into_iter()
        .filter(|&said| said)
        .count()
    }
}

/// A shell, as the check of the common tests sees it.
pub struct Shell<'a> {
    pub language: Language,
    /// The folders whose tests the test tool of the shell runs, from the root of the
    /// repository.
    pub tests: &'a [PathBuf],
    /// What the shell says of each common test, by its name.
    pub entries: &'a BTreeMap<String, Entry>,
}

/// What the check found in the rules or the tests of a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The shell has no test for the common test `test`, no issue for it and no reason.
    NotNamed { test: String },
    /// The file `file` of the test is not there.
    NoFile { test: String, file: PathBuf },
    /// The file `file` of the test does not name the file of its examples, so it cannot read
    /// them.
    DoesNotRead {
        test: String,
        file: PathBuf,
        examples: String,
    },
    /// The test tool of the shell does not run the file `file`: it is outside the folders of
    /// the tests, or not named as a test.
    NotCollected { test: String, file: PathBuf },
    /// No file in the folder of the Rust file `file` declares it as a module, so cargo does
    /// not build it.
    NotDeclared { test: String, file: PathBuf },
    /// The shell waits for the common test `test` with no issue: its `pending` is 0.
    NoIssue { test: String },
    /// The shell says more than one of `test`, `pending` and `excluded` of the common test
    /// `test`.
    MoreThanOne { test: String },
    /// The shell says something of `name`, which is no common test of `rules/core.toml`.
    UnknownTest { name: String },
}

/// What is wrong with what `shell` has for the common tests `tests`, in the repository at
/// `root`.
pub fn check(root: &Path, tests: &BTreeMap<String, CommonTest>, shell: &Shell) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (name, common) in tests {
        let Some(entry) = shell.entries.get(name).filter(|entry| entry.said() > 0) else {
            problems.push(Problem::NotNamed { test: name.clone() });
            continue;
        };
        if entry.said() > 1 {
            problems.push(Problem::MoreThanOne { test: name.clone() });
            continue;
        }
        if let Some(file) = &entry.test {
            problems.extend(check_file(root, shell, name, common, file));
        }
        if entry.pending == Some(0) {
            problems.push(Problem::NoIssue { test: name.clone() });
        }
    }
    for name in shell.entries.keys() {
        if !tests.contains_key(name) {
            problems.push(Problem::UnknownTest { name: name.clone() });
        }
    }
    problems
}

/// What is wrong with the file `file` of the test of the common test `name`.
fn check_file(
    root: &Path,
    shell: &Shell,
    name: &str,
    common: &CommonTest,
    file: &Path,
) -> Option<Problem> {
    let Ok(text) = fs::read_to_string(root.join(file)) else {
        return Some(Problem::NoFile {
            test: name.to_string(),
            file: file.to_path_buf(),
        });
    };
    if !is_collected(shell, file) {
        return Some(Problem::NotCollected {
            test: name.to_string(),
            file: file.to_path_buf(),
        });
    }
    if shell.language == Language::Rust && !is_declared(root, file) {
        return Some(Problem::NotDeclared {
            test: name.to_string(),
            file: file.to_path_buf(),
        });
    }
    if !text.contains(&common.examples) {
        return Some(Problem::DoesNotRead {
            test: name.to_string(),
            file: file.to_path_buf(),
            examples: common.examples.clone(),
        });
    }
    None
}

/// True when a Rust file in the folder of `file` declares it as a module, as `mod.rs`, `lib.rs`
/// or `main.rs` does with `mod list_changes;`. A file that declares a module of another folder
/// with `#[path]` is not seen.
fn is_declared(root: &Path, file: &Path) -> bool {
    let (Some(stem), Some(folder)) = (file.file_stem().and_then(|s| s.to_str()), file.parent())
    else {
        return false;
    };
    let declaration = format!("mod {stem};");
    let Ok(entries) = fs::read_dir(root.join(folder)) else {
        return false;
    };
    entries.flatten().map(|entry| entry.path()).any(|path| {
        path != root.join(file)
            && path.extension().is_some_and(|ext| ext == "rs")
            && fs::read_to_string(&path)
                .is_ok_and(|text| text.lines().any(|line| line.trim() == declaration))
    })
}

/// True when the test tool of `shell` runs the file `file`.
fn is_collected(shell: &Shell, file: &Path) -> bool {
    let named_as_a_test = file
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| shell.language.names_a_test(name));
    named_as_a_test && shell.tests.iter().any(|folder| file.starts_with(folder))
}
