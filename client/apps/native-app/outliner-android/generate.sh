#!/bin/sh
# Makes generated/ from the Rust crates: the types of the border and the class that calls the
# core, as Kotlin (kotlin/), and the core as a library for each kind of device (jniLibs/).
# Run it again after a change in Rust. It needs ANDROID_NDK_HOME. It is generate.ps1 for Git Bash.
#
# The core is built for both kinds of device, as boltffi.toml of client-shell-embedded says.
# PETA_ANDROID_ABIS builds only the kinds that it names, as boltffi names them (arm64, x86_64),
# separated by spaces. The CI sets it to the kind of its emulator: each kind takes minutes.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
rust="$here/../../.."

rm -rf "$here/generated"

(cd "$rust" && cargo run -p client-native-bridge --features facet_typegen --bin codegen -- kotlin "$here/generated")

# The generator writes a build script for a project of its own. Here the types are sources of
# the app, and Gradle would compile the script as one of them
rm "$here/generated/kotlin/build.gradle.kts"

overlay=""
if [ -n "${PETA_ANDROID_ABIS:-}" ]; then
    overlay=$(mktemp)
    trap 'rm -f "$overlay"' EXIT
    list=$(printf '"%s", ' $PETA_ANDROID_ABIS)
    printf '[targets.android]\narchitectures = [%s]\n' "${list%, }" >"$overlay"
fi
(cd "$rust/libs/client-shell-embedded" && boltffi pack android --release ${overlay:+--overlay "$overlay"})
