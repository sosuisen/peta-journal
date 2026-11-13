# Builds the app (the Debug build), ends a running app, and starts the new one.
#
#   ./debug.ps1 -url /pages/notes
#   ./debug.ps1 -Generate -server http://localhost:8087
#
# -Generate first makes generated/ again, which is needed once and after a change in Rust.
# Everything else goes to the app. The journal server has to run already.
param(
    [switch]$Generate,
    [Parameter(ValueFromRemainingArguments)]
    [string[]]$ToApp
)
$ErrorActionPreference = 'Stop'

$here = $PSScriptRoot
if ($Generate -or -not (Test-Path "$here/generated")) {
    & "$here/generate.ps1"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

# A running app holds its files, and the build could not write them
Get-Process PetaStickies -ErrorAction SilentlyContinue | Stop-Process -Force

dotnet build "$here/PetaStickies" -c Debug -nologo -v q
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$app = Join-Path $here 'PetaStickies/bin/Debug/net11.0-windows10.0.22621.0/win-x64/PetaStickies.exe'
if ($ToApp) { Start-Process $app -ArgumentList $ToApp } else { Start-Process $app }
