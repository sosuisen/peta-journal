# Makes generated/ from the Rust crates: the types of the border and the class that calls the
# core, as Kotlin (kotlin/), and the core as a library for each kind of device (jniLibs/).
# Run it again after a change in Rust. It needs ANDROID_NDK_HOME.
$ErrorActionPreference = 'Stop'

$here = $PSScriptRoot
$rust = Join-Path $here '../frontend-rust'

if (Test-Path "$here/generated") { Remove-Item "$here/generated" -Recurse -Force }

Push-Location $rust
try {
    cargo run -p client-native-bridge --features facet_typegen --bin codegen -- kotlin "$here/generated"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally { Pop-Location }

# The generator writes a build script for a project of its own. Here the types are sources of
# the app, and Gradle would compile the script as one of them
Remove-Item "$here/generated/kotlin/build.gradle.kts"

Push-Location (Join-Path $rust 'crates/client-shell-embedded')
try {
    boltffi pack android --release
    exit $LASTEXITCODE
}
finally { Pop-Location }
