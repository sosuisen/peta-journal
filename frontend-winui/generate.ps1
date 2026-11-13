# Makes the two C# projects under generated/ from the Rust crates: the types of the border
# (csharp/PetaTypes.csproj) and the core as a library with the C# that calls it (PetaCore).
# Run it again after a change in Rust.
$ErrorActionPreference = 'Stop'

$here = $PSScriptRoot
$rust = Join-Path $here '../frontend-rust'

if (Test-Path "$here/generated") { Remove-Item "$here/generated" -Recurse -Force }

# BoltFFI looks for the linker that .cargo/config.toml names. It is part of the Rust toolchain,
# and cargo finds it there, but it is not on the path
$linker = Join-Path (rustc --print sysroot) 'lib/rustlib/x86_64-pc-windows-msvc/bin'
$env:PATH = "$linker;$env:PATH"

Push-Location $rust
try {
    cargo run -p client-native-bridge --features facet_typegen --bin codegen -- csharp "$here/generated"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally { Pop-Location }

Push-Location (Join-Path $rust 'crates/client-shell-http')
try {
    boltffi pack csharp --release
    exit $LASTEXITCODE
}
finally { Pop-Location }
