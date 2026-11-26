//! The check that every shell has the common tests (ADR 089).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

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
}

/// What is wrong with the tests that `entries` name for the common tests `tests`, in the
/// repository at `root`.
pub fn check(
    root: &Path,
    tests: &BTreeMap<String, CommonTest>,
    entries: &BTreeMap<String, Entry>,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (name, common) in tests {
        let Some(entry) = entries.get(name) else {
            problems.push(Problem::NotNamed { test: name.clone() });
            continue;
        };
        if let Some(file) = &entry.test {
            problems.extend(check_file(root, name, common, file));
        }
    }
    problems
}

/// What is wrong with the file `file` of the test of the common test `name`.
fn check_file(root: &Path, name: &str, common: &CommonTest, file: &Path) -> Option<Problem> {
    let Ok(text) = fs::read_to_string(root.join(file)) else {
        return Some(Problem::NoFile {
            test: name.to_string(),
            file: file.to_path_buf(),
        });
    };
    if !text.contains(&common.examples) {
        return Some(Problem::DoesNotRead {
            test: name.to_string(),
            file: file.to_path_buf(),
            examples: common.examples.clone(),
        });
    }
    None
}
