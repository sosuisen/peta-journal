@echo off
setlocal

set PROTO_DIR=..\..\proto\src\main\proto
set OUT_DIR=Proto

if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"

where protoc >nul 2>&1
if errorlevel 1 (
    echo ERROR: protoc not found in PATH. Install Protocol Buffers compiler.
    echo   https://github.com/protocolbuffers/protobuf/releases
    exit /b 1
)

echo Generating C# protobuf classes...
protoc --proto_path="%PROTO_DIR%" --csharp_out="%OUT_DIR%" ^
    "%PROTO_DIR%\common.proto" ^
    "%PROTO_DIR%\page.proto" ^
    "%PROTO_DIR%\block.proto" ^
    "%PROTO_DIR%\search.proto" ^
    "%PROTO_DIR%\api.proto"

if errorlevel 1 (
    echo ERROR: protoc failed
    exit /b 1
)

echo Proto generation complete. Files in %OUT_DIR%\
dir /b "%OUT_DIR%\*.cs"
