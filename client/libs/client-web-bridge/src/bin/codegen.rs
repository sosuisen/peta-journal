//! Writes the TypeScript types of the border into a folder (ADR 080):
//!
//!   cargo run -p client-web-bridge --bin codegen -- <folder>

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let Some(out) = std::env::args().nth(1) else {
        eprintln!("usage: codegen <folder>");
        std::process::exit(2);
    };
    if let Err(error) = client_web_bridge::codegen::write_typescript_types(out.as_ref()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

// The types are written on the machine that builds, not in the browser
#[cfg(target_arch = "wasm32")]
fn main() {}
