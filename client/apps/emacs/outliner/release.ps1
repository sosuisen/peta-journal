# Builds the program (the release build) into client/target/release. The mode of Emacs finds
# it there.
$ErrorActionPreference = 'Stop'

Push-Location $PSScriptRoot
try {
    cargo build --release -p client-shell-emacs
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
