#!/bin/sh
# Makes the two Swift packages under generated/ from the Rust crates: the types of the border
# (PetaTypes) and the core with the journal inside (PetaCore), as a library for a device, for
# the simulator and for the Mac. Run it again after a change in Rust.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
rust="$here/../frontend-rust"

rm -rf "$here/generated"
(cd "$rust" && cargo run -p peta-shell-native --features facet_typegen --bin codegen -- swift "$here/generated")
(cd "$rust/crates/shell-android" && boltffi pack apple --release)
