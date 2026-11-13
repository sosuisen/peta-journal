# Builds Stickies (the Release build). stickies/INSTALL.md lists the tools.
#
# A running Stickies of this build holds its files: end it first.
$ErrorActionPreference = 'Stop'

# Why not `dotnet build`: the PRI files of WinUI need the build tools of Visual Studio
$msbuild = (Get-Command msbuild -ErrorAction SilentlyContinue).Source
if (-not $msbuild) {
    $vswhere = "${env:ProgramFiles(x86)}/Microsoft Visual Studio/Installer/vswhere.exe"
    $msbuild = & $vswhere -latest -prerelease -requires Microsoft.Component.MSBuild -find 'MSBuild/**/Bin/MSBuild.exe' |
        Select-Object -First 1
}
if (-not $msbuild) { throw 'Cannot find msbuild: install Visual Studio with the .NET desktop workload' }

$platform = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'ARM64' } else { 'x64' }

& $msbuild "$PSScriptRoot/stickies/Stickies.csproj" -restore -p:Configuration=Release "-p:Platform=$platform" -verbosity:minimal -nologo
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Built $(Join-Path $PSScriptRoot "stickies/bin/$platform/Release/net10.0-windows10.0.22621.0/Stickies.exe")"
