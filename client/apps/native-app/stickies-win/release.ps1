# Builds the app (the Release build).
#
#   ./release.ps1
#   ./release.ps1 -Run -url /pages/notes
#   ./release.ps1 -Generate -Run -server http://localhost:8087
#
# -Generate first makes generated/ again, which is needed once and after a change in Rust.
# -Run ends a running app before the build, and starts the new one after it. Everything else
# goes to the app. The journal server has to run already then.
param(
    [switch]$Generate,
    [switch]$Run,
    [Parameter(ValueFromRemainingArguments)]
    [string[]]$ToApp
)
$ErrorActionPreference = 'Stop'

$here = $PSScriptRoot
if ($Generate -or -not (Test-Path "$here/generated")) {
    & "$here/generate.ps1"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

# A running app holds its files, and the build could not write them.
# Why not without -Run: the app of the user would end and not come back
if ($Run) { Get-Process PetaStickies -ErrorAction SilentlyContinue | Stop-Process -Force }

dotnet build "$here/PetaStickies" -c Release -nologo -v q
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$app = Join-Path $here 'PetaStickies/bin/Release/net10.0-windows10.0.22621.0/win-x64/PetaStickies.exe'
if (-not $Run) {
    Write-Host "Built $app"
    exit 0
}
if ($ToApp) { Start-Process $app -ArgumentList $ToApp } else { Start-Process $app }
