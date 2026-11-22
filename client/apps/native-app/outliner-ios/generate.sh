#!/bin/sh
# Makes the two Swift packages from the Rust crates: the types of the border (PetaTypes, in
# ../swift-shared/generated/swift) and the core with the journal inside (PetaCore, in
# generated/), as a library for a device, for the simulator and for the Mac. Run it again after
# a change in Rust.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
rust="$here/../../.."

# The types go to the package that both apps share: SwiftPM takes a package of one name from
# one place only
shared="$here/../swift-shared/generated"
rm -rf "$here/generated" "$shared"
(cd "$rust" && cargo run -p client-native-bridge --features facet_typegen --bin codegen -- swift "$shared")
# The oldest systems the app runs on, as in Package.swift. Rust builds for iOS 10 by default,
# and the C code of SQLite and zstd then asks for what only iOS 13 and later have
# (`___chkstk_darwin`): the library for a device does not link
(cd "$rust/libs/client-shell-embedded" &&
    IPHONEOS_DEPLOYMENT_TARGET=17.0 MACOSX_DEPLOYMENT_TARGET=14.0 boltffi pack apple --release)
