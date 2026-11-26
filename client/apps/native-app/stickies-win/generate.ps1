# Makes the two C# projects under generated/ from the Rust crates: the types of the border
# (csharp/PetaTypes.csproj) and the core as a library with the C# that calls it (PetaCore).
# Run it again after a change in Rust.
$ErrorActionPreference = 'Stop'

$here = $PSScriptRoot
$rust = Join-Path $here '../../..'

if (Test-Path "$here/generated") { Remove-Item "$here/generated" -Recurse -Force }

# The app is built for win-x64 only, and BoltFFI packs only for x86_64. On an ARM64 PC the
# default Rust is aarch64, so the x64 Rust of the version in rust-toolchain.toml builds here,
# under emulation. A RUSTUP_TOOLCHAIN that is set already is kept.
# Why it is put back at the end: release.ps1 runs this script in its own session, and the
# builds after it (and the shell of the user) would turn to x64 Rust too
$oldToolchain = $env:RUSTUP_TOOLCHAIN
$oldPath = $env:PATH
$arm64 = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64'
if ($arm64 -and -not $oldToolchain) {
    $toml = Get-Content (Join-Path $here '../../../../rust-toolchain.toml') -Raw
    if ($toml -notmatch '(?m)^channel = "([^"]+)"') { throw 'rust-toolchain.toml has no channel' }
    $env:RUSTUP_TOOLCHAIN = "$($Matches[1])-x86_64-pc-windows-msvc"
    Write-Host "ARM64: the Rust of $env:RUSTUP_TOOLCHAIN builds the core"
}

try {
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

    Push-Location (Join-Path $rust 'libs/client-shell-http')
    try {
        boltffi pack csharp --release
        exit $LASTEXITCODE
    }
    finally { Pop-Location }
}
finally {
    $env:RUSTUP_TOOLCHAIN = $oldToolchain
    $env:PATH = $oldPath
}
