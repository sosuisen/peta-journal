# Builds the app (the release build) as a package that is not signed:
# app/build/outputs/apk/release/app-release-unsigned.apk. It installs nothing: a device takes
# only a signed package.
#
#   ./release.ps1
#   ./release.ps1 "-PdropboxKey=<App key>" "-PdropboxFolder=/PetaJournal"
#   ./release.ps1 -Generate ...     makes generated/ again first, after a change in Rust
#
# Everything else goes to Gradle. README.md says what the properties mean.
param(
    [switch]$Generate,
    [Parameter(ValueFromRemainingArguments)]
    [string[]]$ToGradle
)
$ErrorActionPreference = 'Stop'

$here = $PSScriptRoot
if ($Generate -or -not (Test-Path "$here/generated")) {
    & "$here/generate.ps1"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Push-Location $here
try {
    ./gradlew :app:assembleRelease @ToGradle
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
