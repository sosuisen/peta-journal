#!/bin/sh
# Makes the two Swift packages from the Rust crates: the types of the border (PetaTypes, in
# ../swift-shared/generated/swift) and the core as a library (PetaCore, in generated/). Run it
# again after a change in Rust.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
rust="$here/../../.."

# The types go to the package that both apps share: SwiftPM takes a package of one name from
# one place only
shared="$here/../swift-shared/generated"
rm -rf "$here/generated" "$shared"
(cd "$rust" && cargo run -p client-native-bridge --features facet_typegen --bin codegen -- swift "$shared")
(cd "$rust/libs/client-shell-http" && boltffi pack apple --release)
