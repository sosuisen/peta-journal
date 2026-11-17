//! `app-icon windows <svg> <ico>` writes the icon of a Windows app, and
//! `app-icon ios <svg> <png>` that of an iOS app. Both are made from the SVG.

use std::process::ExitCode;

const USAGE: &str = "usage: app-icon windows <svg> <ico>\n       app-icon ios <svg> <png>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [kind, svg, icon] = args.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let make = match kind.as_str() {
        "windows" => app_icon::windows_icon,
        "ios" => app_icon::ios_icon,
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match write(make, svg, icon) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("app-icon: {error}");
            ExitCode::FAILURE
        }
    }
}

fn write(
    make: fn(&[u8]) -> Result<Vec<u8>, app_icon::Error>,
    svg: &str,
    icon: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(icon, make(&std::fs::read(svg)?)?)?;
    Ok(())
}
