//! Writes the TypeScript types of the border into a folder (ADR 080):
//!
//!   cargo run -p client-web-bridge --bin web-codegen -- <folder>

// Not `codegen`: `client-native-bridge` has a program of that name. In one build of both crates,
// as `cargo cov` makes, the two programs are written to the same file, and the tests of one
// crate run the program of the other.

// The program ends by returning from `main`, not with `exit`: a program that exits writes no
// coverage
#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    use std::process::ExitCode;
    let Some(out) = std::env::args().nth(1) else {
        eprintln!("usage: web-codegen <folder>");
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
