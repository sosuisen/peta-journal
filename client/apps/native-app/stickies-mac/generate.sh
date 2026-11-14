#!/bin/sh
# Makes the two Swift packages from the Rust crates: the types of the border (PetaTypes, in
# generated/swift) and the core as a library (PetaCore, in generated/). Run it again
# after a change in Rust.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
rust="$here/../../.."

rm -rf "$here/generated"
(cd "$rust" && cargo run -p client-native-bridge --features facet_typegen --bin codegen -- swift "$here/generated")
(cd "$rust/libs/client-shell-http" && boltffi pack apple --release)
