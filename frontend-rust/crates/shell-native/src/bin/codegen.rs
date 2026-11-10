//! Writes the Swift types of the border into the folder it is given.

use peta_shell_native::codegen::write_swift_types;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).expect("the folder to write into"));
    write_swift_types(&out).expect("the types are written");
}
