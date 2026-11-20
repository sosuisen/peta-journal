# Brings PetaJournal on this PC up to date: pulls, makes the release builds, and starts the
# journal server, then the web frontend in the browser and PetaStickies.
#
#   ./release-all.ps1            pulls the branch first
#   ./release-all.ps1 -NoPull    builds what is checked out now
#
# The order:
#   1. git pull --ff-only
#   2. PetaStickies and MemoView end. Then the journal server ends.
#   3. The release builds: the web frontend, the journal server, the Emacs shell, todo,
#      PetaStickies and MemoView.
#   4. The journal server starts in its own window, with the web frontend beside it.
#   5. When the server answers on port 8087, the browser and PetaStickies start.
#
# The apps end before the server: they save what is typed to the server when they close.
# The Emacs shell is not ended: Emacs would lose its buffers. When it runs, its build is left
# out. Close the PetaJournal buffers in Emacs and run the script again to build it.
param([switch]$NoPull)
$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$port = 8087
$apps = "$root/client/apps"
$release = "$root/journal/target/release"

function Invoke-Step([string]$name, [scriptblock]$step) {
    Write-Host "==> $name" -ForegroundColor Cyan
    $global:LASTEXITCODE = 0
    & $step
    if ($LASTEXITCODE -ne 0) { throw "$name failed with exit code $LASTEXITCODE" }
}

Add-Type -Namespace PetaRelease -Name Win32 -MemberDefinition @'
public delegate bool EnumWindowsProc(System.IntPtr hWnd, System.IntPtr lParam);
[System.Runtime.InteropServices.DllImport("user32.dll")]
public static extern bool EnumWindows(EnumWindowsProc proc, System.IntPtr lParam);
[System.Runtime.InteropServices.DllImport("user32.dll")]
public static extern uint GetWindowThreadProcessId(System.IntPtr hWnd, out uint processId);
[System.Runtime.InteropServices.DllImport("user32.dll")]
public static extern bool IsWindowVisible(System.IntPtr hWnd);
[System.Runtime.InteropServices.DllImport("user32.dll")]
public static extern bool PostMessage(System.IntPtr hWnd, uint msg, System.IntPtr wParam, System.IntPtr lParam);
'@

# Why not Stop-Process at once: a sticky closed by force loses the text that waits to be saved.
# WM_CLOSE closes each window as the taskbar does, and the stickies come back in the next run.
function Close-App([string]$name) {
    $procs = @(Get-Process $name -ErrorAction SilentlyContinue)
    if (-not $procs) { return }
    Write-Host "Closing $name"
    $ids = $procs.Id
    $close = [PetaRelease.Win32+EnumWindowsProc] {
        param($hWnd, $lParam)
        $owner = [uint32]0
        [void][PetaRelease.Win32]::GetWindowThreadProcessId($hWnd, [ref]$owner)
        if ($ids -contains $owner -and [PetaRelease.Win32]::IsWindowVisible($hWnd)) {
            [void][PetaRelease.Win32]::PostMessage($hWnd, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
        }
        $true
    }
    [void][PetaRelease.Win32]::EnumWindows($close, [IntPtr]::Zero)
    foreach ($p in $procs) {
        if (-not $p.WaitForExit(15000)) {
            Write-Warning "$name did not end in 15s: ending it by force"
            $p.Kill()
            $p.WaitForExit()
        }
    }
}

# Why not Stop-Process at once: the server writes the search index when Ctrl+C ends it.
# A helper sends Ctrl+C: this script would lose its own console if it attached to the server's.
function Stop-Server([System.Diagnostics.Process]$server) {
    $helper = @'
Add-Type -Namespace PetaRelease -Name Console -MemberDefinition @"
[DllImport("kernel32.dll")] public static extern bool FreeConsole();
[DllImport("kernel32.dll")] public static extern bool AttachConsole(uint processId);
[DllImport("kernel32.dll")] public static extern bool SetConsoleCtrlHandler(System.IntPtr handler, bool add);
[DllImport("kernel32.dll")] public static extern bool GenerateConsoleCtrlEvent(uint ctrlEvent, uint processGroupId);
"@
[void][PetaRelease.Console]::FreeConsole()
if (-not [PetaRelease.Console]::AttachConsole(SERVER_ID)) { exit 1 }
[void][PetaRelease.Console]::SetConsoleCtrlHandler([IntPtr]::Zero, $true)
[void][PetaRelease.Console]::GenerateConsoleCtrlEvent(0, 0)
'@ -replace 'SERVER_ID', $server.Id
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($helper))
    Start-Process pwsh -ArgumentList '-NoProfile', '-EncodedCommand', $encoded -WindowStyle Hidden -Wait
    if (-not $server.WaitForExit(15000)) {
        Write-Warning 'The journal server did not end in 15s: ending it by force'
        $server.Kill()
        $server.WaitForExit()
    }
}

function Test-Port([int]$number) {
    $client = [System.Net.Sockets.TcpClient]::new()
    try {
        return $client.ConnectAsync('127.0.0.1', $number).Wait(500)
    }
    catch {
        return $false
    }
    finally {
        $client.Dispose()
    }
}

if (-not $NoPull) {
    Invoke-Step 'Pull' { git -C $root pull --ff-only }
}

Close-App PetaStickies
Close-App MemoView
# The server of debug.ps1 holds the port too, so every journal-server ends
foreach ($server in @(Get-Process journal-server -ErrorAction SilentlyContinue)) {
    Write-Host "Ending the journal server ($($server.Path))"
    Stop-Server $server
}
if (Test-Port $port) { throw "Another program uses port $port. End it first." }

Invoke-Step 'Web frontend' { & "$apps/web-app/outliner/release.ps1" }
Invoke-Step 'Journal server' { & "$root/journal/journal-server/release.ps1" }
# Why a copy and not --frontend-dir: a debug `trunk build` in dist/ would change the frontend
# of the running release server too
Invoke-Step 'Copy the web frontend' {
    $frontend = "$release/frontend"
    if (Test-Path $frontend) { Remove-Item -Recurse -Force $frontend }
    Copy-Item -Recurse "$apps/web-app/outliner/dist" $frontend
}
if (Get-Process outliner-emacs -ErrorAction SilentlyContinue) {
    Write-Warning 'The Emacs shell runs: its build is left out. Close the PetaJournal buffers in Emacs first.'
}
else {
    Invoke-Step 'Emacs shell' { & "$apps/emacs/outliner/release.ps1" }
}
Invoke-Step 'todo' { & "$apps/web-app/todo/release.ps1" }
# -Generate: the pull may have changed the Rust core under the types of C#
Invoke-Step 'PetaStickies' { & "$apps/native-app/stickies-win/release.ps1" -Generate }
Invoke-Step 'MemoView' { & "$apps/webview-app/memoview-win/release.ps1" }

Write-Host '==> Starting the journal server' -ForegroundColor Cyan
$server = Start-Process "$release/journal-server.exe" -WorkingDirectory $release -PassThru
$deadline = (Get-Date).AddSeconds(60)
while (-not (Test-Port $port)) {
    if ($server.HasExited) { throw "The journal server ended with exit code $($server.ExitCode)" }
    if ((Get-Date) -gt $deadline) { throw "The journal server does not answer on port $port" }
    Start-Sleep -Milliseconds 300
}

Write-Host '==> Starting the apps' -ForegroundColor Cyan
Start-Process "http://localhost:$port"
Start-Process "$apps/native-app/stickies-win/PetaStickies/bin/Release/net11.0-windows10.0.22621.0/win-x64/PetaStickies.exe"
Write-Host "Done. The journal server runs at http://localhost:$port"
