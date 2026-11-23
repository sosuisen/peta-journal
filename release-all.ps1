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
#   4. todo is copied to apps/todo/ of the config folder, where the journal server serves it
#      at /apps/todo/ (ADR 073).
#   5. The journal server starts in its own window, with the web frontend beside it.
#   6. When the server answers on port 8087, the browser opens the web frontend and todo, and
#      PetaStickies starts.
#
# The config folder is ~/.peta-journal, or PETAJOURNAL_CONFIG_DIR when it is set, as for the
# journal server. The home is HOME, or USERPROFILE without it, as the server takes it too:
# Git Bash sets HOME, and release-all.sh runs this script from there.
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
$configDir = $env:PETAJOURNAL_CONFIG_DIR
if (-not $configDir) {
    $userHome = $env:HOME ? $env:HOME : $env:USERPROFILE
    $configDir = Join-Path $userHome '.peta-journal'
}

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

function Wait-Port([int]$number, [System.Diagnostics.Process]$server, [string]$name) {
    $deadline = (Get-Date).AddSeconds(60)
    while (-not (Test-Port $number)) {
        if ($server.HasExited) { throw "The $name ended with exit code $($server.ExitCode)" }
        if ((Get-Date) -gt $deadline) { throw "The $name does not answer on port $number" }
        Start-Sleep -Milliseconds 300
    }
}

if (-not $NoPull) {
    $before = (Get-FileHash $PSCommandPath).Hash
    Invoke-Step 'Pull' { git -C $root pull --ff-only }
    # PowerShell read this script when it started: what the pull changed in it would not run.
    # Why another process: the types of Add-Type of this version could differ from the new one
    if ((Get-FileHash $PSCommandPath).Hash -ne $before) {
        Write-Host 'The pull changed this script: starting the new version'
        pwsh -NoProfile -File $PSCommandPath -NoPull
        exit $LASTEXITCODE
    }
}

Close-App PetaStickies
Close-App MemoView
# The server of debug.ps1 holds the port too, so every journal-server ends
foreach ($server in @(Get-Process journal-server -ErrorAction SilentlyContinue)) {
    Write-Host "Ending the journal server ($($server.Path))"
    Stop-Server $server
}
if (Test-Port $port) { throw "Another program uses port $port. End it first." }

# The React shell (ADR 079)
Invoke-Step 'Web frontend' { & "$apps/web-app/outliner-react/release.ps1" }
Invoke-Step 'Journal server' { & "$root/journal/journal-server/release.ps1" }
# Why a copy and not --frontend-dir: a new build in dist/ during development would change the
# frontend of the running release server too
Invoke-Step 'Copy the web frontend' {
    $frontend = "$release/frontend"
    if (Test-Path $frontend) { Remove-Item -Recurse -Force $frontend }
    Copy-Item -Recurse "$apps/web-app/outliner-react/dist" $frontend
}
if (Get-Process outliner-emacs -ErrorAction SilentlyContinue) {
    Write-Warning 'The Emacs shell runs: its build is left out. Close the PetaJournal buffers in Emacs first.'
}
else {
    Invoke-Step 'Emacs shell' { & "$apps/emacs/outliner/release.ps1" }
}
Invoke-Step 'todo' { & "$apps/web-app/todo/release.ps1" }
# The files of an older build go first: their hashed names would stay beside the new ones
Invoke-Step 'Copy todo' {
    $todo = Join-Path $configDir 'apps/todo'
    if (Test-Path $todo) { Remove-Item -Recurse -Force $todo }
    New-Item -ItemType Directory -Force (Split-Path $todo) | Out-Null
    Copy-Item -Recurse "$apps/web-app/todo/dist" $todo
}
# -Generate: the pull may have changed the Rust core under the types of C#
Invoke-Step 'PetaStickies' { & "$apps/native-app/stickies-win/release.ps1" -Generate }
Invoke-Step 'MemoView' { & "$apps/webview-app/memoview-win/release.ps1" }

Write-Host '==> Starting the journal server' -ForegroundColor Cyan
$server = Start-Process "$release/journal-server.exe" -WorkingDirectory $release -PassThru
Wait-Port $port $server 'journal server'

Write-Host '==> Starting the apps' -ForegroundColor Cyan
Start-Process "http://localhost:$port"
Start-Process "http://localhost:$port/apps/todo/"
Start-Process "$apps/native-app/stickies-win/PetaStickies/bin/Release/net11.0-windows10.0.22621.0/win-x64/PetaStickies.exe"

Write-Host "Done. The journal server runs at http://localhost:$port, todo at http://localhost:$port/apps/todo/"
