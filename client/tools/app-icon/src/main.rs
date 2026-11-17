//! `app-icon windows <svg> <ico>` writes the icon of a Windows app,
//! `app-icon ios <svg> <png>` that of an iOS app, and `app-icon macos <svg> <appiconset>` the
//! icon set of a macOS app into its folder. Each is made from the SVG.

use std::process::ExitCode;

const USAGE: &str = "usage: app-icon windows <svg> <ico>\n       app-icon ios <svg> <png>\n       app-icon macos <svg> <appiconset>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [kind, svg, icon] = args.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    if kind == "macos" {
        return done(write_set(svg, icon));
    }
    let make = match kind.as_str() {
        "windows" => app_icon::windows_icon,
        "ios" => app_icon::ios_icon,
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    done(write(make, svg, icon))
}

fn done(written: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match written {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("app-icon: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Writes the files of the icon set of macOS into the folder `set`, which is made if missing.
fn write_set(svg: &str, set: &str) -> Result<(), Box<dyn std::error::Error>> {
    let set = std::path::Path::new(set);
    std::fs::create_dir_all(set)?;
    for (name, bytes) in app_icon::macos_icon_set(&std::fs::read(svg)?)? {
        std::fs::write(set.join(name), bytes)?;
    }
    Ok(())
}

fn write(
    make: fn(&[u8]) -> Result<Vec<u8>, app_icon::Error>,
    svg: &str,
    icon: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(icon, make(&std::fs::read(svg)?)?)?;
    Ok(())
}
