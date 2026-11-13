# Builds the web frontend into dist/: small files, for daily use.
#
# The journal server need not restart: reload the browser after the build.
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    trunk build --release
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
