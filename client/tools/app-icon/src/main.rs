//! `app-icon <svg> <ico>` writes the icon of a Windows app, made from the SVG.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [svg, ico] = args.as_slice() else {
        eprintln!("usage: app-icon <svg> <ico>");
        return ExitCode::FAILURE;
    };
    match make(svg, ico) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("app-icon: {error}");
            ExitCode::FAILURE
        }
    }
}

fn make(svg: &str, ico: &str) -> Result<(), Box<dyn std::error::Error>> {
    let icon = app_icon::windows_icon(&std::fs::read(svg)?)?;
    std::fs::write(ico, icon)?;
    Ok(())
}
