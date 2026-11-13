# Builds MemoView (the Debug build), ends a running app, and starts the new one. What is given
# goes to the app.
#
# The journal server has to run already. memoview/INSTALL.md lists the tools.
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

# A running app holds its files, and the build could not write them
Get-Process MemoView -ErrorAction SilentlyContinue | Stop-Process -Force

& $msbuild "$PSScriptRoot/memoview/MemoView.csproj" -restore -p:Configuration=Debug "-p:Platform=$platform" -verbosity:minimal -nologo
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$app = Join-Path $PSScriptRoot "memoview/bin/$platform/Debug/net10.0-windows10.0.22621.0/MemoView.exe"
if ($args) { Start-Process $app -ArgumentList $args } else { Start-Process $app }
