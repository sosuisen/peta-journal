# Builds the web frontend into frontend-rust/crates/shell-leptos/dist. It works from any folder.
#
#   ./build-frontend.ps1         small files, for daily use
#   ./build-frontend.ps1 -Dev    a faster build with larger files, for development
#
# The journal server need not restart: reload the browser after the build.
param([switch]$Dev)

$ErrorActionPreference = 'Stop'

Push-Location (Join-Path $PSScriptRoot 'frontend-rust/crates/shell-leptos')
try {
    if ($Dev) { trunk build } else { trunk build --release }
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
