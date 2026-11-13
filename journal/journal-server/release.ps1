# Builds the journal server (the release build) into journal/target/release.
#
# The built program does not see journal/.cargo/config.toml. To serve the web frontend, give
# it --frontend-dir, or copy dist/ of the web frontend to target/release/frontend/.
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    cargo build --release -p journal-server
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
