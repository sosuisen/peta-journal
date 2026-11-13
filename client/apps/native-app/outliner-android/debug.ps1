# Builds the app (the debug build), installs it on the device or the emulator that adb sees,
# and starts it.
#
#   ./debug.ps1
#   ./debug.ps1 -Pabi=arm64-v8a "-PdropboxKey=<App key>" "-PdropboxFolder=/PetaJournal"
#   ./debug.ps1 -Generate ...     makes generated/ again first, after a change in Rust
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
    ./gradlew :app:installDebug @ToGradle
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}

& "$env:ANDROID_HOME/platform-tools/adb" shell am start -n net.sosuisen.petajournal/.MainActivity
exit $LASTEXITCODE
