#!/bin/sh
# Makes the two C# projects under generated/ from the Rust crates: the types of the border
# (csharp/PetaTypes.csproj) and the core as a library with the C# that calls it (PetaCore).
# Run it again after a change in Rust. It is generate.ps1 for Git Bash, and it also runs on a
# Mac, where the core is made for the Mac: PetaBorder and its tests run there, the app does not.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
rust="$here/../../.."

rm -rf "$here/generated"

# BoltFFI looks for the linker that .cargo/config.toml names for Windows. It is part of the Rust
# toolchain, and cargo finds it there, but it is not on the path
if command -v cygpath >/dev/null; then
    linker=$(cygpath -u "$(rustc --print sysroot)")/lib/rustlib/x86_64-pc-windows-msvc/bin
    PATH="$linker:$PATH"
    export PATH
fi

(cd "$rust" && cargo run -p client-native-bridge --features facet_typegen --bin codegen -- csharp "$here/generated")
(cd "$rust/libs/client-shell-http" && boltffi pack csharp --release)
