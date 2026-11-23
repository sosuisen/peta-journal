# Builds the journal server (the debug build) and starts it on port 8087. What is given goes
# to the server.
#
#   ./debug.ps1
#   ./debug.ps1 --help
#
# It serves the web frontend from client/apps/web-app/outliner-react/dist (journal/.cargo/config.toml).
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    cargo run -p journal-server -- @args
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
