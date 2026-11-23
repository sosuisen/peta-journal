//! Writes the TypeScript types of the border into a folder (ADR 080):
//!
//!   cargo run -p client-web-bridge --bin codegen -- <folder>

// The program ends by returning from `main`, not with `exit`: a program that exits writes no
// coverage
#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    use std::process::ExitCode;
    let Some(out) = std::env::args().nth(1) else {
        eprintln!("usage: codegen <folder>");
        return ExitCode::from(2);
    };
    match client_web_bridge::codegen::write_typescript_types(out.as_ref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

// The types are written on the machine that builds, not in the browser
#[cfg(target_arch = "wasm32")]
fn main() {}
