//! The check that every shell has the common tests (ADR 089).

use std::collections::BTreeMap;
use std::path::PathBuf;

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
}

/// What is wrong with the tests that `entries` name for the common tests `tests`.
pub fn check(
    tests: &BTreeMap<String, CommonTest>,
    entries: &BTreeMap<String, Entry>,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    for name in tests.keys() {
        if !entries.contains_key(name) {
            problems.push(Problem::NotNamed { test: name.clone() });
        }
    }
    problems
}
