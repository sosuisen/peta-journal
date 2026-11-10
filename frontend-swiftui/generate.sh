#!/bin/sh
# Makes the two Swift packages under generated/ from the Rust crates: the types of the border
# (PetaTypes) and the core as a library (PetaCore). Run it again after a change in Rust.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
rust="$here/../frontend-rust"

rm -rf "$here/generated"
(cd "$rust" && cargo run -p peta-shell-native --features facet_typegen --bin codegen -- "$here/generated")
(cd "$rust/crates/shell-native" && boltffi pack apple --release)
