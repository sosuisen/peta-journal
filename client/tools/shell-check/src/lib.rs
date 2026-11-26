//! Checks that the shells go through the core (ADR 064), and that every shell has the common
//! tests (ADR 089). They read the sources as text.
//!
//! The rules of each shell are TOML files in `rules/`. `rules/core.toml` holds what is the same
//! for every shell.

pub mod common;
pub mod events;
pub mod s2;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use s2::{Language, Report};

/// Where `client-core` defines `Event`, from the root of the repository.
const APP_RS: &str = "client/libs/client-core/src/app.rs";

/// The rules that hold for every shell: `rules/core.toml`.
#[derive(Debug, Deserialize)]
struct CoreRules {
    s2: Exclusions,
    /// The common tests that every shell runs (ADR 089), by name.
    #[serde(default)]
    common: BTreeMap<String, common::CommonTest>,
}

#[derive(Debug, Deserialize)]
struct Exclusions {
    /// The kinds of `Event` left out, each with its reason.
    excluded: BTreeMap<String, String>,
}

/// The rules of one shell: `rules/<shell>.toml`.
#[derive(Debug, Deserialize)]
struct ShellRules {
    s2: ShellS2,
    #[serde(default)]
    common: ShellCommon,
}

/// What a shell says of the common tests (ADR 089).
#[derive(Debug, Default, Deserialize)]
struct ShellCommon {
    /// The folders whose tests the test tool of the shell runs, from the root of the
    /// repository.
    #[serde(default)]
    tests: Vec<PathBuf>,
    /// What the shell says of each common test, by its name.
    #[serde(flatten)]
    entries: BTreeMap<String, common::Entry>,
}

#[derive(Debug, Deserialize)]
struct ShellS2 {
    language: Language,
    /// The folders of the sources, from the root of the repository.
    sources: Vec<PathBuf>,
    /// The folders under `sources` that are not read, such as generated code.
    #[serde(default)]
    skipped: Vec<PathBuf>,
    #[serde(default)]
    excluded: BTreeMap<String, String>,
}

/// Runs S2 for `shell` on the repository at `root`, with the rules in `rules`.
pub fn run_s2(root: &Path, rules: &Path, shell: &str) -> Result<Report, String> {
    let core: CoreRules = read_toml(&rules.join("core.toml"))?;
    let rules: ShellRules = read_toml(&rules.join(format!("{shell}.toml")))?;
    let app = read(&root.join(APP_RS))?;
    let variants =
        events::event_variants(&app).ok_or_else(|| format!("{APP_RS} has no `pub enum Event`"))?;
    let skipped: Vec<PathBuf> = rules.s2.skipped.iter().map(|f| root.join(f)).collect();
    let mut sources = String::new();
    for folder in &rules.s2.sources {
        read_sources(
            &root.join(folder),
            rules.s2.language,
            &skipped,
            &mut sources,
        )?;
    }
    Ok(s2::check(
        &variants,
        &sources,
        rules.s2.language,
        &[&core.s2.excluded, &rules.s2.excluded],
    ))
}

/// Checks that `shell` has every common test of the repository at `root`, with the rules in
/// `rules` (ADR 089).
pub fn run_common(root: &Path, rules: &Path, shell: &str) -> Result<Vec<common::Problem>, String> {
    let core: CoreRules = read_toml(&rules.join("core.toml"))?;
    let rules: ShellRules = read_toml(&rules.join(format!("{shell}.toml")))?;
    let shell = common::Shell {
        language: rules.s2.language,
        tests: &rules.common.tests,
        entries: &rules.common.entries,
    };
    Ok(common::check(root, &core.common, &shell))
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn read_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    toml::from_str(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

/// Adds the text of every source of `language` under `folder` to `into`, but not under the
/// folders in `skipped`. A folder that is not there is an error: a rule that names a wrong
/// folder would let every kind look missing.
fn read_sources(
    folder: &Path,
    language: Language,
    skipped: &[PathBuf],
    into: &mut String,
) -> Result<(), String> {
    let entries = fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            if !skipped.contains(&path) {
                read_sources(&path, language, skipped, into)?;
            }
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| language.reads(name))
        {
            into.push_str(&read(&path)?);
            into.push('\n');
        }
    }
    Ok(())
}
