# Stickies — Build & Run

## Prerequisites

- Visual Studio 2026 (v18) with **.NET desktop development** and **Windows App SDK** workloads
- .NET 10 SDK
- `protoc` (Protocol Buffers compiler) — or run `cd proto && mvn compile` first to get a cached copy

## Generate Protobuf C# Classes

From Git Bash:

```bash
cd ext-desktop-app/stickies
bash generate-proto.sh
```

This generates C# classes in `Proto/` from the proto files in `proto/src/main/proto/`.

## Build

**Important:** Use `msbuild` from Developer PowerShell for VS 18 (not `dotnet build`), because WinUI3's PRI generation requires VS build tooling.

Open **Developer PowerShell for VS 18**, then:

```powershell
cd ext-desktop-app\stickies
msbuild Stickies.csproj -restore -p:Configuration=Debug -p:Platform=x64 -verbosity:minimal
```

Output: `bin\x64\Debug\net10.0-windows10.0.22621.0\Stickies.exe`

## Run

1. Start `journal-server` first (default WebSocket at `ws://localhost:8080/ws`)
2. Run `bin\x64\Debug\net10.0-windows10.0.22621.0\Stickies.exe`

## Settings

Window state is persisted at `~/.peta-stickies` (JSON). Delete this file to reset all windows.
