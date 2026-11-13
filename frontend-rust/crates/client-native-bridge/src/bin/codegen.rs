//! Writes the types of the border into the folder it is given, for the language it is given:
//! `codegen swift <folder>`, `codegen csharp <folder>` or `codegen kotlin <folder>`.

use client_native_bridge::codegen::{write_csharp_types, write_kotlin_types, write_swift_types};
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let language = args.next().expect("the language: swift, csharp or kotlin");
    let out = PathBuf::from(args.next().expect("the folder to write into"));
    match language.as_str() {
        "swift" => write_swift_types(&out),
        "csharp" => write_csharp_types(&out),
        "kotlin" => write_kotlin_types(&out),
        other => panic!("no types for {other:?}: swift, csharp or kotlin"),
    }
    .expect("the types are written");
}
