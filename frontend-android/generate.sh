#!/bin/sh
# Makes generated/ from the Rust crates: the types of the border and the class that calls the
# core, as Kotlin (kotlin/), and the core as a library for each kind of device (jniLibs/).
# Run it again after a change in Rust. It needs ANDROID_NDK_HOME. It is generate.ps1 for Git Bash.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
rust="$here/../frontend-rust"

rm -rf "$here/generated"

(cd "$rust" && cargo run -p client-native-bridge --features facet_typegen --bin codegen -- kotlin "$here/generated")

# The generator writes a build script for a project of its own. Here the types are sources of
# the app, and Gradle would compile the script as one of them
rm "$here/generated/kotlin/build.gradle.kts"

(cd "$rust/crates/shell-android" && boltffi pack android --release)
