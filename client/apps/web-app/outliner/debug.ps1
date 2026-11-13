# Builds the web frontend into dist/: a fast build with larger files, for development.
#
# The journal server shows it at http://localhost:8087. Start the server with
# journal/journal-server/debug.ps1. It need not restart: reload the browser after the build.
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    trunk build
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
