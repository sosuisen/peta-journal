//! `shell-check s2 <shell> [--root <repository>] [--rules <folder>]`
//!
//! Prints what the check found. The exit code is 0 when it found nothing, 1 when it found
//! something, and 2 when it could not run.

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: shell-check s2 <shell> [--root <repository>] [--rules <folder>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

/// Runs the check that `args` name. True when it found nothing.
fn run(args: &[String]) -> Result<bool, String> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut root = manifest.join("../../..");
    let mut rules = manifest.join("rules");
    let (check, shell) = match args {
        [check, shell, ..] => (check.as_str(), shell.as_str()),
        _ => return Err(USAGE.to_string()),
    };
    let mut options = args[2..].iter();
    while let Some(option) = options.next() {
        let value = options.next().ok_or(USAGE)?;
        match option.as_str() {
            "--root" => root = PathBuf::from(value),
            "--rules" => rules = PathBuf::from(value),
            _ => return Err(USAGE.to_string()),
        }
    }
    if check != "s2" {
        return Err(USAGE.to_string());
    }
    let report = shell_check::run_s2(&root, &rules, shell)?;
    for name in &report.missing {
        println!("missing: {name}");
    }
    for name in &report.unknown_exclusions {
        println!("unknown exclusion: {name}");
    }
    Ok(report.is_clean())
}
