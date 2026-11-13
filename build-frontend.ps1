# Builds the web frontend into client/apps/web-app/outliner/dist. It works from any folder.
#
#   ./build-frontend.ps1         small files, for daily use
#   ./build-frontend.ps1 -Dev    a faster build with larger files, for development
#
# The journal server need not restart: reload the browser after the build.
param([switch]$Dev)

$ErrorActionPreference = 'Stop'

Push-Location (Join-Path $PSScriptRoot 'client/apps/web-app/outliner')
try {
    if ($Dev) { trunk build } else { trunk build --release }
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
