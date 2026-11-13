#!/bin/sh
# Makes the two C# projects under generated/ from the Rust crates: the types of the border
# (csharp/PetaTypes.csproj) and the core as a library with the C# that calls it (PetaCore).
# Run it again after a change in Rust. It is generate.ps1 for Git Bash.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
rust="$here/../frontend-rust"

rm -rf "$here/generated"

# BoltFFI looks for the linker that .cargo/config.toml names. It is part of the Rust toolchain,
# and cargo finds it there, but it is not on the path
linker=$(cygpath -u "$(rustc --print sysroot)")/lib/rustlib/x86_64-pc-windows-msvc/bin
PATH="$linker:$PATH"
export PATH

(cd "$rust" && cargo run -p client-native-bridge --features facet_typegen --bin codegen -- csharp "$here/generated")
(cd "$rust/crates/client-shell-http" && boltffi pack csharp --release)
